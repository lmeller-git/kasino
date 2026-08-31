//! `Kasino` is a framework for building high-contention concurrent data structures.
//!
//! By distributing operations across multiple sub-collections using customizable strategies, `Kasino` reduces cache-line invalidation and thread contention.
//!
//! `Kasino` provides out-of-the-box support for elastically relaxed concurrent queues, while also offering a framework to implement other sharded concurrent data structures.
//!
//! Built-in strategies optimize for different tradeoffs between performance and relaxation bounds.
//!
//! ## Usage
//!
//! ```rust
//! # use kasino::{Collection, WithCapacity, Signature, components::{TryPushSignature, PopSignature}};
//! # use std::sync::Mutex;
//! # use std::collections::VecDeque;
//! # use std::marker::PhantomData;
//! # struct QueuePushSignature<T>(PhantomData<T>);
//! # struct MyQueue<T> { deque: Mutex<VecDeque<T>>, cap: usize }
//! # impl<T> Collection for MyQueue<T> {
//! #     type PollSignature = PopSignature<T>;
//! #     type OfferSignature = TryPushSignature<T>;
//! #     fn offer<'input, 'arm>(
//! #         &'arm self,
//! #         item: <Self::OfferSignature as Signature>::Input<'input>,
//! #     ) -> Result<
//! #         <Self::OfferSignature as Signature>::Output<'input, 'arm>,
//! #         <Self::OfferSignature as Signature>::Error<'input, 'arm>,
//! #     > {
//! #         let mut g = self.deque.lock().unwrap();
//! #         if g.len() >= self.cap { Err(item) } else { g.push_back(item); Ok(()) }
//! #     }
//! #     fn poll<'input, 'arm>(
//! #         &'arm self,
//! #         input: <Self::PollSignature as Signature>::Input<'input>,
//! #     ) -> Result<
//! #         <Self::PollSignature as Signature>::Output<'input, 'arm>,
//! #         <Self::PollSignature as Signature>::Error<'input, 'arm>,
//! #     > {
//! #         self.deque.lock().unwrap().pop_front().ok_or(())
//! #     }
//! #     fn len(&self) -> usize { self.deque.lock().unwrap().len() }
//! #     fn capacity(&self) -> usize { self.cap }
//! # }
//! # impl<T, const N: usize> WithCapacity<N> for MyQueue<T> {
//! #     fn with_capacity() -> Self { Self { deque: Mutex::new(VecDeque::with_capacity(N)), cap: N } }
//! # }
//! use kasino::{InlineBandit, strategy::DCBO};
//!
//! let bandit = InlineBandit::<MyQueue<i32>, DCBO, 8>::new();
//!
//! let mut handle = bandit.buy_in();
//! let mut handle2 = handle.fork();
//!
//! assert!(handle.offer(42).is_ok());
//! assert!(handle2.offer(10).is_ok());
//! assert!(handle.poll(()).is_ok());
//! ```
//!
//! ## Property preservation
//!
//! ### Progress Guarantees:
//!
//! - **Lock Freedom**: if the wrapped collection is lock-free, [`Bandit`]'s are also lock-free.
//! - **Obstruction Freedom**: if the wrapped collection exposes obstruction-free methods, all corresponding operations on [`Bandit`]'s are also obstruction-free.
//!
//! ### Ordering and Consistency Guarantees:
//!
//! - **Relaxed Specification**: if the wrapped collection has some specification, [`Bandit`]'s relax that specification based on the chosen strategy.
//! - **Linearizability**: if the wrapped collection is linearizable, all operations on [`Bandit`]'s are also linearizable with respect to their relaxed specification.
//!
//! ### Relaxation
//!
//! The rank error and delay are in general unbounded. However, the rank error and delay of some strategies are bounded with high probability.
//! The exact bounds here are differing across different strategies.
//!
//! For more information refer to the strategies documentation and the reference papers.
//!
//! For an empirical analysis of the rank errors, refer to [relaxed-queue-simulations](https://github.com/lmeller-git/relaxed-queue-simulations).
//!
//! ## Performance
//!
//! Sharding operations to multiple sub-collections incurs both memory cost, as well as additional overhead. Under low contention `Kasino` is slower than the raw data structure, with the decrease in performance depending strongly on chosen [`strategy::Strategy`].
//!
//! However, scheduling thread access across multiple sub-collections allows to reduce cache-line invalidation at high contention, improving performance as thread count increases.
//!
//! ## Limitations
//!
//! - Currently an instantiated [`Bandit`] cannot be resized. Its capacity is fixed at construction time.
//! - The capacity of each sub-collection is fixed statically. The total capacity of a [`Bandit`] is constrained to a multiple of this.
//!
//! ## Advanced Usage
//!
//! The interfaces for [`Collection`], [`strategy::Strategy`] and [`Bandit`] are general enough to support the implementation of a large set of datastructures. For examples of this consult [`examples`](https://github.com/lmeller-git/kasino/tree/main/examples).
//!
//! ## Platform Support
//!
//! All platforms supporting native atomic operations are supported.
//!
//! The feature `atomic-fallback` may be used, if no native atomic operations are available.
//!
//! ## Feature Flags
//!
//! - `std`: Enables `std` support.
//! - `instrumented`: Adds telemetry collection to strategies
//! - `atomic-fallback`:  Uses the `portable-atomic` fallback feature if native atomics are missing. It is discouraged to use this feature, as fallback atomics internally rely on locks.
//! - `default`: None
//!
//! ## Testing
//!
//! Currently testing is based on:
//!
//! - **Miri** - to validate pointer arithmetic and catch undefined behavior.
//! - **Loom and Shuttle** - to test for race conditions and non-blocking invariants.
//! - **ASan** - to check for memory corruption.
//!
//! ## References
//!
//! - Performance, Scalability, and Semantics of Concurrent FIFO Queues, Kirsch et al.
//! - Balanced Allocations over Efficient Queues: A Fast Relaxed FIFO Queue, Geijer et al.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![deny(missing_docs)]
#![deny(clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)]
#![warn(unsafe_op_in_unsafe_fn)]

