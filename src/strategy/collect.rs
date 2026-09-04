use core::{
    cell::Cell,
    marker::PhantomData,
    ops::{Deref, Index},
};

use crossbeam_utils::CachePadded;

use crate::{
    Collection,
    Signature,
    storage::StorageBackend,
    strategy::{Hook, Hooked, Strategy},
    sync::atomic::{AtomicUsize, Ordering},
};

/// Does not collect any remaining items
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct NoCollect<S>(S);

impl<Q: Collection, S: Strategy<Q>> Strategy<Q> for NoCollect<S> {
    type Gambler = S::Gambler;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        self.0.choose_offer_arm(state, gambler)
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        choose_to: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        self.0.choose_poll_arm(choose_to, gambler)
    }

    #[inline]
    fn fork_gambler(&self, gambler: &Self::Gambler) -> Self::Gambler {
        self.0.fork_gambler(gambler)
    }

    #[inline]
    fn create_gambler(&self) -> Self::Gambler {
        self.0.create_gambler()
    }

    #[inline]
    fn on_poll_fail<'b, 'c>(
        &self,
        _state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        _sub_collections: &'c impl StorageBackend<Q>,
        _input: <Q::PollSignature as Signature>::Input<'b>,
    ) -> Option<(<Q::PollSignature as Signature>::Output<'b, 'c>, usize)>
    where
        Q: 'c,
    {
        None
    }
}

pub(crate) trait View<'a, T> {
    fn project(&'a self) -> &'a T;
}

impl<'a, K, U, T> View<'a, T> for K
where
    K: Deref<Target = U>,
    U: View<'a, T> + 'a,
{
    fn project(&'a self) -> &'a T {
        U::project(self)
    }
}

pub(crate) struct StorageView<'a, B, T, K> {
    backend: &'a B,
    _phantom: PhantomData<(&'a T, &'a K)>,
}

impl<'a, B, T, K> StorageView<'a, B, T, K> {
    fn new(backend: &'a B) -> Self {
        Self {
            backend,
            _phantom: PhantomData,
        }
    }
}

impl<'a, B: Index<usize>, T, K> Index<usize> for StorageView<'a, B, T, K>
where
    B::Output: View<'a, T>,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.backend[index].project()
    }
}

impl<'b, B, T, K> StorageBackend<T> for StorageView<'b, B, T, K>
where
    B: StorageBackend<K>,
    K: View<'b, T>,
{
    type Rebind<U> = B::Rebind<U>;

    fn len(&self) -> usize {
        self.backend.len()
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.backend.iter().map(|i| i.project())
    }

    fn is_empty(&self) -> bool {
        self.backend.is_empty()
    }

    fn map_to_buffer<U>(&self, f: impl Fn(usize) -> U) -> Self::Rebind<U> {
        self.backend.map_to_buffer(f)
    }
}

impl<'a, B, T, K> IntoIterator for StorageView<'a, B, T, K> {
    type IntoIter = core::array::IntoIter<(), 0>;
    type Item = ();

    fn into_iter(self) -> Self::IntoIter {
        [].into_iter()
    }
}

pub mod policy {
    //! Policies dictating when global state must be rechecked by a collection strategy.

    /// A policy that dictates that the global state needs to be rechecked if a concurrent call to [`crate::Collection::offer`] happens.
    #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    pub struct OfferInvalidate;
    /// A policy that dictates that the global state needs to be rechecked if a concurrent call to [`crate::Collection::poll`] happens.
    #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    pub struct PollInvalidate;
    /// A policy that dictates that the global state needs to be rechecked if a concurrent call to [`crate::Collection::offer`] or [`crate::Collection::poll`] happens.
    #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    pub struct OfferAndPollInvalidate;

    pub(crate) trait InvalidationPolicy {
        const INVALIDATE_ON_POLL: bool;
        const INVALIDATE_ON_OFFER: bool;
    }

    impl InvalidationPolicy for OfferInvalidate {
        const INVALIDATE_ON_OFFER: bool = true;
        const INVALIDATE_ON_POLL: bool = false;
    }

    impl InvalidationPolicy for OfferAndPollInvalidate {
        const INVALIDATE_ON_OFFER: bool = true;
        const INVALIDATE_ON_POLL: bool = true;
    }

