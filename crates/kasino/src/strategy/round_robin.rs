use crate::{
    Collection,
    storage::StorageBackend,
    strategy::{Hooked, InstrumentedState, Strategy, padded::NoPadding, random::FastPRNG},
};

/// A round robin scheduler.
///
/// This scheduler does not promise a bound on rank error and delay.
#[derive(Default, Debug)]
pub struct RoundRobin;

#[expect(unnameable_types)]
#[derive(Default, Debug)]
pub struct RoundRobinGambler {
    cur: usize,
    fork_state: FastPRNG,
}

impl RoundRobinGambler {
    fn fetch_add(&mut self) -> usize {
        let n = self.cur;
        self.cur += 1;
        n
    }
}

impl<Q: Collection> Strategy<Q> for RoundRobin {
    type Gambler = RoundRobinGambler;

    #[inline]
    fn choose_offer_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        gambler.fetch_add() % state.len()
    }

    #[inline]
    fn choose_poll_arm(
        &self,
        state: &impl StorageBackend<<Self::Gambler as Hooked>::Stake>,
        gambler: &mut Self::Gambler,
    ) -> usize {
        gambler.fetch_add() % state.len()
    }

    #[inline]
    fn fork_gambler(&self, gambler: &Self::Gambler) -> Self::Gambler {
        // TODO this should be better distributed
        let next_cur = gambler.fork_state.next_word();
        RoundRobinGambler {
            cur: next_cur as usize,
            fork_state: next_cur.into(),
        }
    }

    #[inline]
    fn create_gambler(&self) -> Self::Gambler {
        RoundRobinGambler::default()
    }
}

impl Hooked for RoundRobinGambler {
    type RequestedPadding = NoPadding;
    type Stake = InstrumentedState<()>;
}
