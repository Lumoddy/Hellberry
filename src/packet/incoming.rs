use std::{fmt, slice};

use crate::bead::{self, PinMode};
use crate::de::{self, Field, Literal, walk_whitespace};
use crate::im::Ash;

#[derive(Clone, Debug, PartialEq)]
pub enum Packet<'a> {
    /// Encoded as `"ping"`.
    Ping { },
    /// Encoded as `"config"`.
    Config { },
    /// Encoded as `"beads.{bead}.ping"`.
    BeadPing { bead: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.config"`.
    BeadConfig { bead: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pins.{pin}.getPower()"`.
    GetBeadPinPower { bead: Ash<'a, str>, pin: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pins.{pin}.getMode()"`.
    GetBeadPinMode { bead: Ash<'a, str>, pin: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pins.{pin}.flags"`.
    BeadPinFlags { bead: Ash<'a, str>, pin: Ash<'a, str> },
    /// Encoded as `"beads.{bead}.pins.{pin}.setPower({power})"`.
    SetBeadPinPower { bead: Ash<'a, str>, pin: Ash<'a, str>, power: f32 },
    /// Encoded as `"beads.{bead}.pins.{pin}.setMode({mode})"`.
    SetBeadPinMode { bead: Ash<'a, str>, pin: Ash<'a, str>, mode: PinMode },
}

impl Packet<'_> {
    pub fn into_static<'a>(self) -> Packet<'a> {
        match self {
            Packet::Ping { } => Packet::Ping { },
            Packet::Config { } => Packet::Config { },
            Packet::BeadPing { bead } => Packet::BeadPing { bead: bead.into_static() },
            Packet::BeadConfig { bead } => Packet::BeadConfig { bead: bead.into_static() },
            Packet::GetBeadPinPower { bead, pin } => Packet::GetBeadPinPower { bead: bead.into_static(), pin: pin.into_static() },
            Packet::GetBeadPinMode { bead, pin } => Packet::GetBeadPinMode { bead: bead.into_static(), pin: pin.into_static() },
            Packet::BeadPinFlags { bead, pin } => Packet::BeadPinFlags { bead: bead.into_static(), pin: pin.into_static() },
            Packet::SetBeadPinPower { bead, pin, power } => Packet::SetBeadPinPower { bead: bead.into_static(), pin: pin.into_static(), power },
            Packet::SetBeadPinMode { bead, pin, mode } => Packet::SetBeadPinMode { bead: bead.into_static(), pin: pin.into_static(), mode },
        }
    }
}

impl fmt::Display for Packet<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Packet::Ping { } => write!(f, "ping"),
            Packet::Config { } => write!(f, "config"),
            Packet::BeadPing { bead } => write!(f, "beads.{}.ping", bead),
            Packet::BeadConfig { bead } => write!(f, "beads.{}.config", bead),
            Packet::GetBeadPinPower { bead, pin } => write!(f, "beads.{}.pins.{}.getPower()", bead, pin),
            Packet::GetBeadPinMode { bead, pin } => write!(f, "beads.{}.pins.{}.getMode()", bead, pin),
            Packet::BeadPinFlags { bead, pin } => write!(f, "beads.{}.pins.{}.flags", bead, pin),
            Packet::SetBeadPinPower { bead, pin, power } => write!(f, "beads.{}.pins.{}.setPower({})", bead, pin, power),
            Packet::SetBeadPinMode { bead, pin, mode } => write!(f, "beads.{}.pins.{}.setMode({})", bead, pin, mode),
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
            Some(Literal::<&[u8]>(b"ping")) => {
                Some(Packet::Ping { })
            },
            Some(Literal(b"config")) => {
                Some(Packet::Config { })
            },
            Some(Literal(b"beads")) => {
                let Field(Literal(bead)) = Field::deserialize_over(source, whitespace)?;

                let undo = source.clone();
                match Field::deserialize_over(source, whitespace)? {
                    Field(Literal::<&[u8]>(b"ping")) => {
                        Some(Packet::BeadPing { bead })
                    },
                    Field(Literal(b"config")) => {
                        Some(Packet::BeadConfig { bead })
                    },
                    Field(Literal(b"pins")) => {
                        let Field(Literal(pin)) = Field::deserialize_over(source, whitespace)?;

                        let undo = source.clone();
                        match Field::deserialize_over(source, whitespace)? {
                            Field(Literal::<&[u8]>(b"getPower")) => {
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

                                Some(Packet::GetBeadPinPower { bead, pin })
                            },
                            Field(Literal(b"getMode")) => {
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

                                Some(Packet::GetBeadPinMode { bead, pin })
                            },
                            Field(Literal(b"flags")) => {
                                Some(Packet::BeadPinFlags { bead, pin })
                            },
                            Field(Literal(b"setPower")) => {
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

                                let power = u8::deserialize_over(source, whitespace)? as f32
                                    / bead::ANALOG_WRITE_MAX as f32;

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

                                Some(Packet::SetBeadPinPower { bead, pin, power })
                            },
                            Field(Literal(b"setMode")) => {
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

                                Some(Packet::SetBeadPinMode { bead, pin, mode })
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