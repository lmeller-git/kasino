#![allow(unused)]

use std::collections::{HashSet, VecDeque};

use crate::{
    BanditHandle,
    Collection,
    Signature,
    WithCapacity,
    components::{PopSignature, TryPushSignature},
    storage::StorageBackend,
    strategy::{Hooked, Strategy, StrategyStakes},
    sync::Mutex,
};

pub(crate) trait MutAccessForkCollection {
    type Item;
    fn fork(&self) -> Self;
    fn enqueue(&mut self, item: Self::Item) -> Result<(), Self::Item>;
    fn dequeue(&mut self) -> Option<Self::Item>;
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn is_full(&self) -> bool;

    /// Note that this is not an actual force push: evicting an item from sub collection K does not mean the other K - 1 subcollections now have a free spot. It does not mean the next enqueue will succeed sequentially.
    fn force_push(&mut self, item: Self::Item) -> Option<Self::Item> {
        let mut item_container = None;
        self.force_push_and_do(item, |item| {
            item_container.replace(item);
        });
        item_container
    }

    /// Note that this is not an actual force push: evicting an item from sub collection K does not mean the other K - 1 subcollections now have a free spot. It does not mean the next enqueue will succeed sequentially.
    fn force_push_and_do<F>(&mut self, mut item: Self::Item, mut f: F)
    where
        F: FnMut(Self::Item),
    {
        let mut backoff = Backoff::new();
        while let Err(item_) = self.enqueue(item) {
            item = item_;
            backoff.backoff();
            if let Some(next_popped_item) = self.dequeue() {
                f(next_popped_item);
            }
        }
    }
}

impl<'a, Q, S, B, C, T, const SUB_CAP: usize> MutAccessForkCollection
    for BanditHandle<'a, Q, S, B, C, SUB_CAP>
where
    Q: Collection<PollSignature = PopSignature<T>, OfferSignature = TryPushSignature<T>>,
    S: Strategy<Q>,
    B: StorageBackend<Q>,
    C: StorageBackend<StrategyStakes<S, Q>>,
{
    type Item = T;

    fn fork(&self) -> Self {
        self.fork()
    }

    fn enqueue(&mut self, item: Self::Item) -> Result<(), Self::Item> {
        self.offer(item)
    }

    fn dequeue(&mut self) -> Option<Self::Item> {
        self.poll(()).ok()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }
}

pub(crate) struct LockedDeque<T> {
    raw: Mutex<VecDeque<T>>,
    cap: usize,
}

impl<T> Collection for LockedDeque<T> {
    type OfferSignature = TryPushSignature<T>;
    type PollSignature = PopSignature<T>;

    fn offer<'a, 'b>(
        &'b self,
        item: <Self::OfferSignature as Signature>::Input<'a>,
    ) -> Result<
        <Self::OfferSignature as Signature>::Output<'a, 'b>,
        <Self::OfferSignature as Signature>::Error<'a, 'b>,
    > {
        let mut lock = self.raw.lock();
        if lock.len() >= self.cap {
            return Err(item);
        }
        lock.push_front(item);
        Ok(())
    }

    fn poll<'a, 'b>(
        &'b self,
        _input: <Self::PollSignature as Signature>::Input<'a>,
    ) -> Result<
        <Self::PollSignature as Signature>::Output<'a, 'b>,
        <Self::PollSignature as Signature>::Error<'a, 'b>,
    > {
        self.raw.lock().pop_back().ok_or(())
    }

    fn len(&self) -> usize {
        self.raw.lock().len()
    }

    fn capacity(&self) -> usize {
        self.cap
    }
}

impl<T, const N: usize> WithCapacity<N> for LockedDeque<T> {
    fn with_capacity() -> Self {
        Self {
            raw: Mutex::new(VecDeque::with_capacity(N)),
            cap: N,
        }
    }
}

#[cfg(all(not(shuttle), not(loom)))]
const MAX_SPINLOOP: usize = 1024;

pub(crate) struct Backoff {
    #[cfg(all(not(shuttle), not(loom)))]
    state: usize,
}

impl Backoff {
    pub(crate) fn new() -> Self {
        #[cfg(all(not(shuttle), not(loom)))]
        return Self { state: 1 };
        #[cfg(any(shuttle, loom))]
        return Self {};
    }

    pub(crate) fn backoff(&mut self) {
        #[cfg(all(not(shuttle), not(loom)))]
        {
            for _ in 0..self.state {
                crate::sync::hint::spin_loop();
            }
            self.state = (self.state * 2).min(MAX_SPINLOOP);
        }
        #[cfg(any(shuttle, loom))]
        crate::sync::thread::yield_now();
    }
}

pub(crate) fn retry_enqueue<Q: MutAccessForkCollection>(q: &mut Q, item: Q::Item) {
    let mut item = item;
    let mut backoff = Backoff::new();
    loop {
        match q.enqueue(item) {
            Ok(()) => return,
            Err(back) => {
                item = back;
                backoff.backoff();
            }
        }
    }
}

