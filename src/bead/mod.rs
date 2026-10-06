use std::io::Write;
use std::{io, mem};
use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::{Poll, ready};

use crate::packet::{Deserializer, DeserializerU8, DeserializerU16, DeserializerVec, InvalidEscape, Reset, Serialize};

use ControlFlow::*;
use packet::incoming;

pub mod packet;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct PinFlags(u8);

impl From<u8> for PinFlags
{
    fn from(value: u8) -> Self { Self(value) }
}

impl From<PinFlags> for u8
{
    fn from(value: PinFlags) -> Self { value.0 }
}

impl PinFlags
{
    const fn digital_input(&self) -> bool { (self.0 & 1) != 0 }

    const fn set_digital_input(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !1) | if value { 1 } else { 0 };
        self
    }

    const fn digital_output(&self) -> bool { (self.0 & 2) != 0 }

    const fn set_digital_output(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !2) | if value { 2 } else { 0 };
        self
    }

    const fn analog_input(&self) -> bool { (self.0 & 4) != 0 }

    const fn set_analog_input(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !4) | if value { 4 } else { 0 };
        self
    }

    const fn analog_output(&self) -> bool { (self.0 & 8) != 0 }

    const fn set_analog_output(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !8) | if value { 8 } else { 0 };
        self
    }
}

impl Serialize for PinFlags
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        self.0.write_to(writer)
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerPinFlags
{
    inner: DeserializerU8,
}

impl Deserializer for DeserializerPinFlags
{
    type Output = PinFlags;

    type Error = InvalidEscape;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        match self.inner.resume(bytes)
        {
            Poll::Ready((bytes_used, Continue(Ok(result)))) => Poll::Ready((bytes_used, Continue(Ok(result.into())))),
            Poll::Ready((bytes_used, Continue(Err(error)))) => Poll::Ready((bytes_used, Continue(Err(error)))),
            Poll::Ready((bytes_used, Break(cancel))) => Poll::Ready((bytes_used, Break(cancel))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PinMode
{
    DigitalInput,
    DigitalListen,
    DigitalOutput,
    AnalogInput,
    AnalogOutput,
}

impl From<PinMode> for u8
{
    fn from(value: PinMode) -> Self
    {
        match value
        {
            PinMode::DigitalInput => 0,
            PinMode::DigitalListen => 1,
            PinMode::DigitalOutput => 2,
            PinMode::AnalogInput => 3,
            PinMode::AnalogOutput => 4,
        }
    }
}

impl TryFrom<u8> for PinMode
{
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error>
    {
        match value
        {
            0 => Ok(PinMode::DigitalInput),
            1 => Ok(PinMode::DigitalListen),
            2 => Ok(PinMode::DigitalOutput),
            3 => Ok(PinMode::AnalogInput),
            4 => Ok(PinMode::AnalogOutput),
            _ => Err(()),
        }
    }
}

impl Serialize for PinMode
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        match self
        {
            Self::DigitalInput => 0u8.write_to(writer),
            Self::DigitalListen => 1u8.write_to(writer),
            Self::DigitalOutput => 2u8.write_to(writer),
            Self::AnalogInput => 3u8.write_to(writer),
            Self::AnalogOutput => 4u8.write_to(writer),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerPinMode
{
    inner: DeserializerU8,
}

impl Deserializer for DeserializerPinMode
{
    type Output = PinMode;

    type Error = incoming::Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        match self.inner.resume(bytes)
        {
            Poll::Ready((bytes_used, Continue(Ok(result)))) => Poll::Ready((bytes_used, Continue(match result.try_into()
            {
                Ok(x) => Ok(x),
                Err(()) => Err(incoming::Error::IncomingInvalidPinMode { byte: result }),
            }))),
            Poll::Ready((bytes_used, Continue(Err(error)))) => Poll::Ready((bytes_used, Continue(Err(error.into())))),
            Poll::Ready((bytes_used, Break(cancel))) => Poll::Ready((bytes_used, Break(cancel))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigPin
{
    name: Vec<u8>,
    flags: PinFlags,
}

impl Serialize for ConfigPin
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        self.name.write_to(writer)?;
        self.flags.write_to(writer)
    }
}

#[derive(Clone, Debug)]
enum DeserializerConfigPinState
{
    Name { de: DeserializerVec<DeserializerU8> },
    Flag { name: Vec<u8>, de: DeserializerPinFlags },
    Done,
}

impl Default for DeserializerConfigPinState
{
    fn default() -> Self { Self::Name { de: <_>::default() } }
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerConfigPin
{
    state: DeserializerConfigPinState,
}

impl Deserializer for DeserializerConfigPin
{
    type Output = ConfigPin;

    type Error = incoming::Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerConfigPinState::*;

        let mut total_bytes = 0;

        loop
        {
            match self.state
            {
                Name { ref mut de } =>
                {
                    match ready!(de.resume(&bytes[total_bytes..]))
                    {
                        (bytes_used, Continue(Ok(name))) =>
                        {
                            total_bytes += bytes_used.get();
                            self.state = Flag { name, de: DeserializerPinFlags::default() };
                        },
                        (bytes_used, Continue(Err(error))) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Err(error.into()))));
                        },
                        (bytes_used, Break(cancel)) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Break(cancel)));
                        },
                    }
                },
                Flag { ref mut name, ref mut de } =>
                {
                    match ready!(de.resume(&bytes[total_bytes..]))
                    {
                        (bytes_used, Continue(Ok(flags))) =>
                        {
                            total_bytes += bytes_used.get();
                            let pin = ConfigPin { name: mem::take(name), flags };
                            self.state = Done;
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Ok(pin))));
                        },
                        (bytes_used, Continue(Err(error))) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Err(error.into()))));
                        },
                        (bytes_used, Break(cancel)) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Break(cancel)));
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config
{
    version: u16,
    name: Vec<u8>,
    pins: Vec<ConfigPin>,
}

