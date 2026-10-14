use std::ops::{self, Deref, DerefMut, Index, IndexMut};
use std::{cmp, hash};
use std::pin::Pin;

use crate::PinEach;
use crate::slice::{IterMutPin, IterPin};

impl<T> PinEach<Vec<T>>
{
    pub const fn new(pointer: Vec<T>) -> Self { Self { pointer } }

    pub const fn len(&self) -> usize { self.pointer.len() }

    pub const fn capacity(&self) -> usize { self.pointer.capacity() }

    pub fn into_inner(pin: PinEach<Vec<T>>) -> Vec<T>
    where
        T: Unpin, { pin.pointer }

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

    pub fn push(&mut self, value: T) -> Result<(), T>
    {
        self.push_mut(value).map(drop)
    }

    pub fn push_mut(&mut self, value: T) -> Result<Pin<&mut T>, T>
    {
        if self.len() == self.capacity() { return Err(value) };

        // SAFETY: PinEach enforces the vec's elements to not move.
        Ok(unsafe { Pin::new_unchecked(self.pointer.push_mut(value)) })
    }

    pub fn pop(&mut self) -> bool
    {
        self.pointer.pop().is_some()
    }

    pub fn pop_with(&mut self, f: impl FnOnce(Pin<&mut T>)) -> bool
    {
        self.last_mut().map(f);
        self.pop()
    }

    pub fn clear(&mut self)
    {
        self.pointer.clear();
    }

    pub fn clear_and_unpin(mut self) -> Vec<T>
    {
        self.clear();
        self.pointer
    }

    pub fn index(&self, index: usize) -> Pin<&T>
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { Pin::new_unchecked(&self.pointer[index]) }
    }

    pub fn index_mut(&mut self, index: usize) -> Pin<&mut T>
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { Pin::new_unchecked(&mut self.pointer[index]) }
    }

    pub fn slice<'a, R>(&'a self, range: R) -> PinEach<&'a [T]>
    where
        Vec<T>: ops::Index<R>,
        &'a <Vec<T> as ops::Index<R>>::Output: Into<&'a [T]> + 'a,
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index(range).into()) }
    }

    pub fn slice_mut<'a, R>(&'a mut self, range: R) -> PinEach<&'a mut [T]>
    where
        Vec<T>: ops::IndexMut<R>,
        &'a mut <Vec<T> as ops::Index<R>>::Output: Into<&'a mut [T]> + 'a,
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.index_mut(range).into()) }
    }

    pub const fn as_ref(&self) -> PinEach<&[T]>
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.as_slice()) }
    }

    pub const fn as_mut(&mut self) -> PinEach<&mut [T]>
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer.as_mut_slice()) }
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

impl<T> From<Vec<T>> for PinEach<Vec<T>>
{
    fn from(value: Vec<T>) -> Self
    {
        Self::new(value)
    }
}

impl<T> From<Pin<Vec<T>>> for PinEach<Vec<T>>
{
    fn from(value: Pin<Vec<T>>) -> Self
    {
        // SAFETY: PinEach enforces the vec's elements to not move.
        unsafe { Self::new(Pin::into_inner_unchecked(value)) }
    }
}

impl<T: Unpin> From<PinEach<Vec<T>>> for Vec<T>
{
    fn from(value: PinEach<Vec<T>>) -> Self { PinEach::into_inner(value) }
}

impl<T> Default for PinEach<Vec<T>>
{
    fn default() -> Self { Self::new(Vec::default()) }
}

impl<T: Unpin, U: Unpin> PartialEq<PinEach<Vec<U>>> for PinEach<Vec<T>>
where
    [T]: PartialEq<[U]>,
{
    fn eq(&self, other: &PinEach<Vec<U>>) -> bool
    {
        <[T]>::eq(self, other)
    }

    fn ne(&self, other: &PinEach<Vec<U>>) -> bool
    {
        <[T]>::ne(self, other)
    }
}

impl<T: Unpin + Eq> Eq for PinEach<Vec<T>> { }

impl<T: Unpin, U: Unpin> PartialOrd<PinEach<Vec<U>>> for PinEach<Vec<T>>
where
    [T]: PartialOrd<[U]>,
{
    fn partial_cmp(&self, other: &PinEach<Vec<U>>) -> Option<cmp::Ordering>
    {
        <[T]>::partial_cmp(self, other)
    }

    fn lt(&self, other: &PinEach<Vec<U>>) -> bool
    {
        <[T]>::lt(self, other)
    }

    fn le(&self, other: &PinEach<Vec<U>>) -> bool
    {
        <[T]>::le(self, other)
    }

    fn gt(&self, other: &PinEach<Vec<U>>) -> bool
    {
        <[T]>::gt(self, other)
    }

    fn ge(&self, other: &PinEach<Vec<U>>) -> bool
    {
        <[T]>::ge(self, other)
    }
}

impl<T: Unpin + Ord> Ord for PinEach<Vec<T>>
{
    fn cmp(&self, other: &Self) -> cmp::Ordering
    {
        <[T]>::cmp(self, other)
    }
}

impl<T: Unpin + hash::Hash> hash::Hash for PinEach<Vec<T>>
{
    fn hash<H: hash::Hasher>(&self, state: &mut H)
    {
        <[T]>::hash(self, state);
    }
}

impl<T> Deref for PinEach<Vec<T>>
{
    type Target = [T];

    fn deref(&self) -> &Self::Target { &self.pointer }
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

impl<T: Unpin> DerefMut for PinEach<Vec<T>>
where
    helper::PinEachHelper<Vec<T>>: helper::PinEachDerefMutHelper<Target = Self::Target>,
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // SAFETY: Pin and PinHelper have the same layout, so this is equivalent to
        // `&mut self.pointer` which is safe because `Target: Unpin`.
        helper::PinEachDerefMutHelper::deref_mut(unsafe {
            &mut *(self as *mut PinEach<Vec<T>> as *mut helper::PinEachHelper<Vec<T>>)
        })
    }
}

impl<T> FromIterator<T> for PinEach<Vec<T>>
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self
    {
        Self::new(Vec::from_iter(iter))
    }
}

impl<'a, T> IntoIterator for &'a PinEach<Vec<T>>
{
    type Item = Pin<&'a T>;

    type IntoIter = IterPin<'a, T>;

    fn into_iter(self) -> Self::IntoIter { self.iter_pin() }
}

impl<'a, T> IntoIterator for &'a mut PinEach<Vec<T>>
{
    type Item = Pin<&'a mut T>;

    type IntoIter = IterMutPin<'a, T>;

    fn into_iter(self) -> Self::IntoIter { self.iter_pin_mut() }
}