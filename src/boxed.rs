use alloc::boxed::Box;
use core::ops::{Index, IndexMut};

use crate::{
    Collection,
    WithCapacity,
    construction::{Bandit, BanditHandle, DEFAULT_QUEUE_CAP},
    storage::StorageBackend,
    strategy::{Hooked, Strategy},
};

/// a dynamically stored slice
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BoxedStorage<T> {
    arr: Box<[T]>,
}

impl<T: Default> Default for BoxedStorage<T> {
    #[inline]
    fn default() -> Self {
        Self {
            arr: Default::default(),
        }
    }
}

impl<T> BoxedStorage<T> {
    #[inline]
    pub(crate) fn from_fn_and_size(f: impl Fn(usize) -> T, size: usize) -> Self {
        Self {
            arr: (0..size).map(f).collect(),
        }
    }
}

impl<T> StorageBackend<T> for BoxedStorage<T> {
    type Rebind<U> = BoxedStorage<U>;

    #[inline]
    fn len(&self) -> usize {
        self.arr.len()
    }

    #[inline]
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.arr.iter()
    }

    #[inline]
    fn map_to_buffer<K>(&self, f: impl Fn(usize) -> K) -> Self::Rebind<K> {
        BoxedStorage::from_fn_and_size(f, self.arr.len())
    }
}

impl<T> IntoIterator for BoxedStorage<T> {
    type IntoIter = alloc::vec::IntoIter<T>;
    type Item = T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.arr.into_iter()
    }
}

impl<T> Index<usize> for BoxedStorage<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &<BoxedStorage<T> as Index<usize>>::Output {
        &self.arr[index]
    }
}

impl<T> IndexMut<usize> for BoxedStorage<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut <BoxedStorage<T> as Index<usize>>::Output {
        self.arr.get_mut(index).unwrap()
    }
}

/// A handle to a [`BoxedBandit`].
#[expect(type_alias_bounds)]
pub type BoxedBanditHandle<
    'a,
    Q: Collection,
    S: Strategy<Q>,
    const SUB_CAP: usize = DEFAULT_QUEUE_CAP,
> = BanditHandle<'a, Q, S, BoxedStorage<Q>, BoxedStorage<<S::Gambler as Hooked>::Stake>, SUB_CAP>;

/// a subcollection container, which is stored dynamically
#[expect(type_alias_bounds)]
pub type BoxedBandit<Q: Collection, S: Strategy<Q>, const SUB_CAP: usize = DEFAULT_QUEUE_CAP> =
    Bandit<Q, S, BoxedStorage<Q>, BoxedStorage<<S::Gambler as Hooked>::Stake>, SUB_CAP>;

impl<Q, S, const SUB_CAP: usize> BoxedBandit<Q, S, SUB_CAP>
where
    Q: WithCapacity<SUB_CAP>,
    S: Strategy<Q> + Default,
    Q: Collection,
{
    /// constructs a new `BoxedLop`
    #[must_use]
    #[inline]
    pub fn new(n_cores: usize) -> Self {
        const {
            assert!(SUB_CAP > 0, "The capacity per arm should be > 0");
        }

        assert!(n_cores > 0, "The number of arms should be > 0");
        Self::new_with(
            BoxedStorage::from_fn_and_size(
                |_| <Q as WithCapacity<SUB_CAP>>::with_capacity(),
                n_cores,
            ),
            BoxedStorage::from_fn_and_size(|_| Default::default(), n_cores),
        )
    }
}
