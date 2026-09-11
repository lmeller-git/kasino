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

use core::marker::PhantomData;

use kasino::{
    Collection,
    strategy::{Hooked, Strategy, padded::NoPadding},
};

use crate::sync::atomic::{AtomicUsize, Ordering};

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
    counter: AtomicUsize,
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
    pub fn pin_thread(&mut self) -> Result<(), R::Error> {
        if let Ok(aff) = R::get_affinity() {
            self.affinity = aff;
            Ok(())
        } else {
            R::set_affinity(self.affinity)
        }
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
        let affinity = match R::get_affinity() {
            Ok(aff) => aff,
            Err(_) => {
                let total_cores = R::available_cores().count();
                let count = self.counter.fetch_add(1, Ordering::Relaxed);
                let idx = count % total_cores;
                let aff = R::available_cores().nth(idx).unwrap();
                debug_assert!(R::set_affinity(aff).is_ok());
                aff
            }
        };

        CorePinnedGambler {
            affinity,
            _dat: PhantomData,
        }
    }
}

impl<R> Hooked for CorePinnedGambler<R> {
    type RequestedPadding = NoPadding;
    type Stake = ();
}
