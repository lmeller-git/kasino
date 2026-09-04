use core::marker::PhantomData;

use crate::{
    Collection,
    Signature,
    components::PushPopCollection,
    storage::StorageBackend,
    strategy::{Hooked, StorageView, Strategy, StrategyStakes, View, padded::PaddingRequest},
};

pub(crate) const DEFAULT_QUEUE_CAP: usize = 32;

/// A [`Bandit`] distributes accesses to the wrapped datastructure across N sub-collections.
/// Access is distribute according to the [`Strategy`] `S` over the sub-collections `B` using state `C`.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct Bandit<Q, S, B, C, const SUB_CAP: usize = DEFAULT_QUEUE_CAP>
where
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    strategy: S,
    sub_collections: B,
    collection_state: C,
    _p: PhantomData<Q>,
}

impl<Q, S, B, C, const SUB_CAP: usize> Bandit<Q, S, B, C, SUB_CAP>
where
    S: Default,
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    pub(crate) fn new_with(queues: B, states: C) -> Self {
        Self {
            strategy: S::default(),
            sub_collections: queues,
            collection_state: states,
            _p: PhantomData,
        }
    }
}

impl<Q, S, B, C, const SUB_CAP: usize> Bandit<Q, S, B, C, SUB_CAP>
where
    B: StorageBackend<Q>,
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    /// returns the number of sub collections
    #[inline]
    pub fn arm_count(&self) -> usize {
        self.sub_collections.len()
    }
}

impl<Q, S, B, C, const SUB_CAP: usize> Bandit<Q, S, B, C, SUB_CAP>
where
    S: Strategy<Q>,
    Q: Collection,
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    /// constructs a new handle to this container
    ///
    /// This method should only be called once per thread pool.
    /// Create more handles using [`BanditHandle::fork`].
    #[inline]
    pub fn buy_in(&self) -> BanditHandle<'_, Q, S, B, C, SUB_CAP> {
        BanditHandle {
            parent: self,
            gambler: self.strategy.create_gambler(),
        }
    }
}

impl<Q, S, B, C, const SUB_CAP: usize> Bandit<Q, S, B, C, SUB_CAP>
where
    B: StorageBackend<Q>,
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    /// Consumes this collection and returns an iterator over its arms.
    #[inline]
    pub fn into_arms(self) -> impl Iterator<Item = B::Item> {
        self.sub_collections.into_iter()
    }
}

impl<Q, S, B, C, const SUB_CAP: usize> Bandit<Q, S, B, C, SUB_CAP>
where
    Q: IntoIterator,
    B: IntoIterator<Item = Q>,
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    /// Consumes this collection and returns an iterator over all contained items.
    #[inline]
    pub fn into_items(self) -> impl Iterator<Item = Q::Item> {
        self.sub_collections
            .into_iter()
            .flat_map(|collection| collection.into_iter())
    }
}

/// An owned handle into the core bandit.
///
/// This handle provides access to the functionality of the wrapped [`Collection`].
#[must_use]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct BanditHandle<'a, Q, S, B, C, const SUB_CAP: usize = DEFAULT_QUEUE_CAP>
where
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<StrategyStakes<S, Q>>,
{
    parent: &'a Bandit<Q, S, B, C, SUB_CAP>,
    gambler: S::Gambler,
}

