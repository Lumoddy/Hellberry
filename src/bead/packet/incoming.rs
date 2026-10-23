use std::{fmt, slice};

use crate::bead::{PinConfig, PinFlags, PinMode};
use crate::de::{self, Field, Literal, walk_whitespace};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet
{
    /// Encoded as `"pong"`.
    Pong,
    /// Encoded as `"pins.{pin}.power = {power}"`.
    PinPowerResponse { pin: String, power: u16 },
    /// Encoded as `"pins.{pin}.mode = {mode}"`.
    PinModeResponse { pin: String, mode: PinMode },
    /// Encoded as `"pins.{pin}.flags = {flags}"`.
    PinFlagsResponse { pin: String, flags: PinFlags },
    /// Encoded as `"error.malformedPacketAt(<pos>)"`.
    MalformedPacket { pos: usize },
    /// Encoded as `"pins.{pin}.error.unsupportedMode({mode})"`.
    UnsupportedPinMode { pin: String, mode: PinMode },
    /// Encoded as `"pins.{pin}.error.getPowerDenied()"`.
    PinGetPowerDenied { pin: String },
    /// Encoded as `"pins.{pin}.error.setPowerDenied()"`.
    PinSetPowerDenied { pin: String },
    /// Encoded as `"config = { model: <model>, pins: [{ name: <name>, flags: <flags> }, ...] }`.
    Config { config: PinConfig },
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pong => write!(f, "pong"),
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

impl de::Deserialize<'_> for Option<Packet> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        Ok(match Option::deserialize_over(source, whitespace)? {
            None => return Ok(None),
            Some(Literal::<&[u8]>(b"pong")) => {
                Some(Packet::Pong)
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

                let config = PinConfig::deserialize_over(source, whitespace)?;

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

                                if let Literal(b',') = Literal::deserialize_over(source, whitespace)? {
                                    if whitespace {
                                        walk_whitespace(source);
                                    }
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
            _ => {
                *source = undo;
                return Err(());
            },
        })
    }
}