use std::ops::{self, Deref, DerefMut, Index, IndexMut};
use std::{cmp, hash, slice};
use std::pin::Pin;

use crate::PinEach;

impl<'a, T> PinEach<&'a [T]>
{
    pub const fn new(pointer: &'a [T]) -> Self
    where
        T: Unpin, { Self { pointer } }

    pub const fn len(&self) -> usize { self.pointer.len() }

    pub fn first(&self) -> Option<Pin<&T>>
    {
        self.pointer.first().map(|value|
        {
            // SAFETY: PinEach enforces the vec's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    pub fn last(&self) -> Option<Pin<&T>>
    {
        self.pointer.last().map(|value|
        {
            // SAFETY: PinEach enforces the vec's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    pub const fn index(&self, index: usize) -> Pin<&'a T>
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { Pin::new_unchecked(&self.pointer[index]) }
    }

    pub fn slice<R>(&self, range: R) -> PinEach<&'a [T]>
    where
        [T]: ops::Index<R>,
        &'a <[T] as ops::Index<R>>::Output: Into<&'a [T]> + 'a,
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index(range).into()) }
    }

    pub fn into_slice<R>(self, range: R) -> PinEach<&'a [T]>
    where
        [T]: ops::Index<R>,
        &'a <[T] as ops::Index<R>>::Output: Into<&'a [T]> + 'a,
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index(range).into()) }
    }

    pub fn iter_pin(&self) -> IterPin<'_, T>
    {
        IterPin { inner: self.pointer.iter() }
    }
}

impl<'a, T> PinEach<&'a mut [T]>
{
    pub const fn new(pointer: &'a mut [T]) -> Self
    where
        T: Unpin, { Self { pointer } }

    pub const fn len(&self) -> usize { self.pointer.len() }

    pub fn first(&self) -> Option<Pin<&T>>
    {
        self.pointer.first().map(|value|
        {
            // SAFETY: PinEach enforces the vec's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    pub fn first_mut(&mut self) -> Option<Pin<&mut T>>
    {
        self.pointer.first_mut().map(|value|
        {
            // SAFETY: PinEach enforces the vec's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    pub fn last(&self) -> Option<Pin<&T>>
    {
        self.pointer.last().map(|value|
        {
            // SAFETY: PinEach enforces the vec's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    pub fn last_mut(&mut self) -> Option<Pin<&mut T>>
    {
        self.pointer.last_mut().map(|value|
        {
            // SAFETY: PinEach enforces the vec's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    pub const fn index(&self, index: usize) -> Pin<&T>
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { Pin::new_unchecked(&self.pointer[index]) }
    }

    pub const fn index_mut(&mut self, index: usize) -> Pin<&mut T>
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { Pin::new_unchecked(&mut self.pointer[index]) }
    }

    pub fn slice<'s, R>(&'s self, range: R) -> PinEach<&'s [T]>
    where
        [T]: ops::Index<R>,
        &'s <[T] as ops::Index<R>>::Output: Into<&'s [T]> + 's,
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index(range).into()) }
    }

    pub fn slice_mut<'s, R>(&'s mut self, range: R) -> PinEach<&'s mut [T]>
    where
        [T]: ops::IndexMut<R>,
        &'s mut <[T] as ops::Index<R>>::Output: Into<&'s mut [T]> + 's,
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index_mut(range).into()) }
    }

    pub fn into_slice<R>(self, range: R) -> PinEach<&'a [T]>
    where
        [T]: ops::Index<R>,
        &'a <[T] as ops::Index<R>>::Output: Into<&'a [T]> + 'a,
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index(range).into()) }
    }

    pub fn into_slice_mut<R>(self, range: R) -> PinEach<&'a mut [T]>
    where
        [T]: ops::IndexMut<R>,
        &'a mut <[T] as ops::Index<R>>::Output: Into<&'a mut [T]> + 'a,
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index_mut(range).into()) }
    }

    pub fn iter_pin(&self) -> IterPin<'_, T>
    {
        IterPin { inner: self.pointer.iter() }
    }

    pub fn iter_pin_mut(&mut self) -> IterMutPin<'_, T>
    {
        IterMutPin { inner: self.pointer.iter_mut() }
    }
}

