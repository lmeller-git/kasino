#![expect(unused_imports)]
#![expect(clippy::disallowed_modules)]

#[cfg(all(not(loom), not(shuttle)))]
pub(crate) use core_::*;
#[cfg(loom)]
pub(crate) use loom_::*;
#[cfg(shuttle)]
pub(crate) use shuttle_::*;

#[cfg(shuttle)]
mod shuttle_ {
    pub(crate) use shuttle::{
        hint,
        sync::{Arc, Weak, atomic},
        thread,
    };
}

#[cfg(loom)]
mod loom_ {
    // no Weak in loom
    pub(crate) use std::sync::Weak;

    pub(crate) use loom::{
        hint,
        sync::{Arc, atomic},
        thread,
    };
}

#[cfg(all(not(loom), not(shuttle)))]
mod core_ {
    #[cfg(any(feature = "alloc", test))]
    pub(crate) use alloc::sync::{Arc, Weak};
    pub(crate) use core::hint;
    #[cfg(any(feature = "std", test))]
    pub(crate) use std::thread;

    pub(crate) use portable_atomic as atomic;
}
