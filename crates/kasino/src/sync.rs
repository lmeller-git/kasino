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

#[cfg(test)]
pub(crate) use mutex_impls::*;

#[cfg(test)]
mod mutex_impls {

    #[cfg(all(not(loom), feature = "std"))]
    pub(crate) use mutex::*;
    #[cfg(all(not(loom), not(shuttle), not(feature = "std")))]
    pub(crate) use spin::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

    #[cfg(all(not(loom), not(shuttle), feature = "std"))]
    mod mutex {
        pub(crate) use std::sync::MutexGuard;

        #[derive(Debug, Default)]
        /// wraps std::sync::Mutex
        pub(crate) struct Mutex<T>(std::sync::Mutex<T>);

        impl<T> Mutex<T> {
            #[expect(dead_code)]
            /// Constructs a new Mutex
            pub(crate) const fn new(t: T) -> Self {
                Self(std::sync::Mutex::new(t))
            }

            /// locks the Mutex. This calls unwrap() on the internal Mutex, panicking on poison.
            pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
                self.0.lock().unwrap()
            }

            /// tries to lock the mutex
            #[expect(dead_code)]
            pub(crate) fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
                self.0.try_lock().ok()
            }
        }
    }

    #[cfg(loom)]
    pub(crate) use mutex::*;

    #[cfg(loom)]
    mod mutex {
        use core::ops::{Deref, DerefMut};

        pub(crate) use loom::sync::{Arc, MutexGuard};

        #[derive(Debug, Default)]
        /// wraps a loom:::sync::Mutext
        pub(crate) struct Mutex<T>(loom::sync::Mutex<T>);

        impl<T> Mutex<T> {
            /// constructs a new Mutex
            pub(crate) fn new(t: T) -> Self {
                Self(loom::sync::Mutex::new(t))
            }

            /// locks the mutex. unwraps poison
            pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
                self.0.lock().unwrap()
            }

            /// tries to lock the mutex
            #[allow(dead_code)]
            pub(crate) fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
                self.0.try_lock().ok()
            }
        }
    }

    #[cfg(shuttle)]
    pub(crate) use mutex::*;

    #[cfg(shuttle)]
    mod mutex {
        use core::ops::{Deref, DerefMut};

        pub(crate) use shuttle::sync::{Arc, MutexGuard};

        #[derive(Debug, Default)]
        /// wraps a shuttle::sync::mutex
        pub(crate) struct Mutex<T>(shuttle::sync::Mutex<T>);

        impl<T> Mutex<T> {
            #[expect(dead_code)]
            /// constructs a new mutex
            pub(crate) const fn new(t: T) -> Self {
                Self(shuttle::sync::Mutex::new(t))
            }

            /// locks the mutex. unwrapsp poison
            pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
                self.0.lock().unwrap()
            }

            /// tries to lock the mutex
            #[expect(dead_code)]
            pub(crate) fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
                self.0.try_lock().ok()
            }
        }
    }
}
