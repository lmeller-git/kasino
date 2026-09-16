//! Advanced hadware aware strategies for `kasino`.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![deny(missing_docs)]
#![deny(clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)]
#![warn(unsafe_op_in_unsafe_fn)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[allow(unused_extern_crates)]
#[cfg(any(feature = "alloc", test))]
extern crate alloc;

mod sync;

use core::{
    hint::spin_loop,
    marker::PhantomData,
    ops::{Index, RangeBounds},
};

use kasino::{
    Collection,
    strategy::{Hooked, Strategy, padded::NoPadding},
};

use crate::sync::atomic::AtomicBool;

/// todo
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct CoreID(pub usize);

/// todo
pub trait RuntimeData {
    /// todo
    type Error;

    /// todo
    fn get_affinity() -> Result<CoreID, Self::Error>;
    /// todo
    fn set_affinity(id: CoreID) -> Result<(), Self::Error>;
    /// todo
    fn available_cores() -> impl Iterator<Item = CoreID>;
    /// todo
    fn available_parallelism() -> Result<core::num::NonZero<usize>, Self::Error>;
}

/// todo
#[derive(Debug, Default)]
pub struct CorePinned<R> {
    _dat: PhantomData<R>,
}

/// todo
#[must_use]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CorePinnedGambler<R> {
    affinity: CoreID,
    _dat: PhantomData<R>,
}

impl<R> Copy for CorePinnedGambler<R> {}

impl<R> Clone for CorePinnedGambler<R> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: RuntimeData> CorePinnedGambler<R> {
    /// todo
    #[inline]
    pub fn pin_thread(&mut self, core: CoreID) {
        self.affinity = core
    }
}

impl<Q: Collection, R: RuntimeData> Strategy<Q> for CorePinned<R> {
    type Gambler = CorePinnedGambler<R>;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        gambler.affinity.0 % state.len()
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        gambler.affinity.0 % state.len()
    }

    #[inline]
    fn fork_gambler(&self, parent: &Self::Gambler) -> Self::Gambler {
        *parent
    }

    #[expect(clippy::missing_inline_in_public_items)]
    fn create_gambler(&self) -> Self::Gambler {
        CorePinnedGambler {
            affinity: Default::default(),
            _dat: PhantomData,
        }
    }

    fn on_poll_fail<'b, 'c>(
        &self,
        _state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        _bandit_arms: &'c impl kasino::storage::StorageBackend<Q>,
        _input: <<Q as Collection>::PollSignature as kasino::prelude::Signature>::Input<'b>,
        _gambler: &mut Self::Gambler,
    ) -> Option<(
        <<Q as Collection>::PollSignature as kasino::prelude::Signature>::Output<'b, 'c>,
        usize,
    )>
    where
        Q: 'c,
    {
        None
    }

    fn on_offer_fail<'b, 'c>(
        &self,
        _state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        _bandit_arms: &'c impl kasino::storage::StorageBackend<Q>,
        input: <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::Error<'b, 'c>,
        _gambler: &mut Self::Gambler,
    ) -> Result<
        (
            <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::Output<'b, 'c>,
            usize,
        ),
        <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::Error<'b, 'c>,
    >
    where
        Q: 'c,
    {
        Err(input)
    }
}

impl<R> Hooked for CorePinnedGambler<R> {
    type RequestedPadding = NoPadding;
    type Stake = ();
}

/// todo
pub struct SlicedCorePinned<R, S> {
    choose: S,
    registry: CoreRegistry,
    _dat: PhantomData<R>,
}

impl<R, S: Default> Default for SlicedCorePinned<R, S> {
    fn default() -> Self {
        Self {
            choose: Default::default(),
            registry: CoreRegistry::new(),
            _dat: PhantomData,
        }
    }
}

/// todo
#[must_use]
pub struct SlicedCorePinnedGambler<'a, R, G> {
    affinity: CoreID,
    choosing_gambler: G,
    parent: &'a CoreRegistry,
    _dat: PhantomData<R>,
}

impl<'a, R, G: Copy> Copy for SlicedCorePinnedGambler<'a, R, G> {}

impl<'a, R, G: Clone> Clone for SlicedCorePinnedGambler<'a, R, G> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            affinity: self.affinity,
            choosing_gambler: self.choosing_gambler.clone(),
            parent: self.parent,
            _dat: PhantomData,
        }
    }
}