pub(crate) fn retry_dequeue<Q: MutAccessForkCollection>(q: &mut Q) -> Q::Item {
    let mut backoff = Backoff::new();
    loop {
        if let Some(x) = q.dequeue() {
            return x;
        }
        backoff.backoff();
    }
}

pub(crate) fn drain_confirmed_empty<Q: MutAccessForkCollection>(
    q: &mut Q,
    mut on_item: impl FnMut(Q::Item),
) {
    let mut backoff = Backoff::new();
    while !q.is_empty() {
        match q.dequeue() {
            Some(item) => on_item(item),
            None => backoff.backoff(),
        }
    }
}

pub(crate) fn assert_exact_set(popped: &[u32], expected: impl IntoIterator<Item = u32>) {
    let expected: HashSet<u32> = expected.into_iter().collect();
    let mut seen = HashSet::with_capacity(popped.len());
    for &v in popped {
        assert!(expected.contains(&v), "value {v} was never pushed");
        assert!(seen.insert(v), "value {v} popped more than once");
    }
    assert_eq!(
        seen.len(),
        expected.len(),
        "not every pushed value was popped (lost {} item(s))",
        expected.len() - seen.len()
    );
}

#[cfg(not(any(loom, echeneis)))]
pub(crate) use tests::*;

#[cfg(not(any(loom, echeneis)))]
mod tests {
    use super::*;
    use crate::sync::{
        atomic::{AtomicUsize, Ordering},
        thread::scope,
    };

    pub(crate) fn smoke<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32>,
    {
        retry_enqueue(&mut q, 7);
        assert_eq!(retry_dequeue(&mut q), 7);
        retry_enqueue(&mut q, 8);
        assert_eq!(retry_dequeue(&mut q), 8);
        assert!(q.dequeue().is_none());
    }

    pub(crate) fn smoke_long<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32>,
    {
        retry_enqueue(&mut q, 7);
        assert_eq!(retry_dequeue(&mut q), 7);

        retry_enqueue(&mut q, 8);
        retry_enqueue(&mut q, 9);
        let popped = [retry_dequeue(&mut q), retry_dequeue(&mut q)];
        assert_exact_set(&popped, [8, 9]);
        assert!(q.dequeue().is_none());
    }

    pub(crate) fn len_empty_full<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = ()>,
    {
        assert_eq!(q.len(), 0);
        assert_eq!(q.capacity(), 2);
        assert!(q.is_empty());
        assert!(!q.is_full());

        retry_enqueue(&mut q, ());
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
        assert!(!q.is_full());

        retry_enqueue(&mut q, ());
        assert_eq!(q.len(), 2);
        assert!(!q.is_empty());
        assert!(q.is_full());

        retry_dequeue(&mut q);
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
        assert!(!q.is_full());
    }

    pub(crate) fn len<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32> + Sync + Send,
    {
        #[cfg(any(miri, loom, shuttle))]
        const COUNT: usize = 30;
        #[cfg(not(any(miri, loom, shuttle)))]
        const COUNT: usize = 25_000;
        let cap = q.capacity();
        let iters = cap / 20;

        assert_eq!(q.len(), 0);
        assert!(q.is_empty());

        for _ in 0..cap / 10 {
            for i in 0..iters {
                retry_enqueue(&mut q, i as u32);
                assert_eq!(q.len(), i + 1);
            }
            for i in 0..iters {
                retry_dequeue(&mut q);
                assert_eq!(q.len(), iters - i - 1);
            }
        }
        assert_eq!(q.len(), 0);
        assert!(q.is_empty());

        for i in 0..cap {
            retry_enqueue(&mut q, i as u32);
            assert_eq!(q.len(), i + 1);
        }
        assert!(q.is_full());
        assert_eq!(q.len(), cap);

        for _ in 0..cap {
            retry_dequeue(&mut q);
        }
        assert_eq!(q.len(), 0);
        assert!(q.is_empty());

        let mut popped_a = Vec::with_capacity(COUNT);
        let mut arm_push = q.fork();
        let mut arm_pop = q.fork();
        scope(|scope| {
            scope.spawn(|| {
                for i in 0..COUNT {
                    let _len = arm_push.len();
                    retry_enqueue(&mut arm_push, i as u32);
                }
            });
            scope.spawn(|| {
                for _ in 0..COUNT {
                    popped_a.push(retry_dequeue(&mut arm_pop));
                    let len = arm_pop.len();
                    assert!(len <= cap);
                }
            });
        });
        assert_exact_set(&popped_a, 0..COUNT as u32);
        assert_eq!(q.len(), 0);
    }

    pub(crate) fn force_push<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32>,
    {
        assert!(q.is_empty());
        let cap = q.capacity();

        for i in 0..cap {
            retry_enqueue(&mut q, i as u32);
        }
        assert!(q.is_full());
        assert!(q.enqueue(42).is_err());

        for _ in 0..cap {
            _ = q.force_push(42);
        }
        assert!(!q.is_empty());
    }

