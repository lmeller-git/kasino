use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use rand::{RngExt, SeedableRng, rngs::SmallRng};

use crate::{
    Collection,
    storage::StorageBackend,
    strategy::{Hooked, InstrumentedState, NoPad, Strategy},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug)]
pub(crate) struct SplitMix64(AtomicU64);

impl SplitMix64 {
    pub(crate) fn next_u64(&self) -> u64 {
        let mut v = self.0.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
        v = (v ^ (v >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        v = (v ^ (v >> 27)).wrapping_mul(0x94d049bb133111eb);
        v ^ (v >> 31)
    }
}

impl Default for SplitMix64 {
    fn default() -> Self {
        Self(0x9e3779b97f4a7c15.into())
    }
}

impl From<u64> for SplitMix64 {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

#[derive(Debug)]
pub(crate) struct PerThreadRng<R = SmallRng> {
    rng: R,
    current_seed: SplitMix64,
}

impl<S> Default for PerThreadRng<S>
where
    S: SeedableRng,
{
    #[inline]
    fn default() -> Self {
        Self {
            rng: S::seed_from_u64(Default::default()),
            current_seed: Default::default(),
        }
    }
}

impl<R> Deref for PerThreadRng<R> {
    type Target = R;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.rng
    }
}

impl<R> DerefMut for PerThreadRng<R> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.rng
    }
}

impl<R: SeedableRng + rand::Rng> PerThreadRng<R> {
    pub(crate) fn fork_thread(&self) -> Self {
        let my_seed = self.current_seed.next_u64();
        let my_rng = R::seed_from_u64(my_seed);

        Self {
            rng: my_rng,
            current_seed: my_seed.into(),
        }
    }
}

/// A random scheduler.
///
/// This scheduler does not promise a bound on rank error or delay.
#[derive(Debug)]
pub struct RandomAccess<R = SmallRng>(PhantomData<R>);

impl<S> Default for RandomAccess<S> {
    #[inline]
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[expect(unnameable_types)]
#[derive(Debug)]
pub struct RandomAccessGambler<R = SmallRng> {
    rng: PerThreadRng<R>,
}

impl<S> Default for RandomAccessGambler<S>
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
    type Gambler = RandomAccessGambler<S>;

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
        RandomAccessGambler {
            rng: gambler.rng.fork_thread(),
        }
    }

    #[inline]
    fn create_gambler(&self) -> Self::Gambler {
        Default::default()
    }
}

impl<S> Hooked for RandomAccessGambler<S> {
    type Stake = NoPad<InstrumentedState<()>>;
}
