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
                First { ref mut de } =>
                {
                    let (used_bytes, value) = ready!(de.resume(&bytes[total_bytes..]));

                    total_bytes += used_bytes.get();
                    let total_bytes = NonZero::new(total_bytes).unwrap();

                    match value
                    {
                        Break(Reset) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Break(Reset)));
                        },
                        Continue(Ok(first)) =>
                        {
                            self.state = Second { first, de: <_>::default() };
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    };
                },
                Second { ref mut de, .. } =>
                {
                    let (used_bytes, value) = ready!(de.resume(&bytes[total_bytes..]));

                    total_bytes += used_bytes.get();
                    let total_bytes = NonZero::new(total_bytes).unwrap();

                    match value
                    {
                        Break(Reset) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Break(Reset)));
                        },
                        Continue(Ok(second)) =>
                        {
                            let Second { first, .. } = mem::replace(&mut self.state, Done)
                            else { unreachable!() };

                            return Poll::Ready((total_bytes, Continue(Ok((first, second)))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                Done =>
                {
                    panic!("cannot resume completed deserializer");
                },
            }
        }
    }
}

enum DeserializerTupleState<D0, D1, D2>
where
    D0: Deserializer,
    D1: Default + Deserializer,
    D2: Default + Deserializer,
{
    First { de: D0 },
    Second { first: D0::Output, de: D1 },
    Third { first: D0::Output, second: D1::Output, de: D2 },
    Done,
}

impl<D0, D1, D2> Clone for DeserializerTupleState<D0, D1, D2>
where
    D0: Clone + Deserializer,
    D0::Output: Clone,
    D1: Clone + Default + Deserializer,
    D1::Output: Clone,
    D2: Clone + Default + Deserializer,
{
    fn clone(&self) -> Self
    {
        match self
        {
            Self::First { de } => Self::First { de: de.clone() },
            Self::Second { first, de } => Self::Second { first: first.clone(), de: de.clone() },
            Self::Third { first, second, de } => Self::Third { first: first.clone(), second: second.clone(), de: de.clone() },
            Self::Done => Self::Done,
        }
    }
}

impl<D0, D1, D2> Default for DeserializerTupleState<D0, D1, D2>
where
    D0: Clone + Default + Deserializer,
    D1: Clone + Default + Deserializer,
    D2: Clone + Default + Deserializer,
{
    fn default() -> Self { Self::First { de: <_>::default() } }
}

impl<D0, D1, D2> Debug for DeserializerTupleState<D0, D1, D2>
where
    D0: Clone + Default + Debug + Deserializer,
    D0::Output: Debug,
    D1: Clone + Default + Debug + Deserializer,
    D1::Output: Debug,
    D2: Clone + Default + Debug + Deserializer,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::First { de } => f.debug_struct("First").field("de", de).finish(),
            Self::Second { first, de } => f.debug_struct("Second").field("first", first).field("de", de).finish(),
            Self::Third { first, second, de } => f.debug_struct("Third").field("first", first).field("second", second).field("de", de).finish(),
            Self::Done => write!(f, "Done"),
        }
    }
}

pub struct DeserializerTuple<D0, D1, D2, E>
where
    D0: Deserializer<Error: Into<E>>,
    D1: Default + Deserializer<Error: Into<E>>,
    D2: Default + Deserializer<Error: Into<E>>,
{
    state: DeserializerTupleState<D0, D1, D2>,
    _marker: PhantomData<fn() -> E>,
}

impl<D0, D1, D2, E> Clone for DeserializerTuple<D0, D1, D2, E>
where
    D0: Clone + Deserializer<Error: Into<E>>,
    D0::Output: Clone,
    D1: Clone + Default + Deserializer<Error: Into<E>>,
    D1::Output: Clone,
    D2: Clone + Default + Deserializer<Error: Into<E>>,
{
    fn clone(&self) -> Self
    {
        Self { state: self.state.clone(), _marker: PhantomData }
    }
}

impl<D0, D1, D2, E> Default for DeserializerTuple<D0, D1, D2, E>
where
    D0: Clone + Default + Deserializer<Error: Into<E>>,
    D1: Clone + Default + Deserializer<Error: Into<E>>,
    D2: Clone + Default + Deserializer<Error: Into<E>>,
{
    fn default() -> Self
    {
        Self { state: <_>::default(), _marker: PhantomData }
    }
}

impl<D0, D1, D2, E> Debug for DeserializerTuple<D0, D1, D2, E>
where
    D0: Clone + Default + Debug + Deserializer<Error: Into<E>>,
    D0::Output: Debug,
    D1: Clone + Default + Debug + Deserializer<Error: Into<E>>,
    D1::Output: Debug,
    D2: Clone + Default + Debug + Deserializer<Error: Into<E>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("DeserializerTuple").field("state", &self.state).finish()
    }
}

impl<D0, D1, D2, E> Deserializer for DeserializerTuple<D0, D1, D2, E>
where
    D0: Deserializer<Error: Into<E>>,
    D1: Default + Deserializer<Error: Into<E>>,
    D2: Default + Deserializer<Error: Into<E>>,
{
    type Output = (D0::Output, D1::Output, D2::Output);

    type Error = E;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerTupleState::*;

        let mut total_bytes = 0;

        loop
        {
            match self.state
            {
                First { ref mut de } =>
                {
                    let (used_bytes, value) = ready!(de.resume(&bytes[total_bytes..]));

                    total_bytes += used_bytes.get();
                    let total_bytes = NonZero::new(total_bytes).unwrap();

                    match value
                    {
                        Break(Reset) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Break(Reset)));
                        },
                        Continue(Ok(first)) =>
                        {
                            self.state = Second { first, de: <_>::default() };
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    };
                },
                Second { ref mut de, .. } =>
                {
                    let (used_bytes, value) = ready!(de.resume(&bytes[total_bytes..]));

                    total_bytes += used_bytes.get();
                    let total_bytes = NonZero::new(total_bytes).unwrap();

                    match value
                    {
                        Break(Reset) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Break(Reset)));
                        },
                        Continue(Ok(second)) =>
                        {
                            let Second { first, .. } = mem::replace(&mut self.state, Done)
                            else { unreachable!() };

                            self.state = Third { first, second, de: <_>::default() };
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    };
                },
                Third { ref mut de, .. } =>
                {
                    let (used_bytes, value) = ready!(de.resume(&bytes[total_bytes..]));

                    total_bytes += used_bytes.get();
                    let total_bytes = NonZero::new(total_bytes).unwrap();

                    match value
                    {
                        Break(Reset) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Break(Reset)));
                        },
                        Continue(Ok(third)) =>
                        {
                            let Third { first, second, .. } = mem::replace(&mut self.state, Done)
                            else { unreachable!() };

                            return Poll::Ready((total_bytes, Continue(Ok((first, second, third)))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    };
                },
                Done =>
                {
                    panic!("cannot resume completed deserializer");
                },
            }
        }
    }
}