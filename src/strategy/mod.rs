//! Strategies used in this crate

mod collect;
mod dcbo;
mod dra;
mod random;
mod round_robin;

use core::ops::{Deref, DerefMut};

pub use collect::{DoubleCollectPoll, LinearCollectOffer, NoCollectPoll, policy};
pub(crate) use collect::{StorageView, View};
pub use dcbo::DCBO;
pub use dra::DRA;
pub use random::RandomAccess;
pub use round_robin::RoundRobin;
pub mod padded;

use crate::{
    Collection,
    Signature,
    storage::StorageBackend,
    strategy::padded::{PaddingRequest, truthiness::Truthiness},
    sync::atomic::{AtomicUsize, Ordering},
};

/// The fully type reolved and potentially cachepadded stake of this hook.
#[expect(type_alias_bounds)]
pub type ResolvedStake<H: Hooked> =
    <<H as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<<H as Hooked>::Stake>;
/// The [`ResolvedStake`] of this strategies hook.
#[expect(type_alias_bounds)]
pub type StrategyStakes<S: Strategy<Q>, Q: Collection> = ResolvedStake<<S as Strategy<Q>>::Gambler>;

/// A strategy that determines which arm is pulled next by a gambler
pub trait Strategy<Q: Collection> {
    /// An owned gambler, keeping track of its history to make decisions based on this strategy.
    type Gambler: Hooked;

    /// choose the next arm that we call [`Collection::offer`] on
    #[must_use = "a pulled arm should be used and the result communicated back to the gambler via the `Hooked` trait"]
    fn choose_offer_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize;
    /// choose the next arm that we call [`Collection::poll`] on
    #[must_use = "a pulled arm should be used and the result communicated back to the gambler via the `Hooked` trait"]
    fn choose_poll_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize;

    /// forks a gambler into a new one
    #[must_use]
    fn fork_gambler(&self, parent: &Self::Gambler) -> Self::Gambler;
    /// creates a new owned gambler with default values
    #[must_use]
    fn create_gambler(&self) -> Self::Gambler;

    /// Ensure that we checked all arms in a consistent manner after [`Collection::poll`] failed on the arm we pulled.
    #[inline]
    fn on_poll_fail<'b, 'c>(
        &self,
        _state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        bandit_arms: &'c impl StorageBackend<Q>,
        input: <Q::PollSignature as Signature>::Input<'b>,
    ) -> Option<(<Q::PollSignature as Signature>::Output<'b, 'c>, usize)>
    where
        Q: 'c,
    {
        for (i, q) in bandit_arms.iter().enumerate() {
            if let Ok(item) = q.poll(input) {
                return Some((item, i));
            }
        }
        None
    }

    /// Ensure that we checked all arms in a consistent manner after [`Collection::offer`] failed on the arm we pulled.
    #[expect(clippy::type_complexity)]
    #[inline]
    fn on_offer_fail<'b, 'c>(
        &self,
        _state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        _bandit_arms: &'c impl StorageBackend<Q>,
        input: <Q::OfferSignature as Signature>::Error<'b, 'c>,
    ) -> Result<
        (<Q::OfferSignature as Signature>::Output<'b, 'c>, usize),
        <Q::OfferSignature as Signature>::Error<'b, 'c>,
    >
    where
        Q: 'c,
    {
        Err(input)
    }
}

/// a hook for the stake in the bandits arms
pub trait Hook {
    /// mutate the state on a successful [`Collection::offer`]
    #[inline]
    fn on_offer_succ(&self) {}
    /// mutate the state on a failed [`Collection::offer`]
    #[inline]
    fn on_offer_fail(&self) {}
    /// mutate the state on a successful [`Collection::poll`]
    #[inline]
    fn on_poll_succ(&self) {}
    /// mutate the state on a failed [`Collection::poll`]
    #[inline]
    fn on_poll_fail(&self) {}
}

