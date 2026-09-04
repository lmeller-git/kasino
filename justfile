default:
    @just --list

test-full crate="kasino": test
    cargo +nightly miri test -p {{crate}} --locked --all-features
    LOOM_MAX_PREEMPTIONS=2 RUSTFLAGS="--cfg loom" cargo test -p {{crate}} --locked --lib --all-features --release
    RUSTFLAGS="--cfg shuttle" cargo test -p {{crate}} --locked --lib --all-features

lint-rs:
    cargo +nightly fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

check-rs:
    cargo +nightly docs-rs -p kasino
    cargo hack --workspace --feature-powerset check
    cargo semver-checks --workspace --all-features

test:
    cargo test --workspace --locked --all-features --all-targets
    cargo test --workspace --locked --all-features --doc
    cargo test --workspace --locked --no-default-features --all-targets
    cargo test --workspace --locked --no-default-features --doc

lint: lint-rs

check: check-rs

bench crate="kasino":
    cargo bench -p {{crate}} --all-features