#[cfg(any(feature = "std", test))]
extern crate std;

#[allow(unused_extern_crates)]
#[cfg(any(feature = "alloc", test))]
extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed;
pub mod components;
mod construction;
mod inline;
pub mod storage;
pub mod strategy;
mod sync;

#[cfg(test)]
mod tests;

pub mod prelude {
    //! Useful types that should suffice for standard usage.

    #[cfg(feature = "alloc")]
    pub use crate::{BoxedBandit, BoxedBanditHandle};
    pub use crate::{
        Collection,
        InlineBandit,
        InlineBanditHandle,
        Signature,
        strategy::{DCBO, DRA, DoubleCollect, NoCollect, RandomAccess, RoundRobin},
    };
}

#[cfg(feature = "alloc")]
pub use boxed::*;
pub use construction::{Bandit, BanditHandle};
pub use inline::*;

/// Description about the signature of a failable method
pub trait Signature {
    /// The input
    type Input<'a>;
    /// The successful output
    type Output<'input, 'arm>
    where
        Self: 'arm;
    /// the error
    type Error<'input, 'arm>
    where
        Self: 'arm;
}

/// The interface for a generic data structure.
pub trait Collection
where
    for<'a> <Self::PollSignature as Signature>::Input<'a>: Copy,
{
    /// The signature of the [`Self::offer`] method
    type OfferSignature: Signature;
    /// The signature of the [`Self::poll`] method
    type PollSignature: Signature;

    /// Attempt to act on this collection.
    fn offer<'input, 'arm>(
        &'arm self,
        item: <Self::OfferSignature as Signature>::Input<'input>,
    ) -> Result<
        <Self::OfferSignature as Signature>::Output<'input, 'arm>,
        <Self::OfferSignature as Signature>::Error<'input, 'arm>,
    >;
    /// Attempt to act on this collection in a constrained way.
    ///
    /// The input is `Copy`.
    ///
    /// `Self::poll` may be called multiple times per `Bandit::poll` invocation.
    fn poll<'input, 'arm>(
        &'arm self,
        input: <Self::PollSignature as Signature>::Input<'input>,
    ) -> Result<
        <Self::PollSignature as Signature>::Output<'input, 'arm>,
        <Self::PollSignature as Signature>::Error<'input, 'arm>,
    >;
    /// The length of the collection
    fn len(&self) -> usize;
    /// The capacity of the collection
    fn capacity(&self) -> usize;
    /// Is the collection empty?
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A collection that may be created with a static initial capacity N
pub trait WithCapacity<const N: usize> {
    /// Constructs a new Collection with capacity N
    fn with_capacity() -> Self;
}