impl<'a, R: RuntimeData, G> SlicedCorePinnedGambler<'a, R, G> {
    /// todo
    #[inline]
    pub fn pin_thread(&mut self, core: CoreID) {
        self.affinity = core;
    }
}

impl<Q: Collection, R: RuntimeData, S: Strategy<Q>> Strategy<Q> for SlicedCorePinned<R, S> {
    type Gambler = SlicedCorePinnedGambler<R, S::Gambler>;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        let (count, rank) = self.registry.get_active_rank(gambler.affinity.0);
        let (start, len) = get_core_slice(rank, count, state.len());

        if len == 1 {
            start
        } else {
            self.choose.choose_offer_arm(
                &SliceStorageView {
                    backend: state,
                    start,
                    len,
                },
                &mut gambler.choosing_gambler,
            )
        }
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        let (count, rank) = self.registry.get_active_rank(gambler.affinity.0);
        let (start, len) = get_core_slice(rank, count, state.len());

        if len == 1 {
            start
        } else {
            self.choose.choose_offer_arm(
                &SliceStorageView {
                    backend: state,
                    start,
                    len,
                },
                &mut gambler.choosing_gambler,
            )
        }
    }

    #[inline]
    fn fork_gambler(&self, parent: &Self::Gambler) -> Self::Gambler {
        SlicedCorePinnedGambler {
            affinity: parent.affinity,
            choosing_gambler: self.choose.fork_gambler(&parent.choosing_gambler),
            parent: &self.registry,
            _dat: PhantomData,
        }
    }

    #[expect(clippy::missing_inline_in_public_items)]
    fn create_gambler(&self) -> Self::Gambler {
        SlicedCorePinnedGambler {
            affinity: Default::default(),
            choosing_gambler: self.choose.create_gambler(),
            parent: &self.registry,
            _dat: PhantomData,
        }
    }

    fn on_poll_fail<'b, 'c>(
        &self,
        state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        bandit_arms: &'c impl kasino::storage::StorageBackend<Q>,
        input: <<Q as Collection>::PollSignature as kasino::prelude::Signature>::Input<'b>,
        gambler: &mut Self::Gambler,
    ) -> Option<(
        <<Q as Collection>::PollSignature as kasino::prelude::Signature>::Output<'b, 'c>,
        usize,
    )>
    where
        Q: 'c,
    {
        let (count, rank) = self.registry.get_active_rank(gambler.affinity.0);
        let (start, len) = get_core_slice(rank, count, state.len());

        for i in start..start + len {
            if let Ok(o) = bandit_arms[i].poll(input) {
                return Some((o, i));
            }
        }

        self.choose
            .on_poll_fail(state, bandit_arms, input, &mut gambler.choosing_gambler)
    }

    fn on_offer_fail<'b, 'c>(
        &self,
        state: &impl kasino::storage::StorageBackend<<Self::Gambler as Hooked>::Stake>,
        bandit_arms: &'c impl kasino::storage::StorageBackend<Q>,
        mut input: <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::Error<'b, 'c>,
        gambler: &mut Self::Gambler,
    ) -> Result<
        (
            <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::Output<'b, 'c>,
            usize,
        ),
        <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::Error<'b, 'c>,
    >
    where
        Q: 'c,
    {
        let (count, rank) = self.registry.get_active_rank(gambler.affinity.0);
        let (start, len) = get_core_slice(rank, count, state.len());

        for i in start..start + len {
            match bandit_arms[i].offer(
                <<Q as Collection>::OfferSignature as kasino::prelude::Signature>::reclaim_input(
                    input,
                )?,
            ) {
                Ok(res) => return Ok((res, i)),
                Err(e) => input = e,
            }
        }

        self.choose
            .on_offer_fail(state, bandit_arms, input, &mut gambler.choosing_gambler)
    }
}

impl<R, G: Hooked> Hooked for SlicedCorePinnedGambler<R, G> {
    type RequestedPadding = G::RequestedPadding;
    type Stake = G::Stake;

    fn on_offer_succ(&mut self, sub_state: &Self::Stake) {
        self.choosing_gambler.on_offer_succ(sub_state);
    }

