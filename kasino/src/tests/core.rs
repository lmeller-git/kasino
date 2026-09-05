use crate::{
    InlineBandit,
    strategy::{DCBO, DoubleCollectPoll},
    tests::test_library::{
        LockedDeque,
        force_push,
        len,
        len_empty_full,
        linearizable,
        mpmc,
        mpmc_ring_buffer,
        mpsc,
        smoke,
        smoke_long,
        spsc,
    },
};

#[cfg(feature = "alloc")]
mod boxed {
    use super::*;
    use crate::BoxedBandit;

    #[test]
    fn smoke_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        smoke(q.buy_in());
    }

    #[test]
    fn smoke_long_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        smoke_long(q.buy_in());
    }

    #[test]
    fn force_push_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        force_push(q.buy_in());
    }

    #[test]
    fn len_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        len(q.buy_in());
    }

    #[test]
    fn len_empty_full_impl() {
        let q: BoxedBandit<LockedDeque<()>, DCBO, 1> = BoxedBandit::new(2);
        len_empty_full(q.buy_in());
    }

    #[test]
    fn mpmc_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        mpmc(q.buy_in());
    }

    #[test]
    fn mpmc_ring_buffer_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        mpmc_ring_buffer(q.buy_in());
    }

    #[test]
    fn mpsc_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        mpsc(q.buy_in());
    }

    #[test]
    fn spsc_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DCBO> = BoxedBandit::new(2);
        spsc(q.buy_in());
    }

    #[test]
    fn linearizable_impl() {
        let q: BoxedBandit<LockedDeque<u32>, DoubleCollectPoll<DCBO>> = BoxedBandit::new(2);
        linearizable(q.buy_in());
    }
}

mod dcbo {
    use super::*;
    #[test]
    fn smoke_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        smoke(q.buy_in());
    }

    #[test]
    fn smoke_long_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        smoke_long(q.buy_in());
    }

    #[test]
    fn force_push_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        force_push(q.buy_in());
    }

    #[test]
    fn len_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        len(q.buy_in());
    }

    #[test]
    fn len_empty_full_impl() {
        let q: InlineBandit<LockedDeque<()>, DCBO, 2, 1> = InlineBandit::new();
        len_empty_full(q.buy_in());
    }

    #[test]
    fn mpmc_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        mpmc(q.buy_in());
    }

    #[test]
    fn mpmc_ring_buffer_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        mpmc_ring_buffer(q.buy_in());
    }

    #[test]
    fn mpsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        mpsc(q.buy_in());
    }

    #[test]
    fn spsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
        spsc(q.buy_in());
    }

    #[test]
    fn linearizable_impl() {
        let q: InlineBandit<LockedDeque<u32>, DoubleCollectPoll<DCBO>, 2> = InlineBandit::new();
        linearizable(q.buy_in());
    }
}

mod dra {
    use super::*;
    use crate::strategy::DRA;

    #[test]
    fn smoke_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        smoke(q.buy_in());
    }

    #[test]
    fn smoke_long_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        smoke_long(q.buy_in());
    }

    #[test]
    fn force_push_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        force_push(q.buy_in());
    }

    #[test]
    fn len_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        len(q.buy_in());
    }

    #[test]
    fn len_empty_full_impl() {
        let q: InlineBandit<LockedDeque<()>, DRA, 2, 1> = InlineBandit::new();
        len_empty_full(q.buy_in());
    }

    #[test]
    fn mpmc_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        mpmc(q.buy_in());
    }

    #[test]
    fn mpmc_ring_buffer_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        mpmc_ring_buffer(q.buy_in());
    }

    #[test]
    fn mpsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        mpsc(q.buy_in());
    }

    #[test]
    fn spsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, DRA, 2> = InlineBandit::new();
        spsc(q.buy_in());
    }

    #[test]
    fn linearizable_impl() {
        let q: InlineBandit<LockedDeque<u32>, DoubleCollectPoll<DRA>, 2> = InlineBandit::new();
        linearizable(q.buy_in());
    }
}

mod random {
    use super::*;
    use crate::strategy::RandomAccess;

    #[test]
    fn smoke_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        smoke(q.buy_in());
    }

    #[test]
    fn smoke_long_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        smoke_long(q.buy_in());
    }

    #[test]
    fn force_push_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        force_push(q.buy_in());
    }

    #[test]
    fn len_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        len(q.buy_in());
    }

    #[test]
    fn len_empty_full_impl() {
        let q: InlineBandit<LockedDeque<()>, RandomAccess, 2, 1> = InlineBandit::new();
        len_empty_full(q.buy_in());
    }

    #[test]
    fn mpmc_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        mpmc(q.buy_in());
    }

    #[test]
    fn mpmc_ring_buffer_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        mpmc_ring_buffer(q.buy_in());
    }

    #[test]
    fn mpsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        mpsc(q.buy_in());
    }

    #[test]
    fn spsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, RandomAccess, 2> = InlineBandit::new();
        spsc(q.buy_in());
    }

    #[test]
    fn linearizable_impl() {
        let q: InlineBandit<LockedDeque<u32>, DoubleCollectPoll<RandomAccess>, 2> =
            InlineBandit::new();
        linearizable(q.buy_in());
    }
}

mod round_robin {
    use super::*;
    use crate::strategy::RoundRobin;

    #[test]
    fn smoke_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        smoke(q.buy_in());
    }

    #[test]
    fn smoke_long_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        smoke_long(q.buy_in());
    }

    #[test]
    fn force_push_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        force_push(q.buy_in());
    }

    #[test]
    fn len_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        len(q.buy_in());
    }

    #[test]
    fn len_empty_full_impl() {
        let q: InlineBandit<LockedDeque<()>, RoundRobin, 2, 1> = InlineBandit::new();
        len_empty_full(q.buy_in());
    }

    #[test]
    fn mpmc_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        mpmc(q.buy_in());
    }

    #[test]
    fn mpmc_ring_buffer_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        mpmc_ring_buffer(q.buy_in());
    }

    #[test]
    fn mpsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        mpsc(q.buy_in());
    }

    #[test]
    fn spsc_impl() {
        let q: InlineBandit<LockedDeque<u32>, RoundRobin, 2> = InlineBandit::new();
        spsc(q.buy_in());
    }

    #[test]
    fn linearizable_impl() {
        let q: InlineBandit<LockedDeque<u32>, DoubleCollectPoll<RoundRobin>, 2> =
            InlineBandit::new();
        linearizable(q.buy_in());
    }
}
