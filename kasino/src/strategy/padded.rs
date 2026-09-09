//! Functionality for specifying memory layout of various types.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crossbeam_utils::CachePadded;

use crate::strategy::{
    Hook,
    padded::type_eval::{Eval, False, True, Truthiness},
};

// This is sealed because `or` evaluation of the padding requests means that we cannot differentiate between two truthy or two falsy requests.
// Thus if arbitrary reqeuests were implemented, then we would have to think about how to propagate them upward, which may require a more sophisticated scheme than or based propagation.
/// Specifies which kind of padding this type requests at the storage level.
#[expect(private_bounds)]
pub trait PaddingRequest: Sealed {
    /// The type of paddigng requested.
    type PaddingStrategy<T>: Deref<Target = T>;
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

/// A transparent wrapper around a T
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeView<T>(T);

impl<T> Deref for TypeView<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TypeView<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Hook> Hook for TypeView<T> {
    #[inline]
    fn on_offer_succ(&self) {
        self.0.on_offer_succ();
    }

    #[inline]
    fn on_offer_fail(&self) {
        self.0.on_offer_fail();
    }

    #[inline]
    fn on_poll_succ(&self) {
        self.0.on_poll_succ();
    }

    #[inline]
    fn on_poll_fail(&self) {
        self.0.on_poll_fail();
    }
}

/// Request for padding in the form of [`CachePadded`].
pub struct RequiresPadding;

impl Sealed for RequiresPadding {}

impl PaddingRequest for RequiresPadding {
    type PaddingStrategy<T> = CachePadded<T>;
}

/// This type requires no padding.
pub struct NoPadding;

impl Sealed for NoPadding {}

impl PaddingRequest for NoPadding {
    type PaddingStrategy<T> = TypeView<T>;
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

pub mod type_eval {
    //! Machinery to evaluate type level `or` expression.

    use super::*;

    /// Truthiness of a type
    pub trait Truthiness {
        /// Describes if this type is truthy
        type IsTruthy;
    }

    /// This type is truthy
    pub struct True;
    /// This type is falsy
    pub struct False;

    impl Eval for True {
        type Output = True;
    }

    impl Eval for False {
        type Output = False;
    }

    /// A typed expression
    pub trait Eval {
        /// The resulting type of evaluating this typed expression
        type Output;
    }

    /// Evaluates an `or` between two types
    pub trait TypeOr<R> {
        /// The output of the or evaluation
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

    /// Forwards the output of the evaluation of one of the two subtypes
    pub trait Select<A, B> {
        /// The output of this evaluation
        type Output;
    }

    impl<A: Eval, B> Select<A, B> for True {
        type Output = <A as Eval>::Output;
    }

    impl<A, B: Eval> Select<A, B> for False {
        type Output = <B as Eval>::Output;
    }

    /// Evaluates to the type of the truthy subtype
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
    /// Evaluates the type of a typed expression
    pub type Evaluate<T: Eval> = <T as Eval>::Output;
}

pub(crate) trait Sealed {}
