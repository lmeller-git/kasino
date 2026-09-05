use core::marker::PhantomData;

use rand::{RngExt, SeedableRng, rngs::SmallRng};

use crate::{
    Collection,
    storage::StorageBackend,
    strategy::{EDCount, Hooked, Strategy, padded::RequiresPadding, random::PerThreadRng},
    sync::atomic::Ordering,
};

/// A DRA scheduler.
///
/// Rank error and delay of this strategy are bounded with high probability.
///
/// Reference: Performance, Scalability, and Semantics of Concurrent FIFO Queues, Kirsch et al.
#[derive(Debug)]
pub struct DRA<const CHOOSE: usize = 2, R = SmallRng>(PhantomData<R>);

impl<R, const CHOOSE: usize> Default for DRA<CHOOSE, R> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

#[expect(unnameable_types)]
#[derive(Debug)]
pub struct DRAGambler<const CHOOSE: usize = 2, R = SmallRng>(PerThreadRng<R>);

impl<R: SeedableRng, const CHOOSE: usize> Default for DRAGambler<CHOOSE, R> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<R: RngExt + SeedableRng, Q: Collection, const CHOOSE: usize> Strategy<Q> for DRA<CHOOSE, R> {
    type Gambler = DRAGambler<CHOOSE, R>;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        (0..CHOOSE)
            .map(|_| gambler.0.random_range(..state.len()))
            .min_by_key(|&i| {
                state[i]
                    .offer_count
                    .load(Ordering::Relaxed)
                    .saturating_sub(state[i].poll_count())
            })
            .unwrap()
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        (0..CHOOSE)
            .map(|_| gambler.0.random_range(..state.len()))
            .max_by_key(|&i| {
                state[i]
                    .poll_count
                    .load(Ordering::Relaxed)
                    .saturating_sub(state[i].offer_count())
            })
            .unwrap()
    }

    #[inline]
    fn fork_gambler(&self, gambler: &Self::Gambler) -> Self::Gambler {
        DRAGambler(gambler.0.fork_thread())
    }

    #[inline]
    fn create_gambler(&self) -> Self::Gambler {
        const {
            assert!(
                CHOOSE > 0,
                "The number of arms to be chosen over should be > 0"
            );
        }
        Default::default()
    }
}

impl<R, const CHOOSE: usize> Hooked for DRAGambler<CHOOSE, R> {
    type RequestedPadding = RequiresPadding;
    type Stake = EDCount;
}
