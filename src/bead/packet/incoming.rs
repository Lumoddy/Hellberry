use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::{Poll, ready};

use crate::bead::{Config, DeserializerConfig, DeserializerPinMode, PinMode};
use crate::packet::{Deserializer, DeserializerPair, DeserializerU8, DeserializerU16, InvalidEscape, Reset};

use ControlFlow::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet
{
    Pong,
    Config(Config),
    GetPinPowerResponse { pin: u8, power: u16 },
    GetPinModeResponse { pin: u8, mode: PinMode },
    SetPinPowerResponse { pin: u8, power: u16 },
    SetPinModeResponse { pin: u8, mode: PinMode },
    PinListen { pin: u8, power: u16 },
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Error
{
    InvalidPinMode { byte: u8 },
    InvalidPinId { byte: u8 },
    InvalidPacketId { byte: u8 },
    InvalidWriteToInput { pin: u8, power: u16 },
    InvalidUnsupportedMode { pin: u8, mode: PinMode },
    InvalidEscape { byte: u8 },
    IncomingInvalidPinMode { byte: u8 },
    IncomingInvalidPacketId { byte: u8 },
    IncomingInvalidEscape { byte: u8 },
    IncomingInvalidUTF8,
}

impl From<InvalidEscape> for Error
{
    fn from(value: InvalidEscape) -> Self
    {
        Self::IncomingInvalidEscape { byte: value.byte }
    }
}

#[derive(Clone, Debug)]
enum DeserializerPacketState
{
    PacketId { de: DeserializerU8 },
    Config { de: DeserializerConfig },
    GetPinPowerResponse { de: DeserializerPair<DeserializerU8, DeserializerU16, Error> },
    GetPinModeResponse { de: DeserializerPair<DeserializerU8, DeserializerPinMode, Error> },
    SetPinPowerResponse { de: DeserializerPair<DeserializerU8, DeserializerU16, Error> },
    SetPinModeResponse { de: DeserializerPair<DeserializerU8, DeserializerPinMode, Error> },
    PinListen { de: DeserializerPair<DeserializerU8, DeserializerU16, Error> },
    InvalidPinMode { de: DeserializerU8 },
    InvalidPinId { de: DeserializerU8 },
    InvalidPacketId { de: DeserializerU8 },
    InvalidWriteToInput { de: DeserializerPair<DeserializerU8, DeserializerU16, Error> },
    InvalidEscape { de: DeserializerU8 },
    Done,
}

impl Default for DeserializerPacketState
{
    fn default() -> Self { Self::PacketId { de: <_>::default() } }
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerPacket
{
    state: DeserializerPacketState,
}

impl Deserializer for DeserializerPacket
{
    type Output = Packet;

    type Error = Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerPacketState::*;

        let mut total_bytes = 0;

        loop
        {
            match self.state
            {
                PacketId { ref mut de } =>
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
                        Continue(Ok(0)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::Pong))));
                        },
                        Continue(Ok(1)) =>
                        {
                            self.state = Config { de: <_>::default() };
                        },
                        Continue(Ok(2)) =>
                        {
                            self.state = GetPinPowerResponse { de: <_>::default() };
                        },
                        Continue(Ok(3)) =>
                        {
                            self.state = GetPinModeResponse { de: <_>::default() };
                        },
                        Continue(Ok(4)) =>
                        {
                            self.state = SetPinPowerResponse { de: <_>::default() };
                        },
                        Continue(Ok(5)) =>
                        {
                            self.state = SetPinModeResponse { de: <_>::default() };
                        },
                        Continue(Ok(6)) =>
                        {
                            self.state = PinListen { de: <_>::default() };
                        },
                        Continue(Ok(101)) =>
                        {
                            self.state = InvalidPinMode { de: <_>::default() };
                        },
                        Continue(Ok(102)) =>
                        {
                            self.state = InvalidPinId { de: <_>::default() };
                        },
                        Continue(Ok(103)) =>
                        {
                            self.state = InvalidPacketId { de: <_>::default() };
                        },
                        Continue(Ok(104)) =>
                        {
                            self.state = InvalidWriteToInput { de: <_>::default() };
                        },
                        Continue(Ok(105)) =>
                        {
                            self.state = InvalidEscape { de: <_>::default() };
                        },
                        Continue(Ok(byte)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(Error::IncomingInvalidPacketId { byte }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                Config { ref mut de } => 
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
                        Continue(Ok(config)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::Config(config)))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                GetPinPowerResponse { ref mut de } =>
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
                        Continue(Ok((pin, power))) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::GetPinPowerResponse { pin, power }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                GetPinModeResponse { ref mut de } =>
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
                        Continue(Ok((pin, mode))) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::GetPinModeResponse { pin, mode }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                SetPinPowerResponse { ref mut de } =>
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
                        Continue(Ok((pin, power))) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::SetPinPowerResponse { pin, power }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                SetPinModeResponse { ref mut de } =>
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
                        Continue(Ok((pin, mode))) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::SetPinModeResponse { pin, mode }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                PinListen { ref mut de } =>
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
                        Continue(Ok((pin, power))) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Ok(Packet::PinListen { pin, power }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                InvalidPinMode { ref mut de } =>
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
                        Continue(Ok(byte)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(Error::InvalidPinMode { byte }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                InvalidPinId { ref mut de } =>
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
                        Continue(Ok(byte)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(Error::InvalidPinId { byte }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                InvalidPacketId { ref mut de } =>
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
                        Continue(Ok(byte)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(Error::InvalidPacketId { byte }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                InvalidWriteToInput { ref mut de } =>
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
                        Continue(Ok((pin, power))) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(Error::InvalidWriteToInput { pin, power }))));
                        },
                        Continue(Err(error)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(error.into()))));
                        },
                    }
                },
                InvalidEscape { ref mut de } =>
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
                        Continue(Ok(byte)) =>
                        {
                            self.state = Done;
                            return Poll::Ready((total_bytes, Continue(Err(Error::InvalidEscape { byte }))));
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