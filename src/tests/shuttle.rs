use crate::{
    InlineBandit,
    strategy::{DCBO, DoubleCollectPoll},
    tests::test_library::{LockedDeque, linearizable, mpmc, mpmc_ring_buffer, mpsc, spsc},
};

const RETRIES: usize = 1000;
const DEPTH: usize = 10;

#[test]
fn spsc_impl() {
    shuttle::check_pct(
        || {
            let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
            spsc(q.buy_in());
        },
        RETRIES,
        DEPTH,
    );
}

#[test]
fn mpsc_impl() {
    shuttle::check_pct(
        || {
            let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
            mpsc(q.buy_in());
        },
        RETRIES,
        DEPTH,
    );
}

#[test]
fn mpmc_impl() {
    shuttle::check_pct(
        || {
            let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
            mpmc(q.buy_in());
        },
        RETRIES,
        DEPTH,
    );
}

#[test]
fn mpmc_ring_buffer_impl() {
    shuttle::check_pct(
        || {
            let q: InlineBandit<LockedDeque<u32>, DCBO, 2> = InlineBandit::new();
            mpmc_ring_buffer(q.buy_in());
        },
        RETRIES,
        DEPTH,
    );
}

#[test]
fn linearizable_impl() {
    shuttle::check_pct(
        || {
            let q: InlineBandit<LockedDeque<u32>, DoubleCollectPoll<DCBO>, 2> = InlineBandit::new();
            linearizable(q.buy_in());
        },
        RETRIES,
        DEPTH,
    )
}
