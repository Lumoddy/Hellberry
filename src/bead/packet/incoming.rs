use std::{fmt, slice};

use crate::bead::{BeadConfig, PinFlags, PinMode};
use crate::de::{self, Field, Literal, walk_whitespace};
use crate::im::Ash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet<'a> {
    /// Encoded as `"pong"`.
    Pong { },
    /// Encoded as `"pins.{pin}.power = {power}"`.
    PinPowerResponse { pin: Ash<'a, str>, power: u16 },
    /// Encoded as `"pins.{pin}.mode = {mode}"`.
    PinModeResponse { pin: Ash<'a, str>, mode: PinMode },
    /// Encoded as `"pins.{pin}.flags = {flags}"`.
    PinFlagsResponse { pin: Ash<'a, str>, flags: PinFlags },
    /// Encoded as `"error.malformedPacketAt(<pos>)"`.
    MalformedPacket { pos: usize },
    /// Encoded as `"pins.{pin}.error.unsupportedMode({mode})"`.
    UnsupportedPinMode { pin: Ash<'a, str>, mode: PinMode },
    /// Encoded as `"pins.{pin}.error.getPowerDenied()"`.
    PinGetPowerDenied { pin: Ash<'a, str> },
    /// Encoded as `"pins.{pin}.error.setPowerDenied()"`.
    PinSetPowerDenied { pin: Ash<'a, str> },
    /// Encoded as `"config = { model: <model>, pins: [{ name: <name>, flags: <flags> }, ...] }`.
    Config { config: BeadConfig<'a> },
}

impl Packet<'_> {
    pub fn into_static<'a>(self) -> Packet<'a> {
        match self {
            Packet::Pong { } => Packet::Pong { },
            Packet::PinPowerResponse { pin, power } => Packet::PinPowerResponse { pin: pin.into_static(), power },
            Packet::PinModeResponse { pin, mode } => Packet::PinModeResponse { pin: pin.into_static(), mode },
            Packet::PinFlagsResponse { pin, flags } => Packet::PinFlagsResponse { pin: pin.into_static(), flags },
            Packet::MalformedPacket { pos } => Packet::MalformedPacket { pos },
            Packet::UnsupportedPinMode { pin, mode } => Packet::UnsupportedPinMode { pin: pin.into_static(), mode },
            Packet::PinGetPowerDenied { pin } => Packet::PinGetPowerDenied { pin: pin.into_static() },
            Packet::PinSetPowerDenied { pin } => Packet::PinSetPowerDenied { pin: pin.into_static() },
            Packet::Config { config } => Packet::Config { config: config.into_static() },
        }
    }
}

impl fmt::Display for Packet<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pong { } => write!(f, "pong"),
            Self::PinPowerResponse { pin, power } => write!(f, "pins.{pin}.power = {power}"),
            Self::PinModeResponse { pin, mode } => write!(f, "pins.{pin}.mode = {mode}"),
            Self::PinFlagsResponse { pin, flags } => write!(f, "pins.{pin}.flags = {flags}"),
            Self::MalformedPacket { pos } => write!(f, "error.malformedPacketAt({pos})"),
            Self::UnsupportedPinMode { pin, mode } => write!(f, "pins.{pin}.error.unsupportedMode({mode})"),
            Self::PinGetPowerDenied { pin } => write!(f, "pins.{pin}.error.getPowerDenied()"),
            Self::PinSetPowerDenied { pin } => write!(f, "pins.{pin}.error.setPowerDenied()"),
            Self::Config { config } => write!(f, "config = {config}"),
        }
    }
}

impl<'a> de::Deserialize<'a> for Packet<'a> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl<'a> de::Deserialize<'a> for Option<Packet<'a>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        Ok(match Option::deserialize_over(source, whitespace)? {
            None => return Ok(None),
            Some(Literal::<&[u8]>(b"pong")) => {
                Some(Packet::Pong { })
            },
            Some(Literal(b"config")) => {
                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Literal(b'=') = Literal::deserialize_over(source, whitespace)? else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let config = BeadConfig::deserialize_over(source, whitespace)?;

                Some(Packet::Config { config })
            },
            Some(Literal(b"pins")) => {
                if whitespace {
                    walk_whitespace(source);
                }

                let Field(Literal(pin)) = Field::deserialize_over(source, whitespace)?;

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                match Field::deserialize_over(source, whitespace)? {
                    Field(Literal::<&[u8]>(b"power")) => {
                        if whitespace {
                            walk_whitespace(source);
                        }

                        let undo = source.clone();
                        let Literal(b'=') = Literal::deserialize_over(source, whitespace)? else {
                            *source = undo;
                            return Err(());
                        };

                        if whitespace {
                            walk_whitespace(source);
                        }

                        let power = u16::deserialize_over(source, whitespace)?;

                        Some(Packet::PinPowerResponse { pin, power })
                    },
                    Field(Literal(b"mode")) => {
                        if whitespace {
                            walk_whitespace(source);
                        }

                        let undo = source.clone();
                        let Literal(b'=') = Literal::deserialize_over(source, whitespace)? else {
                            *source = undo;
                            return Err(());
                        };

                        if whitespace {
                            walk_whitespace(source);
                        }

                        let mode = PinMode::deserialize_over(source, whitespace)?;

                        Some(Packet::PinModeResponse { pin, mode })
                    },
                    Field(Literal(b"flags")) => {
                        if whitespace {
                            walk_whitespace(source);
                        }

                        let undo = source.clone();
                        let Literal(b'=') = Literal::deserialize_over(source, whitespace)? else {
                            *source = undo;
                            return Err(());
                        };

                        if whitespace {
                            walk_whitespace(source);
                        }

                        let flags = PinFlags::deserialize_over(source, whitespace)?;

                        Some(Packet::PinFlagsResponse { pin, flags })
                    },
                    Field(Literal(b"error")) => {
                        if whitespace {
                            walk_whitespace(source);
                        }

                        let undo = source.clone();
                        match Field::deserialize_over(source, whitespace)? {
                            Field(Literal::<&[u8]>(b"unsupportedMode")) => {
                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                let Literal(b'(') = Literal::deserialize_over(source, whitespace)? else {
                                    *source = undo;
                                    return Err(());
                                };

                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let mode = PinMode::deserialize_over(source, whitespace)?;

                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                if let Some(b',') = source.next() {
                                    if whitespace {
                                        walk_whitespace(source);
                                    }
                                } else {
                                    *source = undo;
                                }

                                let undo = source.clone();
                                let Literal(b')') = Literal::deserialize_over(source, whitespace)? else {
                                    *source = undo;
                                    return Err(());
                                };

                                Some(Packet::UnsupportedPinMode { pin, mode })
                            },
                            Field(Literal(b"getPowerDenied")) => {
                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                let Literal(b'(') = Literal::deserialize_over(source, whitespace)? else {
                                    *source = undo;
                                    return Err(());
                                };

                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                let Literal(b')') = Literal::deserialize_over(source, whitespace)? else {
                                    *source = undo;
                                    return Err(());
                                };

                                Some(Packet::PinGetPowerDenied { pin })
                            },
                            Field(Literal(b"setPowerDenied")) => {
                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                let Literal(b'(') = Literal::deserialize_over(source, whitespace)? else {
                                    *source = undo;
                                    return Err(());
                                };

                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                let Literal(b')') = Literal::deserialize_over(source, whitespace)? else {
                                    *source = undo;
                                    return Err(());
                                };

                                Some(Packet::PinSetPowerDenied { pin })
                            },
                            _ => {
                                *source = undo;
                                return Err(());
                            },
                        }
                    },
                    _ => {
                        *source = undo;
                        return Err(());
                    },
                }
            },
            Some(Literal(b"error")) => {
                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                match Field::deserialize_over(source, whitespace)? {
                    Field(Literal::<&[u8]>(b"malformedPacketAt")) => {
                        if whitespace {
                            walk_whitespace(source);
                        }

                        let undo = source.clone();
                        let Literal(b'(') = Literal::deserialize_over(source, whitespace)? else {
                            *source = undo;
                            return Err(());
                        };

                        if whitespace {
                            walk_whitespace(source);
                        }

                        let pos = usize::deserialize_over(source, whitespace)?;

                        if whitespace {
                            walk_whitespace(source);
                        }

                        let undo = source.clone();
                        let Literal(b')') = Literal::deserialize_over(source, whitespace)? else {
                            *source = undo;
                            return Err(());
                        };

                        Some(Packet::MalformedPacket { pos })
                    },
                    _ => {
                        *source = undo;
                        return Err(());
                    },
                }
            },
            _ => {
                *source = undo;
                return Err(());
            },
        })
    }
}