use core::ops::{Deref, DerefMut};

use rand::{RngExt, SeedableRng, rngs::SmallRng};

use crate::{
    Collection,
    storage::StorageBackend,
    strategy::{Hooked, InstrumentedState, NoPad, Strategy},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub(crate) struct PerThreadRndg<R = SmallRng> {
    rng: R,
    current_seed: AtomicU64,
}

impl<S> Default for PerThreadRndg<S>
where
    S: SeedableRng,
{
    #[inline]
    fn default() -> Self {
        Self {
            rng: S::seed_from_u64(0),
            current_seed: AtomicU64::new(1),
        }
    }
}

impl<R> Deref for PerThreadRndg<R> {
    type Target = R;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.rng
    }
}

impl<R> DerefMut for PerThreadRndg<R> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.rng
    }
}

impl<R: SeedableRng + rand::Rng> PerThreadRndg<R> {
    pub(crate) fn fork_thread(&self) -> Self {
        let my_seed = self.current_seed.fetch_add(1, Ordering::AcqRel);

        let mut my_rng = R::seed_from_u64(my_seed);
        let my_next_seed = my_rng.next_u64();

        Self {
            rng: my_rng,
            current_seed: my_next_seed.into(),
        }
    }
}

/// A random scheduler.
///
/// This scheduler does not promise a bound on rank error or delay.
#[derive(Debug)]
pub struct RandomAccess<R = SmallRng> {
    rng: PerThreadRndg<R>,
}

impl<S> Default for RandomAccess<S>
where
    S: SeedableRng,
{
    #[inline]
    fn default() -> Self {
        Self {
            rng: Default::default(),
        }
    }
}

impl<Q: Collection, S: RngExt + SeedableRng> Strategy<Q> for RandomAccess<S> {
    type Gambler = Self;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        gambler.rng.random_range(..state.len())
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        gambler.rng.random_range(..state.len())
    }

    #[inline]
    fn fork_gambler(&self, gambler: &Self::Gambler) -> Self::Gambler {
        Self {
            rng: gambler.rng.fork_thread(),
        }
    }

    #[inline]
    fn create_gambler(&self) -> Self::Gambler {
        Self::default()
    }
}

impl<S> Hooked for RandomAccess<S> {
    type Stake = NoPad<InstrumentedState<()>>;
}
