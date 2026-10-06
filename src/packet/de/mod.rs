
mod int;
pub use int::*;

mod pair;
pub use pair::*;

mod vec;
pub use vec::*;

use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::Poll;

use super::Reset;

use ControlFlow::*;

pub trait Deserializer
{
    type Output;

    type Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>;
}

impl<T: ?Sized + Deserializer> Deserializer for &mut T
{
    type Output = T::Output;

    type Error = T::Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        (**self).resume(bytes)
    }
}



#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InvalidEscape { pub byte: u8 }