    fn on_offer_fail(&mut self, sub_state: &Self::Stake) {
        self.choosing_gambler.on_offer_fail(sub_state);
    }

    fn on_poll_succ(&mut self, sub_state: &Self::Stake) {
        self.choosing_gambler.on_poll_succ(sub_state);
    }

    fn on_poll_fail(&mut self, sub_state: &Self::Stake) {
        self.choosing_gambler.on_poll_fail(sub_state);
    }
}

use crate::sync::atomic::{AtomicU16, AtomicU64, Ordering};

/// todo
pub struct SliceStorageView<'a, B> {
    backend: &'a B,
    start: usize,
    len: usize,
}

impl<'a, S, B: kasino::storage::StorageBackend<S>> kasino::storage::StorageBackend<S>
    for SliceStorageView<'a, B>
{
    type Rebind<U> = B::Rebind<U>;

    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }

    fn iter<'b>(&'b self) -> impl Iterator<Item = &'b S>
    where
        S: 'b,
    {
        self.backend.iter()
    }

    fn map_to_buffer<U>(&self, f: impl Fn(usize) -> U) -> Self::Rebind<U> {
        B::map_to_buffer(&self.backend, f)
    }

    fn is_empty(&self) -> bool {
        self.len == 0 || self.backend.is_empty()
    }
}

impl<'a, B: Index<usize>> Index<usize> for SliceStorageView<'a, B> {
    type Output = B::Output;

    fn index(&self, index: usize) -> &Self::Output {
        &self.backend[self.start + index]
    }
}

impl<'a, B: IntoIterator> IntoIterator for SliceStorageView<'a, B> {
    type IntoIter = B::IntoIter;
    type Item = B::Item;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

/// todo
pub struct CoreRegistry<const MAX_CORES: usize = 64> {
    active_mask: AtomicU64,
    core_counts: [AtomicU16; MAX_CORES],
    lock: AtomicBool,
}

impl<const MAX_CORES: usize> CoreRegistry<MAX_CORES> {
    /// todo
    #[inline]
    pub const fn new() -> Self {
        // Const initialization for no-alloc / no-std
        const ZERO: AtomicU16 = AtomicU16::new(0);
        Self {
            active_mask: AtomicU64::new(0),
            core_counts: [ZERO; MAX_CORES],
            lock: AtomicBool::new(false),
        }
    }

    /// COLD PATH: Called during thread register / pin_thread / drop
    #[inline]
    pub fn register_thread(&self, old_core: Option<usize>, new_core: usize) {
        while self.lock.swap(true, Ordering::Relaxed) {
            spin_loop();
        }
        if let Some(old) = old_core {
            if self.core_counts[old].fetch_sub(1, Ordering::Relaxed) == 1 {
                // Last thread left this core -> clear bit
                self.active_mask.fetch_and(!(1 << old), Ordering::Relaxed);
            }
        }
        if self.core_counts[new_core].fetch_add(1, Ordering::Relaxed) == 0 {
            // First thread joined this core -> set bit
            self.active_mask.fetch_or(1 << new_core, Ordering::Relaxed);
        }

        self.lock.store(false, Ordering::Relaxed);
    }

    /// HOT PATH: 1-2 CPU cycles. Reads active core count A and active rank r
    #[inline]
    pub fn get_active_rank(&self, core_id: usize) -> (usize, usize) {
        let mask = self.active_mask.load(Ordering::Relaxed);
        if mask == 0 {
            return (0, 1);
        }

        let active_count = mask.count_ones() as usize; // HW POPCNT instruction
        // Rank = count set bits to the right of core_id
        let lower_bits = mask & ((1u64 << core_id) - 1);
        let rank = lower_bits.count_ones() as usize;

        (rank, active_count)
    }
}

/// todo
#[inline]
pub fn get_core_slice(
    core_rank: usize,
    total_cores: usize,
    total_collections: usize,
) -> (usize, usize) {
    if total_collections < total_cores {
        (core_rank % total_collections, 1)
    } else {
        let base = total_collections / total_cores;
        let rem = total_collections % total_cores;
        let rank = core_rank % total_cores;

        let len = base + (rank < rem) as usize;
        let start = rank * base + rank.min(rem);
        (start, len)
    }
}