impl Serialize for Config
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        self.version.write_to(writer)?;
        self.name.write_to(writer)?;
        self.pins.write_to(writer)
    }
}

#[derive(Clone, Debug)]
enum DeserializerConfigState
{
    Version { de: DeserializerU16 },
    Name { version: u16, de: DeserializerVec<DeserializerU8> },
    Pins { version: u16, name: Vec<u8>, de: DeserializerVec<DeserializerConfigPin> },
    Done,
}

impl Default for DeserializerConfigState
{
    fn default() -> Self { Self::Version { de: <_>::default() } }
}

#[derive(Clone, Debug, Default)]
pub struct DeserializerConfig
{
    state: DeserializerConfigState,
}

impl Deserializer for DeserializerConfig
{
    type Output = Config;

    type Error = incoming::Error;

    fn resume(&mut self, bytes: &[u8]) -> Poll<(NonZero<usize>, ControlFlow<Reset, Result<Self::Output, Self::Error>>)>
    {
        use DeserializerConfigState::*;

        let mut total_bytes = 0;

        loop
        {
            match self.state
            {
                Version { ref mut de } =>
                {
                    match ready!(de.resume(&bytes[total_bytes..]))
                    {
                        (bytes_used, Continue(Ok(version))) =>
                        {
                            total_bytes += bytes_used.get();
                            self.state = Name { version, de: DeserializerVec::default() };
                        },
                        (bytes_used, Continue(Err(error))) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Err(error.into()))));
                        },
                        (bytes_used, Break(cancel)) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Break(cancel)));
                        },
                    }
                },
                Name { ref mut version, ref mut de } =>
                {
                    match ready!(de.resume(&bytes[total_bytes..]))
                    {
                        (bytes_used, Continue(Ok(name))) =>
                        {
                            total_bytes += bytes_used.get();
                            self.state = Pins { version: *version, name, de: DeserializerVec::default() };
                        },
                        (bytes_used, Continue(Err(error))) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Err(error.into()))));
                        },
                        (bytes_used, Break(cancel)) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Break(cancel)));
                        },
                    }
                },
                Pins { ref mut version, ref mut name, ref mut de } =>
                {
                    match ready!(de.resume(&bytes[total_bytes..]))
                    {
                        (bytes_used, Continue(Ok(pins))) =>
                        {
                            total_bytes += bytes_used.get();
                            let config = Config { version: *version, name: mem::take(name), pins };
                            self.state = Done;
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Ok(config))));
                        },
                        (bytes_used, Continue(Err(error))) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Continue(Err(error.into()))));
                        },
                        (bytes_used, Break(cancel)) =>
                        {
                            total_bytes += bytes_used.get();
                            return Poll::Ready((NonZero::new(total_bytes).unwrap(), Break(cancel)));
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