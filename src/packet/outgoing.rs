use std::fmt;

use crate::bead::{BeadConfig, PinFlags, PinMode};
use crate::im::Ash;

#[derive(Clone, Debug, PartialEq)]
pub enum Packet<'a> {
    /// Encoded as `"pong"`.
    Pong { },
    /// Encoded as `"config = { name: <name> }`.
    Config { name: Option<Ash<'a, str>> },
    /// Encoded as `"beads.connect({bead})`.
    BeadConnected { bead: Ash<'a, str> },
    /// Encoded as `"beads.disconnect({bead})`.
    BeadDisconnected { bead: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pong"`.
    BeadPong { bead: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pins.{pin}.power = {power}"`.
    BeadPinPowerResponse { bead: Ash<'a, str>, pin: Ash<'a, str>, power: f32 },
    /// Encoded as `"beads.{bead}.pins.{pin}.mode = {mode}"`.
    BeadPinModeResponse { bead: Ash<'a, str>, pin: Ash<'a, str>, mode: PinMode },
    /// Encoded as `"beads.{bead}.pins.{pin}.flags = {flags}"`.
    BeadPinFlagsResponse { bead: Ash<'a, str>, pin: Ash<'a, str>, flags: PinFlags },
    /// Encoded as `"error.malformedPacketAt(<pos>)"`.
    MalformedPacket { pos: usize },
    /// Encoded as `"beads.{bead}.pins.{pin}.error.unsupportedMode({mode})"`.
    UnsupportedBeadPinMode { bead: Ash<'a, str>, pin: Ash<'a, str>, mode: PinMode },
    /// Encoded as `"beads.{bead}.pins.{pin}.error.getPowerDenied()"`.
    BeadPinGetPowerDenied { bead: Ash<'a, str>, pin: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pins.{pin}.error.setPowerDenied()"`.
    BeadPinSetPowerDenied { bead: Ash<'a, str>, pin: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.config = { model: <model>, pins: [{ name: <name>, flags: <flags> }, ...] }`.
    BeadConfig { bead: Ash<'a, str>, config: BeadConfig<'a> },
}

impl Packet<'_> {
    pub fn into_static<'a>(self) -> Packet<'a> {
        match self {
            Packet::Pong { } => Packet::Pong { },
            Packet::Config { name } => Packet::Config { name: name.map(|x| x.into_static()) },
            Packet::BeadConnected { bead } => Packet::BeadConnected { bead: bead.into_static() },
            Packet::BeadDisconnected { bead } => Packet::BeadDisconnected { bead: bead.into_static() },
            Packet::BeadPong { bead } => Packet::BeadPong { bead: bead.into_static() },
            Packet::BeadPinPowerResponse { bead, pin, power } => Packet::BeadPinPowerResponse { bead: bead.into_static(), pin: pin.into_static(), power },
            Packet::BeadPinModeResponse { bead, pin, mode } => Packet::BeadPinModeResponse { bead: bead.into_static(), pin: pin.into_static(), mode },
            Packet::BeadPinFlagsResponse { bead, pin, flags } => Packet::BeadPinFlagsResponse { bead: bead.into_static(), pin: pin.into_static(), flags },
            Packet::MalformedPacket { pos } => Packet::MalformedPacket { pos },
            Packet::UnsupportedBeadPinMode { bead, pin, mode } => Packet::UnsupportedBeadPinMode { bead: bead.into_static(), pin: pin.into_static(), mode },
            Packet::BeadPinGetPowerDenied { bead, pin } => Packet::BeadPinGetPowerDenied { bead: bead.into_static(), pin: pin.into_static() },
            Packet::BeadPinSetPowerDenied { bead, pin } => Packet::BeadPinSetPowerDenied { bead: bead.into_static(), pin: pin.into_static() },
            Packet::BeadConfig { bead, config } => Packet::BeadConfig { bead: bead.into_static(), config: config.into_static() },
        }
    }
}

impl fmt::Display for Packet<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Packet::Pong { } => write!(f, "pong"),
            Packet::Config { name: None } => write!(f, "config = {{ }}"),
            Packet::Config { name: Some(name) } => write!(f, "config = {{ name: {} }}", name),
            Packet::BeadConnected { bead } => write!(f, "beads.connect(\"{}\")", bead),
            Packet::BeadDisconnected { bead } => write!(f, "beads.disconnect(\"{}\")", bead),
            Packet::BeadPong { bead } => write!(f, "beads.{}.pong", bead),
            Packet::BeadPinPowerResponse { bead, pin, power } => write!(f, "beads.{}.pins.{}.power = {}", bead, pin, power),
            Packet::BeadPinModeResponse { bead, pin, mode } => write!(f, "beads.{}.pins.{}.mode = {}", bead, pin, mode),
            Packet::BeadPinFlagsResponse { bead, pin, flags } => write!(f, "beads.{}.pins.{}.flags = {}", bead, pin, flags),
            Packet::MalformedPacket { pos } => write!(f, "error.malformedPacketAt({})", pos),
            Packet::UnsupportedBeadPinMode { bead, pin, mode } => write!(f, "beads.{}.pins.{}.error.unsupportedMode({})", bead, pin, mode),
            Packet::BeadPinGetPowerDenied { bead, pin } => write!(f, "beads.{}.pins.{}.error.getPowerDenied()", bead, pin),
            Packet::BeadPinSetPowerDenied { bead, pin } => write!(f, "beads.{}.pins.{}.error.setPowerDenied()", bead, pin),
            Packet::BeadConfig { bead, config } => write!(f, "beads.{}.config = {}", bead, config),
        }
    }
}