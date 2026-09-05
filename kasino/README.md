[![Codecov](https://codecov.io/github/lmeller-git/kasino/coverage.svg?branch=main)](https://codecov.io/gh/lmeller-git/kasino)
![CI Test](https://github.com/lmeller-git/kasino/actions/workflows/test.yml/badge.svg?branch=main)
![Safety Test](https://github.com/lmeller-git/kasino/actions/workflows/safety.yml/badge.svg?branch=main)
![no_std Test](https://github.com/lmeller-git/kasino/actions/workflows/nostd.yml/badge.svg?branch=main)
[![Crates.io](https://img.shields.io/crates/v/kasino)](https://crates.io/crates/kasino)
[![Docs.rs](https://docs.rs/kasino/badge.svg)](https://docs.rs/kasino)

# Kasino

<!-- cargo-rdme start -->

`Kasino` is a framework for building high-contention concurrent data structures.

By distributing operations across multiple sub-collections using customizable strategies, `Kasino` reduces cache-line invalidation and thread contention.

`Kasino` provides out-of-the-box support for elastically relaxed concurrent queues, while also offering a framework to implement other sharded concurrent data structures.

Built-in strategies optimize for different tradeoffs between performance and relaxation bounds.

### Usage

```rust
use kasino::{InlineBandit, strategy::DCBO};

let bandit = InlineBandit::<MyQueue<i32>, DCBO, 8>::new();

let mut handle = bandit.buy_in();
let mut handle2 = handle.fork();

assert!(handle.offer(42).is_ok());
assert!(handle2.offer(10).is_ok());
assert!(handle.poll(()).is_ok());
```

### Property preservation

#### Progress Guarantees:

- **Lock Freedom**: if the wrapped collection is lock-free, [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html)'s are also lock-free.
- **Obstruction Freedom**: if the wrapped collection exposes obstruction-free methods, all corresponding operations on [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html)'s are also obstruction-free.

#### Ordering and Consistency Guarantees:

- **Relaxed Specification**: if the wrapped collection has some specification, [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html)'s relax that specification based on the chosen strategy.
- **Linearizability**: if the wrapped collection is linearizable, all operations on [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html)'s are also linearizable with respect to their relaxed specification.

#### Relaxation

The rank error and delay are in general unbounded. However, the rank error and delay of some strategies are bounded with high probability.
The exact bounds here are differing across different strategies.

For more information refer to the strategies documentation and the reference papers.

For an empirical analysis of the rank errors, refer to [relaxed-queue-simulations](https://github.com/lmeller-git/relaxed-queue-simulations).

### Performance

Sharding operations to multiple sub-collections incurs both memory cost, as well as additional overhead. Under low contention `Kasino` is slower than the raw data structure, with the decrease in performance depending strongly on chosen [`strategy::Strategy`](https://docs.rs/kasino/latest/kasino/strategy/trait.Strategy.html).

However, scheduling thread access across multiple sub-collections allows to reduce cache-line invalidation at high contention, improving performance as thread count increases.

### Limitations

- Currently an instantiated [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html) cannot be resized. Its capacity is fixed at construction time.
- The capacity of each sub-collection is fixed statically. The total capacity of a [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html) is constrained to a multiple of this.

### Advanced Usage

The interfaces for [`Collection`](https://docs.rs/kasino/latest/kasino/trait.Collection.html), [`strategy::Strategy`](https://docs.rs/kasino/latest/kasino/strategy/trait.Strategy.html) and [`Bandit`](https://docs.rs/kasino/latest/kasino/construction/struct.Bandit.html) are general enough to support the implementation of a large set of datastructures. For examples of this consult [`examples`](https://github.com/lmeller-git/kasino/tree/main/examples).

### Platform Support

All platforms supporting native atomic operations are supported.

The feature `atomic-fallback` may be used, if no native atomic operations are available.

### Feature Flags

- `std`: Enables `std` support.
- `instrumented`: Adds telemetry collection to strategies
- `atomic-fallback`:  Uses the `portable-atomic` fallback feature if native atomics are missing. It is discouraged to use this feature, as fallback atomics internally rely on locks.
- `default`: None

### Testing

Currently testing is based on:

- **Miri** - to validate pointer arithmetic and catch undefined behavior.
- **Loom and Shuttle** - to test for race conditions and non-blocking invariants.
- **ASan** - to check for memory corruption.

### References

- Performance, Scalability, and Semantics of Concurrent FIFO Queues, Kirsch et al.
- Balanced Allocations over Efficient Queues: A Fast Relaxed FIFO Queue, Geijer et al.

<!-- cargo-rdme end -->

