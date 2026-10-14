use std::fmt;

mod slice;

mod vec;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PinEach<Ptr>
{
    pub(crate) pointer: Ptr,
}

impl<Ptr> PinEach<Ptr>
{
    pub const unsafe fn new_unchecked(pointer: Ptr) -> Self { Self { pointer } }

    pub unsafe fn into_inner_unchecked(pin: PinEach<Ptr>) -> Ptr
    {
        pin.pointer
    }
}

impl<'a, T: ?Sized> PinEach<&'a T>
{
    pub unsafe fn map_unchecked<U, F>(self, func: F) -> PinEach<&'a U>
    where
        U: ?Sized,
        F: FnOnce(&T) -> &U,
    {
        let pointer = &*self.pointer;
        let new_pointer = func(pointer);

        // SAFETY: the safety contract for `new_unchecked` must be
        // upheld by the caller.
        unsafe { PinEach::new_unchecked(new_pointer) }
    }

    #[must_use]
    pub const fn get_ref(self) -> &'a T { self.pointer }

    pub const fn as_ref(&self) -> PinEach<&'a T>
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer) }
    }
}

impl<'a, T: ?Sized> PinEach<&'a mut T>
{
    pub const fn into_ref(self) -> PinEach<&'a T>
    {
        PinEach { pointer: self.pointer }
    }

    #[must_use = "`self` will be dropped if the result is not used"]
    pub const fn get_mut(self) -> &'a mut T
    where
        T: Unpin,
    {
        self.pointer
    }

    #[must_use = "`self` will be dropped if the result is not used"]
    pub const unsafe fn get_unchecked_mut(self) -> &'a mut T
    {
        self.pointer
    }

    #[must_use = "`self` will be dropped if the result is not used"]
    pub unsafe fn map_unchecked_mut<U, F>(self, func: F) -> PinEach<&'a mut U>
    where
        U: ?Sized,
        F: FnOnce(&mut T) -> &mut U,
    {
        // SAFETY: the caller is responsible for not moving the
        // value out of this reference.
        let pointer = unsafe { PinEach::get_unchecked_mut(self) };
        let new_pointer = func(pointer);
        // SAFETY: as the value of `this` is guaranteed to not have
        // been moved out, this call to `new_unchecked` is safe.
        unsafe { PinEach::new_unchecked(new_pointer) }
    }

    pub const fn as_ref(&self) -> PinEach<&T>
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer) }
    }

    pub const fn as_mut(&mut self) -> PinEach<&mut T>
    {
        // SAFETY: PinEach enforces the slice's elements to not move.
        unsafe { PinEach::new_unchecked(self.pointer) }
    }
}

impl<Ptr: fmt::Debug> fmt::Debug for PinEach<Ptr>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(&self.pointer, f)
    }
}

impl<Ptr: fmt::Display> fmt::Display for PinEach<Ptr>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(&self.pointer, f)
    }
}

impl<Ptr: fmt::Pointer> fmt::Pointer for PinEach<Ptr>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Pointer::fmt(&self.pointer, f)
    }
}