impl<'a, Q, S, B, C, const SUB_CAP: usize> BanditHandle<'a, Q, S, B, C, SUB_CAP>
where
    B: StorageBackend<Q>,
    Q: Collection,
    S: Strategy<Q>,
    C: StorageBackend<
        <<S::Gambler as Hooked>::RequestedPadding as PaddingRequest>::PaddingStrategy<
            <S::Gambler as Hooked>::Stake,
        >,
    >,
{
    /// Fork this handle into a new one
    #[inline]
    pub fn fork(&self) -> Self {
        Self {
            parent: self.parent,
            gambler: S::fork_gambler(&self.parent.strategy, &self.gambler),
        }
    }

    /// Make a call to [`Collection::offer`] to an arms as chosen by this handles gambler.
    ///
    /// If the call fails, [`Strategy::on_offer_fail`] may be called to ensure consistency across all arms.
    #[inline]
    pub fn offer<'b, 'c>(
        &'c mut self,
        item: <Q::OfferSignature as Signature>::Input<'b>,
    ) -> Result<
        <Q::OfferSignature as Signature>::Output<'b, 'c>,
        <Q::OfferSignature as Signature>::Error<'b, 'c>,
    > {
        Self::offer_internal(self.parent, &mut self.gambler, item).0
    }

    /// Makes a call to [`Collection::offer`] and returns the stake associated with the arm we pulled.
    ///
    /// If the call fails, [`Strategy::on_offer_fail`] may be called to ensure consistency across all arms.
    ///
    /// On failure returns the info associated with the arm originally pulled.
    #[inline]
    #[expect(clippy::type_complexity)]
    pub fn offer_with_info<'b, 'c>(
        &'c mut self,
        item: <Q::OfferSignature as Signature>::Input<'b>,
    ) -> (
        Result<
            <Q::OfferSignature as Signature>::Output<'b, 'c>,
            <Q::OfferSignature as Signature>::Error<'b, 'c>,
        >,
        &'c <S::Gambler as Hooked>::Stake,
    ) {
        let (res, idx) = Self::offer_internal(self.parent, &mut self.gambler, item);
        (res, self.parent.collection_state[idx].project())
    }

    /// Makes a call to [`Self::offer`] and returns the index associated with the arm we pulled.    #[inline]
    #[expect(clippy::type_complexity)]
    pub(crate) fn offer_internal<'b, 'c>(
        parent: &'c Bandit<Q, S, B, C, SUB_CAP>,
        gambler: &mut S::Gambler,
        item: <Q::OfferSignature as Signature>::Input<'b>,
    ) -> (
        Result<
            <Q::OfferSignature as Signature>::Output<'b, 'c>,
            <Q::OfferSignature as Signature>::Error<'b, 'c>,
        >,
        usize,
    ) {
        let i = parent
            .strategy
            .choose_offer_arm(&StorageView::new(&parent.collection_state), gambler);
        match parent.sub_collections[i].offer(item) {
            Ok(r) => {
                gambler.on_offer_succ(parent.collection_state[i].project());
                (Ok(r), i)
            }
            Err(e) => {
                gambler.on_offer_fail(parent.collection_state[i].project());
                let r = parent.strategy.on_offer_fail(
                    &StorageView::new(&parent.collection_state),
                    &parent.sub_collections,
                    e,
                );

                match r {
                    Ok((out, i)) => {
                        gambler.on_offer_succ(parent.collection_state[i].project());
                        (Ok(out), i)
                    }
                    Err(e) => (Err(e), i),
                }
            }
        }
    }

    /// Make a call to [`Collection::poll`] to an arm as chosen by this handles gambler.
    ///
    /// If the call fails, [`Strategy::on_poll_fail`] may be called to ensure consistency across all arms.
    #[inline]
    pub fn poll<'b, 'c>(
        &'c mut self,
        input: <Q::PollSignature as Signature>::Input<'b>,
    ) -> Result<
        <Q::PollSignature as Signature>::Output<'b, 'c>,
        <Q::PollSignature as Signature>::Error<'b, 'c>,
    > {
        Self::poll_internal(self.parent, &mut self.gambler, input).0
    }

    /// Makes a call to [`Collection::poll`] and returns the stake associated with the arm we pulled.
    ///
    /// If the call fails, [`Strategy::on_poll_fail`] may be called to ensure consistency across all arms.
    ///
    /// On failure returns the info associated with the arm originally pulled.
    #[expect(clippy::type_complexity)]
    #[inline]
    pub fn poll_with_info<'b, 'c>(
        &'c mut self,
        input: <Q::PollSignature as Signature>::Input<'b>,
    ) -> (
        Result<
            <Q::PollSignature as Signature>::Output<'b, 'c>,
            <Q::PollSignature as Signature>::Error<'b, 'c>,
        >,
        &'c <S::Gambler as Hooked>::Stake,
    ) {
        let (res, idx) = Self::poll_internal(self.parent, &mut self.gambler, input);
        (res, self.parent.collection_state[idx].project())
    }

    /// Makes a call to [`Self::poll`] and returns the index associated with the arm we pulled.
    #[expect(clippy::type_complexity)]
    pub(crate) fn poll_internal<'b, 'c>(
        parent: &'c Bandit<Q, S, B, C, SUB_CAP>,
        gambler: &mut S::Gambler,
        input: <Q::PollSignature as Signature>::Input<'b>,
    ) -> (
        Result<
            <Q::PollSignature as Signature>::Output<'b, 'c>,
            <Q::PollSignature as Signature>::Error<'b, 'c>,
        >,
        usize,
    ) {
        let i = parent
            .strategy
            .choose_poll_arm(&StorageView::new(&parent.collection_state), gambler);
        match parent.sub_collections[i].poll(input) {
            Ok(r) => {
                gambler.on_poll_succ(parent.collection_state[i].project());
                (Ok(r), i)
            }
            Err(e) => {
                gambler.on_poll_fail(parent.collection_state[i].project());
                let r = parent.strategy.on_poll_fail(
                    &StorageView::new(&parent.collection_state),
                    &parent.sub_collections,
                    input,
                );
                if let Some((r, state)) = r {
                    gambler.on_poll_succ(parent.collection_state[state].project());
                    (Ok(r), state)
                } else {
                    (Err(e), i)
                }
            }
        }
    }

    /// Returns an iterator over all stakes in all arms
    #[inline]
    pub fn state(&self) -> impl Iterator<Item = &StrategyStakes<S, Q>> {
        self.parent.collection_state.iter()
    }

    /// the total len of all arms
    #[inline]
    pub fn len(&self) -> usize {
        self.parent.sub_collections.iter().map(|q| q.len()).sum()
    }

    /// the total capacity of all arms
    #[inline]
    pub fn capacity(&self) -> usize {
        self.parent
            .sub_collections
            .iter()
            .map(|q| q.capacity())
            .sum()
    }

    /// are all arms empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, Q, S, B, C, const SUB_CAP: usize> BanditHandle<'a, Q, S, B, C, SUB_CAP>
where
    B: StorageBackend<Q>,
    S: Strategy<Q>,
    Q: Collection,
    C: StorageBackend<StrategyStakes<S, Q>>,
{
    /// returns the number of arms
    #[inline]
    pub fn arm_count(&self) -> usize {
        self.parent.arm_count()
    }
}

impl<'a, Q, S, B, C, const SUB_CAP: usize> BanditHandle<'a, Q, S, B, C, SUB_CAP>
where
    Q: PushPopCollection,
    S: Strategy<Q>,
    B: StorageBackend<Q>,
    C: StorageBackend<StrategyStakes<S, Q>>,
{
    /// Pushes an item to the collection.
    ///
    /// Returns the item on an erorr.
    ///
    /// This method is a convenience wrapper around [`Self::offer`].
    #[inline]
    pub fn push(&mut self, item: Q::Item) -> Result<(), Q::Item> {
        self.offer(item)
    }

    /// Attempts to pop an item from the collection.
    ///
    /// This method is a convenience wrapper around [`Self::poll`].
    #[inline]
    pub fn pop(&mut self) -> Option<Q::Item> {
        self.poll(()).ok()
    }
}
