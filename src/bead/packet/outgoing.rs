use std::fmt;

use crate::bead::PinMode;
use crate::im::Ash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet<'a> {
    /// Encoded as `"ping"`.
    Ping { },
    /// Encoded as `"config"`.
    Config { },
    /// Encoded as `"pins.{pin}.getPower()"`.
    GetPinPower { pin: Ash<'a, str> },
    /// Encoded as `"pins.{pin}.getMode()"`.
    GetPinMode { pin: Ash<'a, str> },
    /// Encoded as `"pins.{pin}.flags"`.
    PinFlags { pin: Ash<'a, str> },
    /// Encoded as `"pins.{pin}.setPower({power})"`.
    SetPinPower { pin: Ash<'a, str>, power: u8 },
    /// Encoded as `"pins.{pin}.setMode({mode})"`.
    SetPinMode { pin: Ash<'a, str>, mode: PinMode },
}

impl Packet<'_> {
    pub fn into_static<'a>(self) -> Packet<'a> {
        match self {
            Packet::Ping { } => Packet::Ping { },
            Packet::Config { } => Packet::Config { },
            Packet::GetPinPower { pin } => Packet::GetPinPower { pin: pin.into_static() },
            Packet::GetPinMode { pin } => Packet::GetPinMode { pin: pin.into_static() },
            Packet::PinFlags { pin } => Packet::PinFlags { pin: pin.into_static() },
            Packet::SetPinPower { pin, power } => Packet::SetPinPower { pin: pin.into_static(), power },
            Packet::SetPinMode { pin, mode } => Packet::SetPinMode { pin: pin.into_static(), mode },
        }
    }
}

impl fmt::Display for Packet<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Packet::Ping { } => write!(f, "ping"),
            Packet::Config { } => write!(f, "config"),
            Packet::GetPinPower { pin } => write!(f, "pins.{}.getPower()", pin),
            Packet::GetPinMode { pin } => write!(f, "pins.{}.getMode()", pin),
            Packet::PinFlags { pin } => write!(f, "pins.{}.flags", pin),
            Packet::SetPinPower { pin, power } => write!(f, "pins.{}.setPower({})", pin, power),
            Packet::SetPinMode { pin, mode } => write!(f, "pins.{}.setMode({})", pin, mode),
        }
    }
}