use std::fmt;

use crate::bead::PinMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    /// Encoded as `"ping"`.
    Ping,
    /// Encoded as `"config"`.
    Config,
    /// Encoded as `"pins.{pin}.getPower()"`.
    GetPinPower { pin: String },
    /// Encoded as `"pins.{pin}.getMode()"`.
    GetPinMode { pin: String },
    /// Encoded as `"pins.{pin}.flags"`.
    PinFlags { pin: String },
    /// Encoded as `"pins.{pin}.setPower({power})"`.
    SetPinPower { pin: String, power: u8 },
    /// Encoded as `"pins.{pin}.setMode({mode})"`.
    SetPinMode { pin: String, mode: PinMode },
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Packet::Ping => write!(f, "ping"),
            Packet::Config => write!(f, "config"),
            Packet::GetPinPower { pin } => write!(f, "pins.{}.getPower()", pin),
            Packet::GetPinMode { pin } => write!(f, "pins.{}.getMode()", pin),
            Packet::PinFlags { pin } => write!(f, "pins.{}.flags", pin),
            Packet::SetPinPower { pin, power } => write!(f, "pins.{}.setPower({})", pin, power),
            Packet::SetPinMode { pin, mode } => write!(f, "pins.{}.setMode({})", pin, mode),
        }
    }
}