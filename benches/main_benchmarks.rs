use std::{
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
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
use rand::rngs::SmallRng;

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

// ============================== (a) SINGLE-THREADED ==============================
//
// Alternating push/pop on one thread, so the structure stays near-empty
// and never hits capacity edge cases -- isolates per-op overhead. Swept
// across N (sub-queue count) per scheduler, even single-threaded, to see
// how much of the overhead is fixed (visible at N=1, where sampling is
// trivial) vs. scales with N (the actual d-sample comparison cost).

const ST_SUB_CAP: usize = 64;
const SUB_QUEUE_COUNT: usize = 32;

macro_rules! bench_kasino_single_threaded {
    ($group:expr, $name:literal, $Sched:ty, [$($n:literal),+ $(,)?]) => {
        $(
            $group.throughput(Throughput::Elements(1));
            $group.bench_function(BenchmarkId::new($name, $n), |b| {
                let bandit: InlineBandit<QAdapter<u64, ST_SUB_CAP>, $Sched, $n, ST_SUB_CAP> =
                    InlineBandit::new();
                let mut arm = bandit.buy_in();
                b.iter(|| {
                    _ = arm.offer(black_box(42u64));
                    black_box(arm.poll(()))
                });
            });
        )+
    };
}

fn bench_single_threaded(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_threaded_offer_poll");

    group.throughput(Throughput::Elements(1));
    group.bench_function("raw_array_queue", |b| {
        let q = RawArrayQueue::<u64>::new(SUB_QUEUE_COUNT);
        b.iter(|| {
            _ = q.push(black_box(42u64));
            black_box(q.pop())
        });
    });

    bench_kasino_single_threaded!(group, "random", RandomAccess<SmallRng>, [1, 2, 4]);
    bench_kasino_single_threaded!(group, "round_robin", RoundRobin, [1, 2, 4]);
    bench_kasino_single_threaded!(group, "dcbo", DCBO<2>, [1, 2, 4]);
    bench_kasino_single_threaded!(group, "dra", DRA<2>, [1, 2, 4]);

    group.finish();
}

// ============================== (b) MULTITHREADED ==============================
//
// shared ArrayQueue hammered by the same thread count, capacity-matched
// (N * SUB_CAP) so it isn't handicapped on capacity alone -- the
// comparison is meant to isolate scheduling/sharding benefit, not starve
// the baseline. Each criterion "iteration" spawns the full thread set,
// runs COUNT ops/thread, joins, and reports wall time -- iter_custom is
// used so thread spawn/join cost is inside the timed region (it's the
// real cost of using this shape concurrently) but batched sanely rather
// than re-measuring a single op's spawn overhead.

const MT_SUB_CAP: usize = 128;
const MT_COUNT: usize = 20_000;

macro_rules! bench_kasino_mpsc {
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
                            let mut consumer = root.fork();
                            for _ in 0..($n * MT_COUNT) {
                                let mut b = Backoff::new();
                                while consumer.poll(()).is_err() {
                                    b.spin();
                                }
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

fn bench_mpsc(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpsc");

    macro_rules! bench_raw_mpsc {
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
                                for _ in 0..($n * MT_COUNT) {
                                    black_box(retry_pop(&q));
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
    bench_raw_mpsc!([1, 2, 4, 8, 64]);

    bench_kasino_mpsc!(group, "random", RandomAccess<SmallRng>, [1, 2, 4, 8, 64]);
    bench_kasino_mpsc!(group, "round_robin", RoundRobin, [1, 2, 4, 8, 64]);
    bench_kasino_mpsc!(group, "dcbo", DCBO<2>, [1, 2, 4, 64]);
    bench_kasino_mpsc!(group, "dra", DRA<2>, [1, 2, 4, 8, 64]);

    group.finish();
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

    group.finish();
}

criterion_group!(benches, bench_single_threaded, bench_mpsc, bench_mpmc);
criterion_main!(benches);