impl<'a, T: Unpin> From<Pin<&'a [T]>> for PinEach<&'a [T]>
{
    fn from(value: Pin<&'a [T]>) -> Self
    {
        Self::new(Pin::into_inner(value))
    }
}

impl<'a, T: Unpin> From<Pin<&'a mut [T]>> for PinEach<&'a mut [T]>
{
    fn from(value: Pin<&'a mut [T]>) -> Self
    {
        Self::new(Pin::into_inner(value))
    }
}

impl<T: Unpin, U: Unpin> PartialEq<PinEach<&[U]>> for PinEach<&[T]>
where
    [T]: PartialEq<[U]>,
{
    fn eq(&self, other: &PinEach<&[U]>) -> bool
    {
        <[T]>::eq(self, other)
    }

    fn ne(&self, other: &PinEach<&[U]>) -> bool
    {
        <[T]>::ne(self, other)
    }
}

impl<T: Unpin + Eq> Eq for PinEach<&[T]> { }

impl<T: Unpin, U: Unpin> PartialOrd<PinEach<&[U]>> for PinEach<&[T]>
where
    [T]: PartialOrd<[U]>,
{
    fn partial_cmp(&self, other: &PinEach<&[U]>) -> Option<cmp::Ordering>
    {
        <[T]>::partial_cmp(self, other)
    }

    fn lt(&self, other: &PinEach<&[U]>) -> bool
    {
        <[T]>::lt(self, other)
    }

    fn le(&self, other: &PinEach<&[U]>) -> bool
    {
        <[T]>::le(self, other)
    }

    fn gt(&self, other: &PinEach<&[U]>) -> bool
    {
        <[T]>::gt(self, other)
    }

    fn ge(&self, other: &PinEach<&[U]>) -> bool
    {
        <[T]>::ge(self, other)
    }
}

impl<T: Unpin + Ord> Ord for PinEach<&[T]>
{
    fn cmp(&self, other: &Self) -> cmp::Ordering
    {
        <[T]>::cmp(self, other)
    }
}

impl<T: Unpin + hash::Hash> hash::Hash for PinEach<&[T]>
{
    fn hash<H: hash::Hasher>(&self, state: &mut H)
    {
        <[T]>::hash(self, state);
    }
}

impl<T> Deref for PinEach<&[T]>
{
    type Target = [T];

    fn deref(&self) -> &Self::Target { self.pointer }
}

impl<T> Deref for PinEach<&mut [T]>
{
    type Target = [T];

    fn deref(&self) -> &Self::Target { self.pointer }
}

mod helper
{
    /// Helper that prevents downstream crates from implementing `DerefMut` for `PinEach`.
    pub struct PinEachHelper<Ptr>
    {
        pointer: Ptr,
    }

    pub trait PinEachDerefMutHelper
    {
        type Target: ?Sized;

        fn deref_mut(&mut self) -> &mut Self::Target;
    }

    impl<Ptr: super::DerefMut> PinEachDerefMutHelper for PinEachHelper<Ptr>
    where
        Ptr::Target: Unpin,
    {
        type Target = Ptr::Target;

        #[inline(always)]
        fn deref_mut(&mut self) -> &mut Ptr::Target
        {
            &mut self.pointer
        }
    }
}

impl<'a, T: Unpin> DerefMut for PinEach<&'a mut [T]>
where
    helper::PinEachHelper<&'a mut [T]>: helper::PinEachDerefMutHelper<Target = Self::Target>,
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // SAFETY: Pin and PinHelper have the same layout, so this is equivalent to
        // `&mut self.pointer` which is safe because `Target: Unpin`.
        helper::PinEachDerefMutHelper::deref_mut(unsafe {
            &mut *(self as *mut PinEach<&mut [T]> as *mut helper::PinEachHelper<&mut [T]>)
        })
    }
}

#[derive(Clone, Debug)]
pub struct IterPin<'a, T>
{
    pub(super) inner: slice::Iter<'a, T>,
}

impl<'a, T> Iterator for IterPin<'a, T>
{
    type Item = Pin<&'a T>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.inner.next().map(|value|
        {
            // SAFETY: PinEach enforces the slice's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

impl<'a, T> ExactSizeIterator for IterPin<'a, T>
{
    fn len(&self) -> usize { self.inner.len() }
}

#[derive(Debug)]
pub struct IterMutPin<'a, T>
{
    pub(super) inner: slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMutPin<'a, T>
{
    type Item = Pin<&'a mut T>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.inner.next().map(|value|
        {
            // SAFETY: PinEach enforces the slice's elements to not move.
            unsafe { Pin::new_unchecked(value) }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

impl<'a, T> ExactSizeIterator for IterMutPin<'a, T>
{
    fn len(&self) -> usize { self.inner.len() }
}