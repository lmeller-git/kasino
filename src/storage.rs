//! Storage abstractions for different kinds of storages

use core::ops::Index;

/// a generic storage
pub trait StorageBackend<T>: Index<usize, Output = T> + IntoIterator {
    /// A generic storage backend, which this storagebackend knows how to construct
    type Rebind<U>: StorageBackend<U>;

    /// the current length of the storage
    fn len(&self) -> usize;
    /// returns an iterator over all items in the storage
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a;
    /// Builds a Self::Rebind filled with f(idx)
    fn map_to_buffer<U>(&self, f: impl Fn(usize) -> U) -> Self::Rebind<U>;
    /// is the storage empty?
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