    impl InvalidationPolicy for PollInvalidate {
        const INVALIDATE_ON_OFFER: bool = false;
        const INVALIDATE_ON_POLL: bool = true;
    }
}

use policy::{InvalidationPolicy, OfferInvalidate};

/// Runs a double collect on a failed poll.
///
/// This strategy promises empty-linearizability, given the same holds for the raw [`Collection`].
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct DoubleCollect<S, P = OfferInvalidate>(S, PhantomData<P>);

impl<S, P> DoubleCollect<S, P> {
    /// Constructs a new `DoubleCollect`.
    #[inline]
    pub const fn new(strategy: S) -> Self {
        Self(strategy, PhantomData)
    }
}

#[expect(unnameable_types)]
pub struct DoubleCollectGambler<A, P> {
    gambler: A,
    _policy: PhantomData<P>,
}

#[expect(unnameable_types)]
#[derive(Debug)]
pub struct DoubleCollectState<S, P> {
    strategy: S,
    epoch: AtomicUsize,
    _policy: PhantomData<P>,
}

impl<S: Default, P> Default for DoubleCollectState<S, P> {
    fn default() -> Self {
        Self {
            strategy: Default::default(),
            epoch: Default::default(),
            _policy: PhantomData,
        }
    }
}

impl<'a, S, P> View<'a, S> for DoubleCollectState<S, P> {
    fn project(&'a self) -> &'a S {
        &self.strategy
    }
}

impl<A: Hooked, P: InvalidationPolicy> Hooked for DoubleCollectGambler<A, P> {
    type Stake = CachePadded<DoubleCollectState<A::Stake, P>>;
}

impl<T: Hook, P: InvalidationPolicy> Hook for DoubleCollectState<T, P> {
    fn on_offer_succ(&self) {
        if P::INVALIDATE_ON_OFFER {
            self.epoch.fetch_add(1, Ordering::Release);
        }

        self.strategy.on_offer_succ();
    }

    fn on_poll_succ(&self) {
        if P::INVALIDATE_ON_POLL {
            self.epoch.fetch_add(1, Ordering::Release);
        }

        self.strategy.on_poll_succ();
    }
}

impl<S: Strategy<Q>, Q: Collection, P: InvalidationPolicy> Strategy<Q> for DoubleCollect<S, P> {
    type Gambler = DoubleCollectGambler<S::Gambler, P>;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        let idx = self
            .0
            .choose_offer_arm(&StorageView::new(state), &mut gambler.gambler);
        idx
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        let idx = self
            .0
            .choose_poll_arm(&StorageView::new(state), &mut gambler.gambler);
        idx
    }

    #[inline]
    fn fork_gambler(&self, gambler: &Self::Gambler) -> Self::Gambler {
        DoubleCollectGambler {
            gambler: self.0.fork_gambler(&gambler.gambler),
            _policy: PhantomData,
        }
    }

    #[inline]
    fn create_gambler(&self) -> Self::Gambler {
        DoubleCollectGambler {
            gambler: self.0.create_gambler(),
            _policy: PhantomData,
        }
    }

    #[expect(clippy::missing_inline_in_public_items)]
    fn on_poll_fail<'b, 'c>(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        sub_collections: &'c impl StorageBackend<Q>,
        input: <Q::PollSignature as Signature>::Input<'b>,
    ) -> Option<(<Q::PollSignature as Signature>::Output<'b, 'c>, usize)>
    where
        Q: 'c,
    {
        let versions = state.map_to_buffer(|_| Cell::new(0));

        'collect: loop {
            for (i, (item, epoch_slot)) in state.iter().zip(versions.iter()).enumerate() {
                let epoch = item.epoch.load(Ordering::Acquire);
                if let Ok(item) = sub_collections[i].poll(input) {
                    return Some((item, i));
                }
                epoch_slot.set(epoch);
            }

            for (item, stored_epoch) in state.iter().zip(versions.iter()) {
                let epoch = item.epoch.load(Ordering::Acquire);
                if stored_epoch.get() < epoch {
                    continue 'collect;
                }
            }

            return None;
        }
    }
}
