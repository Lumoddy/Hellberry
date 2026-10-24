//! A module for working with shared data.

pub use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use std::fmt;
use std::sync::Arc;

use Ash::*;

/// An alloc-on-send smart pointer. 'Ash' stands for '<u>A</u>tomically
/// <u>SH</u>ared'.
///
/// References a resource until it needs to live for a longer lifetime than the
/// borrow specified, in which case the resource switches to a shared [`Arc`].
/// This can be useful for when some borrow only sometimes needs to be passed
/// between threads.
pub enum Ash<'a, T: ?Sized + 'a> {
    /// Borrowed data.
    Borrowed(&'a T),

    /// Shared data.
    Shared(Arc<T>),
}

impl<T: ?Sized> Ash<'_, T> {
    pub const fn is_borrowed(c: &Self) -> bool { matches!(c, Borrowed(_)) }

    pub const fn is_shared(c: &Self) -> bool { matches!(c, Shared(_)) }
}

impl<T: ?Sized> Clone for Ash<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(borrowed) => Self::Borrowed(borrowed),
            Self::Shared(shared) => Self::Shared(shared.clone()),
        }
    }
}

impl<'a, T: ?Sized> Borrow<T> for Ash<'a, T> {
    fn borrow(&self) -> &T { &**self }
}

impl<'a, T: ?Sized> Ash<'a, T>
where
    &'a T: Into<Arc<T>>,
{
    pub fn to_shared(&mut self) -> &mut Arc<T> {
        match *self {
            Borrowed(borrowed) => {
                *self = Shared(borrowed.into());
                match *self {
                    Borrowed(..) => unreachable!(),
                    Shared(ref mut shared) => shared,
                }
            }
            Shared(ref mut shared) => shared,
        }
    }

    pub fn into_shared(self) -> Arc<T> {
        match self {
            Borrowed(borrowed) => borrowed.into(),
            Shared(shared) => shared,
        }
    }

    pub fn into_static<'b>(self) -> Ash<'b, T> { Shared(self.into()) }
}

impl<'a, T: ?Sized> From<Ash<'a, T>> for Arc<T>
where
    &'a T: Into<Arc<T>>,
{
    fn from(value: Ash<'a, T>) -> Self { value.into_shared() }
}

impl<'a, T: ?Sized> From<&'a T> for Ash<'a, T> {
    fn from(value: &'a T) -> Self { Borrowed(value) }
}

impl<T: ?Sized> From<Arc<T>> for Ash<'_, T> {
    fn from(value: Arc<T>) -> Self { Shared(value) }
}

impl<T: Clone> From<Vec<T>> for Ash<'_, [T]> {
    fn from(value: Vec<T>) -> Self { Shared(value.as_slice().into()) }
}

impl From<String> for Ash<'_, str> {
    fn from(value: String) -> Self { Shared(value.as_str().into()) }
}

impl<T: ?Sized> Deref for Ash<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match *self {
            Borrowed(borrowed) => borrowed,
            Shared(ref shared) => &shared,
        }
    }
}

impl<T: ?Sized + Eq> Eq for Ash<'_, T> { }

impl<T: ?Sized + Ord> Ord for Ash<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: ?Sized + PartialEq<U>, U: ?Sized> PartialEq<Ash<'_, U>> for Ash<'_, T> {
    fn eq(&self, other: &Ash<'_, U>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl<T: ?Sized + PartialOrd<U>, U: ?Sized> PartialOrd<Ash<'_, U>> for Ash<'_, T> {
    fn partial_cmp(&self, other: &Ash<'_, U>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Ash<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Borrowed(ref b) => fmt::Debug::fmt(b, f),
            Shared(ref o) => fmt::Debug::fmt(o, f),
        }
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Ash<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Borrowed(ref b) => fmt::Display::fmt(b, f),
            Shared(ref o) => fmt::Display::fmt(o, f),
        }
    }
}

impl<T: Default> Default for Ash<'_, T> {
    fn default() -> Self { Shared(Arc::default()) }
}

impl<T: ?Sized + Hash> Hash for Ash<'_, T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) { Hash::hash(&**self, state) }
}

impl<T: ?Sized> AsRef<T> for Ash<'_, T>
{
    fn as_ref(&self) -> &T { self }
}