    pub(crate) fn spsc<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32> + Sync + Send,
    {
        #[cfg(any(miri, loom, shuttle))]
        const COUNT: usize = 50;
        #[cfg(not(any(miri, loom, shuttle)))]
        const COUNT: usize = 300_000;

        let mut popped = Vec::with_capacity(COUNT);
        let mut push_arm = q.fork();
        let mut pop_arm = q.fork();
        scope(|scope| {
            scope.spawn(|| {
                for _ in 0..COUNT {
                    popped.push(retry_dequeue(&mut pop_arm));
                }
                assert!(pop_arm.dequeue().is_none());
            });
            scope.spawn(|| {
                for i in 0..COUNT {
                    retry_enqueue(&mut push_arm, i as u32);
                }
            });
        });
        assert_exact_set(&popped, 0..COUNT as u32);
    }

    pub(crate) fn mpsc<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32> + Sync + Send,
    {
        #[cfg(any(miri, loom, shuttle))]
        const COUNT: usize = 10;
        #[cfg(not(any(miri, loom, shuttle)))]
        const COUNT: usize = 30_000;
        const THREADS: usize = 4;

        let v = (0..COUNT).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();
        let mut pop_arm = q.fork();

        scope(|scope| {
            for _ in 0..THREADS {
                let mut arm = q.fork();
                scope.spawn(move || {
                    for i in 0..COUNT {
                        retry_enqueue(&mut arm, i as u32);
                    }
                });
            }
            for _ in 0..THREADS {
                for _ in 0..COUNT {
                    let n = retry_dequeue(&mut pop_arm);
                    v[n as usize].fetch_add(1, Ordering::SeqCst);
                }
            }
        });

        for c in v {
            assert_eq!(c.load(Ordering::SeqCst), THREADS);
        }
    }

    pub(crate) fn mpmc<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32> + Sync + Send,
    {
        #[cfg(any(miri, loom, shuttle))]
        const COUNT: usize = 20;
        #[cfg(not(any(miri, loom, shuttle)))]
        const COUNT: usize = 75_000;
        const THREADS: usize = 4;

        let v = &(0..COUNT).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();

        scope(|scope| {
            for _ in 0..THREADS {
                let mut arm = q.fork();
                scope.spawn(move || {
                    for _ in 0..COUNT {
                        let n = retry_dequeue(&mut arm);
                        v[n as usize].fetch_add(1, Ordering::SeqCst);
                    }
                });
            }
            for _ in 0..THREADS {
                let mut arm = q.fork();
                scope.spawn(move || {
                    for i in 0..COUNT {
                        retry_enqueue(&mut arm, i as u32);
                    }
                });
            }
        });

        for c in v {
            assert_eq!(c.load(Ordering::SeqCst), THREADS);
        }
    }

    pub(crate) fn mpmc_ring_buffer<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32> + Sync + Send,
    {
        #[cfg(any(miri, loom, shuttle))]
        const COUNT: usize = 20;
        #[cfg(not(any(miri, loom, shuttle)))]
        const COUNT: usize = 75_000;
        const THREADS: usize = 2;

        let t = AtomicUsize::new(THREADS);
        let v = (0..COUNT).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();

        scope(|scope| {
            for _ in 0..THREADS {
                let mut arm = q.fork();
                let t = &t;
                let v = &v;
                scope.spawn(move || {
                    loop {
                        drain_confirmed_empty(&mut arm, |n| {
                            v[n as usize].fetch_add(1, Ordering::SeqCst);
                        });
                        if t.load(Ordering::SeqCst) == 0 && arm.is_empty() {
                            break;
                        }
                        Backoff::new().backoff();
                    }
                });
            }

            for _ in 0..THREADS {
                let mut arm = q.fork();
                let t = &t;
                let v = &v;
                scope.spawn(move || {
                    for i in 0..COUNT {
                        arm.force_push_and_do(i as u32, |n| {
                            v[n as usize].fetch_add(1, Ordering::SeqCst);
                        });
                    }
                    t.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });

        for c in v {
            assert_eq!(c.load(Ordering::SeqCst), THREADS);
        }
    }

    /// empty-linearizability on non-relaxed specification
    pub(crate) fn linearizable<Q>(mut q: Q)
    where
        Q: MutAccessForkCollection<Item = u32> + Sync + Send,
    {
        #[cfg(any(miri, loom, shuttle))]
        const COUNT: usize = 50;
        #[cfg(not(any(miri, loom, shuttle)))]
        const COUNT: usize = 25_000;
        const THREADS: usize = 4;

        scope(|scope| {
            for _ in 0..THREADS {
                let mut arm = q.fork();
                scope.spawn(move || {
                    for _ in 0..COUNT {
                        while arm.enqueue(42).is_err() {
                            Backoff::new().backoff();
                        }
                        arm.dequeue().unwrap();
                    }
                });
            }
        });
    }
}
