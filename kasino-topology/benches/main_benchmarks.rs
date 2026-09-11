use std::{
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering},
    thread::{self, available_parallelism},
    time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use crossbeam_queue::ArrayQueue as RawArrayQueue;
use kasino::{
    Collection,
    InlineBandit,
    Signature,
    WithCapacity,
    components::{PopSignature, TryPushSignature},
    strategy::{DCBO, DRA, RandomAccess, RoundRobin},
};
use kasino_topology::{CoreID, CorePinned, RuntimeData};
use rand::rngs::SmallRng;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CoreAffinity2;

impl RuntimeData for CoreAffinity2 {
    type Error = ();

    fn get_affinity() -> Result<CoreID, Self::Error> {
        core_affinity2::get_affinity()
            .unwrap()
            .ok_or(())
            .map(|id| CoreID(id.0))
    }

    fn set_affinity(id: CoreID) -> Result<(), Self::Error> {
        core_affinity2::CoreId(id.0).set_affinity().map_err(|_| ())
    }

    fn available_cores() -> impl Iterator<Item = CoreID> {
        core_affinity2::get_core_ids()
            .unwrap()
            .into_iter()
            .map(|id| CoreID(id.0))
    }

    fn available_parallelism() -> Result<core::num::NonZero<usize>, Self::Error> {
        available_parallelism().map_err(|_| ())
    }
}

struct Backoff();
impl Backoff {
    fn new() -> Self {
        Self()
    }

    fn spin(&mut self) {
        thread::yield_now();
    }
}

struct QAdapter<T, const N: usize>(RawArrayQueue<T>);

impl<T, const N: usize> Collection for QAdapter<T, N> {
    type OfferSignature = TryPushSignature<T>;
    type PollSignature = PopSignature<T>;

    fn offer<'a, 'b>(
        &'b self,
        item: <Self::OfferSignature as Signature>::Input<'a>,
    ) -> Result<
        <Self::OfferSignature as Signature>::Output<'a, 'b>,
        <Self::OfferSignature as Signature>::Error<'a, 'b>,
    > {
        self.0.push(item)
    }

    fn poll<'a, 'b>(
        &'b self,
        _input: <Self::PollSignature as Signature>::Input<'a>,
    ) -> Result<
        <Self::PollSignature as Signature>::Output<'a, 'b>,
        <Self::PollSignature as Signature>::Error<'a, 'b>,
    > {
        self.0.pop().ok_or(())
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl<T, const N: usize> WithCapacity<N> for QAdapter<T, N> {
    fn with_capacity() -> Self {
        Self(RawArrayQueue::new(N))
    }
}

fn retry_push<T>(q: &RawArrayQueue<T>, mut item: T) {
    let mut b = Backoff::new();
    loop {
        match q.push(item) {
            Ok(()) => return,
            Err(back) => {
                item = back;
                b.spin();
            }
        }
    }
}

fn retry_pop<T>(q: &RawArrayQueue<T>) -> T {
    let mut b = Backoff::new();
    loop {
        if let Some(x) = q.pop() {
            return x;
        }
        b.spin();
    }
}

const SUB_QUEUE_COUNT: usize = 32;
const MT_SUB_CAP: usize = 128;
const MT_COUNT: usize = 20_000;

macro_rules! bench_kasino_mpmc {
    ($group:expr, $name:literal, $Sched:ty, [$($n:literal),+ $(,)?]) => {
        $(
            $group.throughput(Throughput::Elements(($n * MT_COUNT) as u64));
            $group.bench_function(BenchmarkId::new($name, $n), |b| {
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        let bandit: InlineBandit<QAdapter<u64, MT_SUB_CAP>, $Sched, SUB_QUEUE_COUNT, MT_SUB_CAP> =
                            InlineBandit::new();
                            let  root = bandit.buy_in();
                        let pollped_total = AtomicUsize::new(0);
                        let start = Instant::now();
                        std::thread::scope(|scope| {
                            for _ in 0..$n {
                                let mut arm = root.fork();
                                scope.spawn(move || {
                                    for i in 0..MT_COUNT {
                                        let mut b = Backoff::new();
                                        while arm.offer(i as u64).is_err() {
                                            b.spin();
                                        }
                                    }
                                });
                            }
                            for _ in 0..$n {
                                let mut arm = root.fork();
                                let pollped_total = &pollped_total;
                                scope.spawn(move || {
                                    let mut pollped = 0usize;
                                    while pollped < MT_COUNT {
                                        let mut b = Backoff::new();
                                        if arm.poll(()).is_ok() {
                                            pollped += 1;
                                        } else {
                                            b.spin();
                                        }
                                    }
                                    pollped_total.fetch_add(pollped, Ordering::Relaxed);
                                });
                            }
                        });
                        total += start.elapsed();
                        debug_assert_eq!(pollped_total.load(Ordering::Relaxed), $n * MT_COUNT);
                    }
                    total
                });
            });
        )+
    };
}

