use std::fmt::Debug;
use std::mem;
use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::Poll;

use crate::packet::{ESCAPE, START};

use super::{Deserializer, InvalidEscape, Reset};

use ControlFlow::*;

#[derive(Clone, Copy, Debug, Default)]
enum DeserializerU8State
{
    #[default]
    Ready,
    Escaped,
    Done,
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerU8
{
    state: DeserializerU8State,
}

impl Deserializer for DeserializerU8
{
    type Output = u8;

    type Error = InvalidEscape;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerU8State::*;

        self.state = match mem::replace(&mut self.state, Done)
        {
            Ready => match bytes
            {
                [ESCAPE, ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Ok(ESCAPE))));
                },
                [ESCAPE, START, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Break(Reset)));
                },
                [ESCAPE, byte, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Err(InvalidEscape { byte: *byte }))));
                },
                [ESCAPE] =>
                {
                    Escaped
                },
                [byte, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Ok(*byte))));
                },
                [] =>
                {
                    Ready
                },
            },
            Escaped => match bytes
            {
                [ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Ok(ESCAPE))));
                },
                [START, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Break(Reset)));
                },
                [byte, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Err(InvalidEscape { byte: *byte }))));
                },
                [] =>
                {
                    Escaped
                },
            },
            Done =>
            {
                panic!("cannot resume completed deserializer");
            },
        };

        Poll::Pending
    }
}

#[derive(Clone, Copy, Debug)]
enum DeserializerU16State
{
    Ready0 { bytes: [u8; 0] },
    Escaped0 { bytes: [u8; 0] },
    Ready1 { bytes: [u8; 1] },
    Escaped1 { bytes: [u8; 1] },
    Done,
}

impl Default for DeserializerU16State
{
    fn default() -> Self
    {
        DeserializerU16State::Ready0 { bytes: [] }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerU16
{
    state: DeserializerU16State,
}

impl Deserializer for DeserializerU16
{
    type Output = u16;

    type Error = InvalidEscape;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerU16State::*;

        let source = bytes;
        self.state = match mem::replace(&mut self.state, Done)
        {
            Ready0 { bytes } => match source
            {
                [ESCAPE, ESCAPE, ESCAPE, ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(4).unwrap(), Continue(Ok(u16::from_be_bytes([ESCAPE, ESCAPE])))));
                },
                [ESCAPE, ESCAPE, ESCAPE, START, ..] =>
                {
                    return Poll::Ready((NonZero::new(4).unwrap(), Break(Reset)));
                },
                [ESCAPE, ESCAPE, ESCAPE, byte, ..] =>
                {
                    return Poll::Ready((NonZero::new(4).unwrap(), Continue(Err(InvalidEscape { byte: *byte }))));
                },
                [ESCAPE, ESCAPE, ESCAPE] =>
                {
                    Escaped1 { bytes: [ESCAPE] }
                },
                [ESCAPE, ESCAPE, byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Continue(Err(InvalidEscape { byte: *byte1 }))));
                },
                [ESCAPE, ESCAPE] =>
                {
                    Ready1 { bytes: [ESCAPE] }
                },
                [ESCAPE, START, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Break(Reset)));
                },
                [ESCAPE, byte, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Err(InvalidEscape { byte: *byte }))));
                },
                [byte0, ESCAPE, ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Continue(Ok(u16::from_be_bytes([*byte0, ESCAPE])))));
                },
                [_, ESCAPE, START, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Break(Reset)));
                },
                [_, ESCAPE, byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Continue(Err(InvalidEscape { byte: *byte1 }))));
                },
                [byte0, ESCAPE] =>
                {
                    Escaped1 { bytes: [*byte0] }
                },
                [byte0, byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Ok(u16::from_be_bytes([*byte0, *byte1])))));
                },
                [byte0] =>
                {
                    Ready1 { bytes: [*byte0] }
                },
                [] =>
                {
                    Ready0 { bytes }
                },
            },
            Escaped0 { bytes } => match source
            {
                [ESCAPE, ESCAPE, ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Continue(Ok(u16::from_be_bytes([ESCAPE, ESCAPE])))));
                },
                [ESCAPE, ESCAPE, START, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Break(Reset)));
                },
                [ESCAPE, ESCAPE, byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(3).unwrap(), Continue(Err(InvalidEscape { byte: *byte1 }))));
                },
                [ESCAPE, ESCAPE] =>
                {
                    Escaped1 { bytes: [ESCAPE] }
                },
                [ESCAPE, byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Err(InvalidEscape { byte: *byte1 }))));
                },
                [ESCAPE] =>
                {
                    Ready1 { bytes: [ESCAPE] }
                },
                [START, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Break(Reset)));
                },
                [byte, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Err(InvalidEscape { byte: *byte }))));
                },
                [] =>
                {
                    Escaped0 { bytes }
                },
            },
            Ready1 { bytes } => match source
            {
                [ESCAPE, ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Ok(u16::from_be_bytes([bytes[0], ESCAPE])))));
                },
                [ESCAPE, START, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Break(Reset)));
                },
                [ESCAPE, byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(2).unwrap(), Continue(Err(InvalidEscape { byte: *byte1 }))));
                },
                [ESCAPE] =>
                {
                    Escaped1 { bytes }
                },
                [byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Ok(u16::from_be_bytes([bytes[0], *byte1])))));
                },
                [] =>
                {
                    Ready1 { bytes }
                },
            },
            Escaped1 { bytes } => match source
            {
                [ESCAPE, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Ok(u16::from_be_bytes([bytes[0], ESCAPE])))));
                },
                [START, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Break(Reset)));
                },
                [byte1, ..] =>
                {
                    return Poll::Ready((NonZero::new(1).unwrap(), Continue(Err(InvalidEscape { byte: *byte1 }))));
                },
                [] =>
                {
                    Escaped1 { bytes }
                },
            },
            Done =>
            {
                panic!("cannot resume completed deserializer");
            },
        };

        Poll::Pending
    }
}