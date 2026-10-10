use std::io::Write;
use std::{io, mem};
use std::num::NonZero;
use std::ops::ControlFlow;
use std::task::{Poll, ready};

use crate::packet::{Deserializer, DeserializerString, DeserializerU8, DeserializerU16, DeserializerVec, InvalidEscape, Reset, Serialize};

use packet::incoming;
use serde::ser::SerializeStruct;

pub mod packet;

pub mod serial_handler;

use ControlFlow::*;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct PinFlags(u8);

impl PinFlags
{
    pub const fn new() -> Self { Self(0) }

    pub const fn digital_input(&self) -> bool { (self.0 & 1) != 0 }

    pub const fn set_digital_input(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !1) | if value { 1 } else { 0 };
        self
    }

    pub const fn digital_output(&self) -> bool { (self.0 & 2) != 0 }

    pub const fn set_digital_output(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !2) | if value { 2 } else { 0 };
        self
    }

    pub const fn analog_input(&self) -> bool { (self.0 & 4) != 0 }

    pub const fn set_analog_input(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !4) | if value { 4 } else { 0 };
        self
    }

    pub const fn analog_output(&self) -> bool { (self.0 & 8) != 0 }

    pub const fn set_analog_output(&mut self, value: bool) -> &mut Self
    {
        self.0 = (self.0 & !8) | if value { 8 } else { 0 };
        self
    }
}

impl From<u8> for PinFlags
{
    fn from(value: u8) -> Self { Self(value) }
}

impl From<PinFlags> for u8
{
    fn from(value: PinFlags) -> Self { value.0 }
}

impl Serialize for PinFlags
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        self.0.write_to(writer)
    }
}

impl serde::Serialize for PinFlags
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        if !serializer.is_human_readable()
        {
            return serializer.serialize_u8(self.0)
        }

        let mut serializer = serializer.serialize_struct("PinFlags", 4)?;
        serializer.serialize_field("digital-input", &self.digital_input())?;
        serializer.serialize_field("digital-output", &self.digital_output())?;
        serializer.serialize_field("analog-input", &self.analog_input())?;
        serializer.serialize_field("analog-output", &self.analog_output())?;
        serializer.end()
    }
}

impl<'de> serde::Deserialize<'de> for PinFlags
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        if !deserializer.is_human_readable()
        {
            return Ok(Self(u8::deserialize(deserializer)?))
        }

        #[derive(serde::Deserialize)]
        struct PinFlagsHelper
        {
            #[serde(rename = "digital-input")]
            digital_input: bool,
            #[serde(rename = "digital-output")]
            digital_output: bool,
            #[serde(rename = "analog-input")]
            analog_input: bool,
            #[serde(rename = "analog-output")]
            analog_output: bool,
        }

        let helper = PinFlagsHelper::deserialize(deserializer)?;
        let mut flags = Self::default();
        flags.set_digital_input(helper.digital_input);
        flags.set_digital_output(helper.digital_output);
        flags.set_analog_input(helper.analog_input);
        flags.set_analog_output(helper.analog_output);
        Ok(flags)
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

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]

pub struct ConfigPin
{
    name: String,
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
    Name { de: DeserializerString },
    Flag { name: String, de: DeserializerPinFlags },
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
                            self.state = Flag { name, de: <_>::default() };
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
    name: String,
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
    Name { version: u16, de: DeserializerString },
    Pins { version: u16, name: String, de: DeserializerVec<DeserializerConfigPin> },
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
                            self.state = Name { version, de: <_>::default() };
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