macro_rules! bench_kasino_topology_mpmc {
    ($group:expr, $name:literal, $Sched:ty, [$($n:literal),+ $(,)?]) => {
        $(
            $group.throughput(Throughput::Elements(($n * MT_COUNT) as u64));
            $group.bench_function(BenchmarkId::new($name, $n), |b| {
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        let bandit: InlineBandit<QAdapter<u64, MT_SUB_CAP>, $Sched, SUB_QUEUE_COUNT, MT_SUB_CAP> =
                            InlineBandit::new();
                        let pollped_total = AtomicUsize::new(0);
                        let start = Instant::now();
                        std::thread::scope(|scope| {
                            for _ in 0..$n {
                                let mut arm = bandit.buy_in();
                                let mut pop_arm = arm.fork();
                                scope.spawn(move || {
                                    arm.gambler().pin_thread().unwrap();
                                    for i in 0..MT_COUNT {
                                        let mut b = Backoff::new();
                                        while arm.offer(i as u64).is_err() {
                                            b.spin();
                                        }
                                    }
                                });

                                let pollped_total = &pollped_total;
                                scope.spawn(move || {
                                    pop_arm.gambler().pin_thread().unwrap();
                                    let mut pollped = 0usize;
                                    while pollped < MT_COUNT {
                                        let mut b = Backoff::new();
                                        if pop_arm.poll(()).is_ok() {
                                            pollped += 1;
                                        } else {
                                            b.spin();
                                        }
                                    }
                                    pollped_total.fetch_add(pollped, Ordering::Relaxed);
                                });
                            }
                        });
                        total += start.elapsed();
                        debug_assert_eq!(pollped_total.load(Ordering::Relaxed), $n * MT_COUNT);
                    }
                    total
                });
            });
        )+
    };
}

fn bench_mpmc(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpmc");

    macro_rules! bench_raw_mpmc {
        ([$($n:literal),+ $(,)?]) => {
            $(
                group.throughput(Throughput::Elements(($n * MT_COUNT) as u64));
                group.bench_function(BenchmarkId::new("raw_shared", $n), |b| {
                    b.iter_custom(|iters| {
                        let mut total = Duration::ZERO;
                        for _ in 0..iters {
                            let q = RawArrayQueue::<u64>::new(SUB_QUEUE_COUNT * MT_SUB_CAP);
                            let start = Instant::now();
                            std::thread::scope(|scope| {
                                for _ in 0..$n {
                                    let q = &q;
                                    scope.spawn(move || {
                                        for i in 0..MT_COUNT {
                                            retry_push(q, i as u64);
                                        }
                                    });
                                }
                                for _ in 0..$n {
                                    let q = &q;
                                    scope.spawn(move || {
                                        for _ in 0..MT_COUNT {
                                            black_box(retry_pop(q));
                                        }
                                    });
                                }
                            });
                            total += start.elapsed();
                        }
                        total
                    });
                });
            )+
        };
    }
    bench_raw_mpmc!([1, 2, 4, 8, 64]);

    bench_kasino_mpmc!(group, "random", RandomAccess<SmallRng>, [1, 2, 4, 8, 64]);
    bench_kasino_mpmc!(group, "round_robin", RoundRobin, [1, 2, 4, 8, 64]);
    bench_kasino_mpmc!(group, "dcbo", DCBO<2>, [1, 2, 4, 8, 64]);
    bench_kasino_mpmc!(group, "dra", DRA<2>, [1, 2, 4, 8, 64]);
    bench_kasino_topology_mpmc!(
        group,
        "core_affinity",
        CorePinned<CoreAffinity2>,
        [1, 2, 4, 8, 64]
    );

    group.finish();
}

criterion_group!(benches, bench_mpmc);
criterion_main!(benches);
