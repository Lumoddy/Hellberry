use std::{fmt, slice};

use crate::de::{self, Literal, walk_whitespace};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinMode {
    Ignored,
    DigitalInput,
    DigitalOutput,
    AnalogInput,
    AnalogOutput,
}

impl fmt::Display for PinMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PinMode::Ignored => write!(f, "IGNORED"),
            PinMode::DigitalInput => write!(f, "DIGITAL_INPUT"),
            PinMode::DigitalOutput => write!(f, "DIGITAL_OUTPUT"),
            PinMode::AnalogInput => write!(f, "ANALOG_INPUT"),
            PinMode::AnalogOutput => write!(f, "ANALOG_OUTPUT"),
        }
    }
}

impl de::Deserialize<'_> for PinMode {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl de::Deserialize<'_> for Option<PinMode> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        Ok(match Option::deserialize_over(source, whitespace)? {
            None => None,
            Some(Literal::<&[u8]>(b"IGNORED")) => Some(PinMode::Ignored),
            Some(Literal(b"DIGITAL_INPUT")) => Some(PinMode::DigitalInput),
            Some(Literal(b"DIGITAL_OUTPUT")) => Some(PinMode::DigitalOutput),
            Some(Literal(b"ANALOG_INPUT")) => Some(PinMode::AnalogInput),
            Some(Literal(b"ANALOG_OUTPUT")) => Some(PinMode::AnalogOutput),
            Some(_) => {
                *source = undo;
                return Err(());
            },
        })
    }
}