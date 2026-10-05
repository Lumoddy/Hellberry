use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::mem;
use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::{Poll, ready};

use super::{Deserializer, Reset};

use ControlFlow::*;

enum DeserializerPairState<D0, D1>
where
    D0: Deserializer,
    D1: Default + Deserializer,
{
    First { de: D0 },
    Second { first: D0::Output, de: D1 },
    Done,
}

impl<D0, D1> Clone for DeserializerPairState<D0, D1>
where
    D0: Clone + Deserializer,
    D0::Output: Clone,
    D1: Clone + Default + Deserializer,
{
    fn clone(&self) -> Self
    {
        match self
        {
            Self::First { de } => Self::First { de: de.clone() },
            Self::Second { first, de } => Self::Second { first: first.clone(), de: de.clone() },
            Self::Done => Self::Done,
        }
    }
}

impl<D0, D1> Default for DeserializerPairState<D0, D1>
where
    D0: Clone + Default + Deserializer,
    D1: Clone + Default + Deserializer,
{
    fn default() -> Self { Self::First { de: <_>::default() } }
}

impl<D0, D1> Debug for DeserializerPairState<D0, D1>
where
    D0: Clone + Default + Debug + Deserializer,
    D0::Output: Debug,
    D1: Clone + Default + Debug + Deserializer,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::First { de } => f.debug_struct("First").field("de", de).finish(),
            Self::Second { first, de } => f.debug_struct("Second").field("first", first).field("de", de).finish(),
            Self::Done => write!(f, "Done"),
        }
    }
}

pub struct DeserializerPair<D0, D1, E>
where
    D0: Deserializer<Error: Into<E>>,
    D1: Default + Deserializer<Error: Into<E>>,
{
    state: DeserializerPairState<D0, D1>,
    _marker: PhantomData<fn() -> E>,
}

impl<D0, D1, E> Clone for DeserializerPair<D0, D1, E>
where
    D0: Clone + Deserializer<Error: Into<E>>,
    D0::Output: Clone,
    D1: Clone + Default + Deserializer<Error: Into<E>>,
{
    fn clone(&self) -> Self
    {
        Self { state: self.state.clone(), _marker: PhantomData }
    }
}

impl<D0, D1, E> Default for DeserializerPair<D0, D1, E>
where
    D0: Clone + Default + Deserializer<Error: Into<E>>,
    D1: Clone + Default + Deserializer<Error: Into<E>>,
{
    fn default() -> Self
    {
        Self { state: <_>::default(), _marker: PhantomData }
    }
}

impl<D0, D1, E> Debug for DeserializerPair<D0, D1, E>
where
    D0: Clone + Default + Debug + Deserializer<Error: Into<E>>,
    D0::Output: Debug,
    D1: Clone + Default + Debug + Deserializer<Error: Into<E>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("DeserializerPair").field("state", &self.state).finish()
    }
}

impl<D0, D1, E> Deserializer for DeserializerPair<D0, D1, E>
where
    D0: Deserializer<Error: Into<E>>,
    D1: Default + Deserializer<Error: Into<E>>,
{
    type Output = (D0::Output, D1::Output);

    type Error = E;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerPairState::*;

        let mut total_bytes = 0;

        loop
        {
            match self.state
            {
                First { ref mut de } => match ready!(de.resume(&bytes[total_bytes..]))
                {
                    (used_bytes, Break(Reset)) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Break(Reset)));
                    },
                    (used_bytes, Continue(Ok(first))) =>
                    {
                        total_bytes += used_bytes.get();
                        self.state = Second { first, de: <_>::default() };
                    },
                    (used_bytes, Continue(Err(error))) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Continue(Err(error.into()))));
                    },
                },
                Second { ref mut de, .. } => match ready!(de.resume(&bytes[total_bytes..]))
                {
                    (used_bytes, Break(Reset)) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Break(Reset)));
                    },
                    (used_bytes, Continue(Ok(second))) =>
                    {
                        let Second { first, .. } = mem::replace(&mut self.state, Done)
                        else { unreachable!() };

                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        return Poll::Ready((used_bytes, Continue(Ok((first, second)))));
                    },
                    (used_bytes, Continue(Err(error))) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Continue(Err(error.into()))));
                    },
                },
                Done =>
                {
                    panic!("cannot resume completed deserializer");
                },
            }
        }
    }
}