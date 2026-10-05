use std::io::Write;
use tokio::io;

use crate::bead::PinMode;
use crate::packet::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet
{
    Ping,
    WholeConfig,
    GetPinPower { pin: u8 },
    GetPinMode { pin: u8 },
    SetPinPower { pin: u8, power: u16 },
    SetPinMode { pin: u8, mode: PinMode },
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

impl Serialize for Packet
{
    fn write_to(&self, writer: &mut impl Write) -> io::Result<()>
    {
        match self
        {
            Self::Ping => (0u8,).write_to(writer),
            Self::WholeConfig => (1u8,).write_to(writer),
            Self::GetPinPower { pin } => (2u8, *pin).write_to(writer),
            Self::GetPinMode { pin } => (3u8, *pin).write_to(writer),
            Self::SetPinPower { pin, power } => (4u8, *pin, *power).write_to(writer),
            Self::SetPinMode { pin, mode } => (5u8, *pin, *mode).write_to(writer),
        }
    }
}