use std::convert::Infallible;
use std::{io, string};

use crate::packet::{Deserialize, Serialize};

use serde::ser::SerializeStruct;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Clone, Debug)]
pub enum InvalidPinModeOrUTF8
{
    InvalidPinMode,
    InvalidUTF8,
}

impl From<string::FromUtf8Error> for InvalidPinModeOrUTF8
{
    fn from(_: string::FromUtf8Error) -> Self { Self::InvalidUTF8 }
}

pub mod packet;

pub mod serial_handler;

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
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        self.0.write_to(writer).await
    }
}

impl Deserialize for PinFlags
{
    type Error = Infallible;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, crate::packet::Error<Self::Error>>
    {
        Ok(Self(u8::read_from(reader).await?))
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
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        match self
        {
            Self::DigitalInput => 0u8.write_to(writer).await,
            Self::DigitalListen => 1u8.write_to(writer).await,
            Self::DigitalOutput => 2u8.write_to(writer).await,
            Self::AnalogInput => 3u8.write_to(writer).await,
            Self::AnalogOutput => 4u8.write_to(writer).await,
        }
    }
}

impl Deserialize for PinMode
{
    type Error = InvalidPinModeOrUTF8;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, crate::packet::Error<Self::Error>>
    {
        match u8::read_from(reader).await.map_err(crate::packet::Error::upcast)?
        {
            0 => Ok(Self::DigitalInput),
            1 => Ok(Self::DigitalListen),
            2 => Ok(Self::DigitalOutput),
            3 => Ok(Self::AnalogInput),
            4 => Ok(Self::AnalogOutput),
            _ => Err(crate::packet::Error::Other(InvalidPinModeOrUTF8::InvalidPinMode)),
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
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        self.name.write_to(writer).await?;
        self.flags.write_to(writer).await
    }
}

impl Deserialize for ConfigPin
{
    type Error = string::FromUtf8Error;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, crate::packet::Error<Self::Error>>
    {
        Ok(Self
        {
            name: String::read_from(reader).await?,
            flags: PinFlags::read_from(reader).await.map_err(crate::packet::Error::upcast)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config
{
    pub version: u16,
    pub name: String,
    pub pins: Vec<ConfigPin>,
}

impl Serialize for Config
{
    async fn write_to(&self, writer: &mut (impl AsyncWrite + Unpin)) -> io::Result<()>
    {
        self.version.write_to(writer).await?;
        self.name.write_to(writer).await?;
        self.pins.write_to(writer).await
    }
}

impl Deserialize for Config
{
    type Error = string::FromUtf8Error;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, crate::packet::Error<Self::Error>>
    {
        Ok(Self
        {
            version: u16::read_from(reader).await.map_err(crate::packet::Error::upcast)?,
            name: String::read_from(reader).await?,
            pins: Vec::read_from(reader).await?,
        })
    }
}