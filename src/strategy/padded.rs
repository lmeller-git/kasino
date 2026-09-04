//! Functionality for specifying memory layout of various types.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crossbeam_utils::CachePadded;

use crate::strategy::{Hook, collect::View};

/// Specifies which kind of padding this type requests at the storage level.
#[expect(private_bounds)]
pub trait PaddingRequest: Sealed {
    /// The type of paddigng requested.
    type PaddingStrategy<T>: for<'a> View<'a, T>;
}

impl<T> Hook for CachePadded<T>
where
    T: Hook,
{
    #[inline]
    fn on_offer_succ(&self) {
        T::on_offer_succ(self);
    }

    #[inline]
    fn on_offer_fail(&self) {
        T::on_offer_fail(self);
    }

    #[inline]
    fn on_poll_succ(&self) {
        T::on_poll_succ(self);
    }

    #[inline]
    fn on_poll_fail(&self) {
        T::on_poll_fail(self);
    }
}

/// a transparent wrapper around a T
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoPad<T>(T);

impl<'a, T> View<'a, T> for NoPad<T> {
    #[inline]
    fn project(&'a self) -> &'a T {
        &self.0
    }
}

impl<T> Hook for NoPad<T>
where
    T: Hook,
{
    #[inline]
    fn on_offer_succ(&self) {
        T::on_offer_succ(self.project());
    }

    #[inline]
    fn on_offer_fail(&self) {
        T::on_offer_fail(self.project());
    }

    #[inline]
    fn on_poll_succ(&self) {
        T::on_poll_succ(self.project());
    }

    #[inline]
    fn on_poll_fail(&self) {
        T::on_poll_fail(self.project());
    }
}

impl<T> AsRef<T> for NoPad<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.project()
    }
}

impl<T> AsMut<T> for NoPad<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A transparent wrapper around a T
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeView<T>(T);

impl<T, U> AsRef<T> for TypeView<U>
where
    U: Deref<Target = T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T, U> AsMut<T> for TypeView<U>
where
    U: DerefMut<Target = T>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<'a, T, K> View<'a, T> for TypeView<K>
where
    K: Deref<Target = T>,
{
    #[inline]
    fn project(&'a self) -> &'a T {
        &self.0
    }
}

/// Request for padding in the form of [`CachePadded`].
pub struct RequiresPadding;

impl Sealed for RequiresPadding {}

impl PaddingRequest for RequiresPadding {
    type PaddingStrategy<T> = TypeView<CachePadded<T>>;
}

/// This type requires no padding.
pub struct NoPadding;

impl Sealed for NoPadding {}

impl PaddingRequest for NoPadding {
    type PaddingStrategy<T> = NoPad<T>;
}

impl Eval for NoPadding {
    type Output = NoPadding;
}

impl Eval for RequiresPadding {
    type Output = RequiresPadding;
}

impl Truthiness for NoPadding {
    type IsTruthy = False;
}

impl Truthiness for RequiresPadding {
    type IsTruthy = True;
}

use truthiness::*;
pub(crate) mod truthiness {
    #![expect(unnameable_types)]

    use super::*;

    pub trait Truthiness {
        type IsTruthy;
    }

    pub struct True;
    pub struct False;

    impl Eval for True {
        type Output = True;
    }

    impl Eval for False {
        type Output = False;
    }

    pub trait Eval {
        type Output;
    }

    pub trait TypeOr<R> {
        type Output;
    }

    impl TypeOr<True> for True {
        type Output = <True as Eval>::Output;
    }

    impl TypeOr<False> for True {
        type Output = <True as Eval>::Output;
    }

    impl TypeOr<True> for False {
        type Output = <True as Eval>::Output;
    }

    impl TypeOr<False> for False {
        type Output = <False as Eval>::Output;
    }

    pub trait Select<A, B> {
        type Output;
    }

    impl<A: Eval, B> Select<A, B> for True {
        type Output = <A as Eval>::Output;
    }

    impl<A, B: Eval> Select<A, B> for False {
        type Output = <B as Eval>::Output;
    }

    pub struct Or<A, B>(PhantomData<(A, B)>);

    impl<A, B> Truthiness for Or<A, B>
    where
        A: Truthiness,
        B: Truthiness,
        A::IsTruthy: TypeOr<B::IsTruthy>,
    {
        type IsTruthy = <A::IsTruthy as TypeOr<B::IsTruthy>>::Output;
    }

    impl<A, B> Eval for Or<A, B>
    where
        A: Truthiness,
        B: Truthiness,
        <A as Truthiness>::IsTruthy: Select<A, B>,
    {
        type Output = <<A as Truthiness>::IsTruthy as Select<A, B>>::Output;
    }

    #[expect(type_alias_bounds)]
    pub(crate) type Evaluate<T: Eval> = <T as Eval>::Output;
}

pub(crate) trait Sealed {}