/// Callbacks that update the gamblers state and stakes based on the outcome of its decision.
pub trait Hooked {
    /// The type of stake in an arm associated with this hook.
    type Stake: Default + Hook;
    /// The type of padding that should be applied to this hooks stake.
    type RequestedPadding: PaddingRequest + Truthiness;
    /// Update the gambler on a successful [`Collection::offer`]
    #[inline]
    fn on_offer_succ(&mut self, sub_state: &Self::Stake) {
        sub_state.on_offer_succ();
    }

    /// Update the gambler on a failed [`Collection::offer`]
    #[inline]
    fn on_offer_fail(&mut self, sub_state: &Self::Stake) {
        sub_state.on_offer_fail();
    }

    /// Update the gambler on a successful [`Collection::poll`]
    #[inline]
    fn on_poll_succ(&mut self, sub_state: &Self::Stake) {
        sub_state.on_poll_succ();
    }

    /// Update the gambler on a failed [`Collection::poll`]
    #[inline]
    fn on_poll_fail(&mut self, sub_state: &Self::Stake) {
        sub_state.on_poll_fail();
    }
}

/// Intercepts some calls to the  wrapped state and records data about it.
#[derive(Debug, Default)]
pub struct InstrumentedState<T> {
    #[cfg(feature = "instrumented")]
    offer_count: AtomicUsize,
    #[cfg(feature = "instrumented")]
    poll_count: AtomicUsize,
    sched_state: T,
}

#[cfg(feature = "instrumented")]
impl<T> InstrumentedState<T> {
    /// The count of offers on a sub collection
    #[inline]
    pub fn offer_count(&self) -> usize {
        self.offer_count.load(Ordering::Relaxed)
    }

    /// The count of polls on a sub collection
    #[inline]
    pub fn poll_count(&self) -> usize {
        self.poll_count.load(Ordering::Relaxed)
    }
}

impl<T> Deref for InstrumentedState<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.sched_state
    }
}

impl<T> DerefMut for InstrumentedState<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sched_state
    }
}

impl<T> Clone for InstrumentedState<T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            #[cfg(feature = "instrumented")]
            offer_count: self.offer_count.load(Ordering::Relaxed).into(),
            #[cfg(feature = "instrumented")]
            poll_count: self.poll_count.load(Ordering::Relaxed).into(),
            sched_state: self.sched_state.clone(),
        }
    }
}

impl<T> Hook for InstrumentedState<T>
where
    T: Hook,
{
    #[inline]
    fn on_offer_succ(&self) {
        #[cfg(feature = "instrumented")]
        self.offer_count.fetch_add(1, Ordering::Relaxed);
        self.sched_state.on_offer_succ();
    }

    #[inline]
    fn on_offer_fail(&self) {
        self.sched_state.on_offer_fail();
    }

    #[inline]
    fn on_poll_succ(&self) {
        #[cfg(feature = "instrumented")]
        self.poll_count.fetch_add(1, Ordering::Relaxed);
        self.sched_state.on_poll_succ();
    }

    #[inline]
    fn on_poll_fail(&self) {
        self.sched_state.on_poll_fail();
    }
}

/// Stores the count of succesful offers and polls on a sub collection
#[derive(Default, Debug)]
pub struct EDCount {
    offer_count: AtomicUsize,
    poll_count: AtomicUsize,
}

impl Clone for EDCount {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            offer_count: self.offer_count.load(Ordering::Relaxed).into(),
            poll_count: self.poll_count.load(Ordering::Relaxed).into(),
        }
    }
}

impl EDCount {
    /// The count of offers on a sub collection
    #[inline]
    pub fn offer_count(&self) -> usize {
        self.offer_count.load(Ordering::Relaxed)
    }

    /// The count of polls on a sub collection
    #[inline]
    pub fn poll_count(&self) -> usize {
        self.poll_count.load(Ordering::Relaxed)
    }
}

impl Hook for EDCount {
    #[inline]
    fn on_offer_succ(&self) {
        self.offer_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn on_poll_succ(&self) {
        self.poll_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl Hook for () {}
