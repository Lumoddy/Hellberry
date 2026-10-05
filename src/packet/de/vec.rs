use std::fmt::{self, Debug};
use std::mem;
use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::{Poll, ready};

use super::{Deserializer, DeserializerU16, InvalidEscape, Reset};

use ControlFlow::*;

enum DeserializerVecState<D>
where
    D: Default + Deserializer,
    InvalidEscape: Into<D::Error>,
{
    Len { de: DeserializerU16 },
    Element { buffer: Vec<D::Output>, len: u16, de: D },
    Done,
}

impl<D> Clone for DeserializerVecState<D>
where
    D: Clone + Default + Deserializer,
    D::Output: Clone,
    InvalidEscape: Into<D::Error>,
{
    fn clone(&self) -> Self
    {
        match self
        {
            Self::Len { de } => Self::Len { de: de.clone() },
            Self::Element { buffer, len, de } => Self::Element { buffer: buffer.clone(), len: len.clone(), de: de.clone() },
            Self::Done => Self::Done,
        }
    }
}

impl<D> Default for DeserializerVecState<D>
where
    D: Default + Deserializer,
    InvalidEscape: Into<D::Error>,
{
    fn default() -> Self { Self::Len { de: <_>::default() } }
}

impl<D> Debug for DeserializerVecState<D>
where
    D: Default + Debug + Deserializer,
    D::Output: Debug,
    InvalidEscape: Into<D::Error>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Len { de } => f.debug_struct("Len").field("de", de).finish(),
            Self::Element { buffer, len, de } => f.debug_struct("Element").field("buffer", buffer).field("len", len).field("de", de).finish(),
            Self::Done => write!(f, "Done"),
        }
    }
}

#[derive(Default)]
pub struct DeserializerVec<D>
where
    D: Default + Deserializer<Error: From<InvalidEscape>>,
{
    state: DeserializerVecState<D>,
}

impl<D> Clone for DeserializerVec<D>
where
    D: Clone + Default + Deserializer<Error: From<InvalidEscape>>,
    D::Output: Clone,
{
    fn clone(&self) -> Self { Self { state: self.state.clone() } }
}

impl<D> Debug for DeserializerVec<D>
where
    D: Default + Debug + Deserializer<Error: From<InvalidEscape>>,
    D::Output: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("DeserializerVec").field("state", &self.state).finish()
    }
}

impl<D> Deserializer for DeserializerVec<D>
where
    D: Default + Deserializer<Error: From<InvalidEscape>>,
{
    type Output = Vec<D::Output>;

    type Error = D::Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerVecState::*;

        let mut total_bytes = 0;

        loop
        {
            match self.state
            {
                Len { ref mut de } => match ready!(de.resume(&bytes[total_bytes..]))
                {
                    (used_bytes, Break(Reset)) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Break(Reset)));
                    },
                    (used_bytes, Continue(Ok(0))) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Continue(Ok(Vec::new()))));
                    },
                    (used_bytes, Continue(Ok(len))) =>
                    {
                        total_bytes += used_bytes.get();
                        self.state = Element
                        {
                            buffer: Vec::with_capacity(len as usize),
                            len,
                            de: <_>::default(),
                        };
                    },
                    (used_bytes, Continue(Err(error))) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Continue(Err(error.into()))));
                    },
                },
                Element { ref mut buffer, len, ref mut de } => match ready!(de.resume(&bytes[total_bytes..]))
                {
                    (used_bytes, Break(Reset)) =>
                    {
                        let used_bytes = NonZero::new(total_bytes + used_bytes.get()).unwrap();
                        self.state = Done;
                        return Poll::Ready((used_bytes, Break(Reset)));
                    },
                    (used_bytes, Continue(Ok(item))) =>
                    {
                        total_bytes += used_bytes.get();
                        buffer.push(item);

                        if buffer.len() == len as usize
                        {
                            let buffer = mem::take(buffer);
                            self.state = Done;
                            return Poll::Ready((used_bytes, Continue(Ok(buffer))));
                        }
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