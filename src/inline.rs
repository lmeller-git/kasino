use core::ops::{Index, IndexMut};

use crate::{
    Collection,
    WithCapacity,
    construction::{Bandit, BanditHandle, DEFAULT_QUEUE_CAP},
    storage::StorageBackend,
    strategy::{Hooked, Strategy},
};

/// an array
#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord, Hash, Copy)]
pub struct InlineStorage<T, const N: usize> {
    arr: [T; N],
}

impl<T: Default, const N: usize> Default for InlineStorage<T, N> {
    #[inline]
    fn default() -> Self {
        Self {
            arr: core::array::from_fn(|_| Default::default()),
        }
    }
}

impl<T, const N: usize> StorageBackend<T> for InlineStorage<T, N> {
    type Rebind<U> = InlineStorage<U, N>;

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
    fn map_to_buffer<U>(&self, f: impl Fn(usize) -> U) -> Self::Rebind<U> {
        InlineStorage {
            arr: core::array::from_fn(f),
        }
    }
}

impl<T, const N: usize> IntoIterator for InlineStorage<T, N> {
    type IntoIter = core::array::IntoIter<T, N>;
    type Item = T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.arr.into_iter()
    }
}

impl<T, const N: usize> InlineStorage<T, N> {
    #[inline]
    fn from_fn(f: impl Fn(usize) -> T) -> Self {
        InlineStorage {
            arr: core::array::from_fn(f),
        }
    }
}

impl<T, const N: usize> Index<usize> for InlineStorage<T, N> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &<InlineStorage<T, N> as Index<usize>>::Output {
        &self.arr[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for InlineStorage<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut <InlineStorage<T, N> as Index<usize>>::Output {
        &mut self.arr[index]
    }
}

/// A handle to an [`InlineBandit`].
#[expect(type_alias_bounds)]
pub type InlineBanditHandle<
    'a,
    Q: Collection,
    S: Strategy<Q>,
    const N: usize,
    const SUB_CAP: usize = DEFAULT_QUEUE_CAP,
> = BanditHandle<
    'a,
    Q,
    S,
    InlineStorage<Q, N>,
    InlineStorage<<S::Gambler as Hooked>::Stake, N>,
    SUB_CAP,
>;

/// A container of `N` sub collections that is stored inline.
#[expect(type_alias_bounds)]
pub type InlineBandit<
    Q: Collection,
    S: Strategy<Q>,
    const N: usize,
    const SUB_CAP: usize = DEFAULT_QUEUE_CAP,
> = Bandit<Q, S, InlineStorage<Q, N>, InlineStorage<<S::Gambler as Hooked>::Stake, N>, SUB_CAP>;

impl<Q: Collection, S, const N: usize, const SUB_CAP: usize> InlineBandit<Q, S, N, SUB_CAP>
where
    Q: WithCapacity<SUB_CAP>,
    S: Strategy<Q> + Default,
{
    /// constructs a new `InlineBandit`
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        const {
            assert!(
                N > 0 && SUB_CAP > 0,
                "The number of arms and the capacity per arm should be > 0"
            );
        }
        Bandit::new_with(
            InlineStorage::from_fn(|_| <Q as WithCapacity<SUB_CAP>>::with_capacity()),
            InlineStorage::from_fn(|_| Default::default()),
        )
    }
}

impl<Q, S, const N: usize, const SUB_CAP: usize> Default for InlineBandit<Q, S, N, SUB_CAP>
where
    Q: WithCapacity<SUB_CAP>,
    S: Strategy<Q> + Default,
    Q: Collection,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
