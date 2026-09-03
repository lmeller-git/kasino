//! Strategies used in this crate

mod collect;
mod dcbo;
mod dra;
mod random;
mod round_robin;

use core::ops::{Deref, DerefMut};

pub use collect::{DoubleCollect, NoCollect, policy};
use crossbeam_utils::CachePadded;
pub use dcbo::DCBO;
pub use dra::DRA;
pub use random::RandomAccess;
pub use round_robin::RoundRobin;

use crate::{
    Collection,
    Signature,
    storage::StorageBackend,
    sync::atomic::{AtomicUsize, Ordering},
};

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
    fn collect<'b, 'c>(
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
    /// The type of stake in an arm associated with this hook
    type Stake: Default + Hook;
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

impl<T> Hook for CachePadded<T>
where
    T: Hook,
{
    #[inline]
    fn on_offer_succ(&self) {
        T::on_offer_succ(self);
    }

    #[inline]
    fn on_offer_fail(&self) {
        T::on_offer_fail(self);
    }

    #[inline]
    fn on_poll_succ(&self) {
        T::on_poll_succ(self);
    }

    #[inline]
    fn on_poll_fail(&self) {
        T::on_poll_fail(self);
    }
}

/// a transparent wrapper around a T
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoPad<T>(T);

impl<T> Deref for NoPad<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NoPad<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Hook for NoPad<T>
where
    T: Hook,
{
    #[inline]
    fn on_offer_succ(&self) {
        T::on_offer_succ(self);
    }

    #[inline]
    fn on_offer_fail(&self) {
        T::on_offer_fail(self);
    }

    #[inline]
    fn on_poll_succ(&self) {
        T::on_poll_succ(self);
    }

    #[inline]
    fn on_poll_fail(&self) {
        T::on_poll_fail(self);
    }
}
