use std::{fmt, slice};

use bitflags::bitflags;

use crate::de::{self, Literal, walk_whitespace};

use super::PinMode;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct PinFlags: u8 {
        const DIGITAL_INPUT = 1 << 0;
        const DIGITAL_OUTPUT = 1 << 1;
        const ANALOG_INPUT = 1 << 2;
        const ANALOG_OUTPUT = 1 << 3;
        const RESERVED = 1 << 4;
    }
}

impl From<PinMode> for PinFlags {
    fn from(value: PinMode) -> Self {
        match value {
            PinMode::Ignored => PinFlags::empty(),
            PinMode::DigitalInput => PinFlags::DIGITAL_INPUT,
            PinMode::DigitalOutput => PinFlags::DIGITAL_OUTPUT,
            PinMode::AnalogInput => PinFlags::ANALOG_INPUT,
            PinMode::AnalogOutput => PinFlags::ANALOG_OUTPUT,
        }
    }
}

impl fmt::Display for PinFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(f, "0")
        } else {
            let mut is_first = true;
            if self.contains(PinFlags::DIGITAL_INPUT) {
                if !is_first {
                    write!(f, " | ")?;
                }
                write!(f, "digitalInput")?;
                is_first = false;
            }
            if self.contains(PinFlags::DIGITAL_OUTPUT) {
                if !is_first {
                    write!(f, " | ")?;
                }
                write!(f, "digitalOutput")?;
                is_first = false;
            }
            if self.contains(PinFlags::ANALOG_INPUT) {
                if !is_first {
                    write!(f, " | ")?;
                }
                write!(f, "analogInput")?;
                is_first = false;
            }
            if self.contains(PinFlags::ANALOG_OUTPUT) {
                if !is_first {
                    write!(f, " | ")?;
                }
                write!(f, "analogOutput")?;
                is_first = false;
            }
            if self.contains(PinFlags::RESERVED) {
                if !is_first {
                    write!(f, " | ")?;
                }
                write!(f, "reserved")?;
            }
            Ok(())
        }
    }
}

impl de::Deserialize<'_> for PinFlags {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl de::Deserialize<'_> for Option<PinFlags> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        let mut result = match Option::deserialize_over(source, whitespace)? {
            None => return Ok(None),
            Some(Literal::<&[u8]>(b"0")) => PinFlags::empty(),
            Some(Literal(b"DIGITAL_INPUT")) => PinFlags::DIGITAL_INPUT,
            Some(Literal(b"DIGITAL_OUTPUT")) => PinFlags::DIGITAL_OUTPUT,
            Some(Literal(b"ANALOG_INPUT")) => PinFlags::ANALOG_INPUT,
            Some(Literal(b"ANALOG_OUTPUT")) => PinFlags::ANALOG_OUTPUT,
            Some(Literal(b"RESERVED")) => PinFlags::RESERVED,
            Some(_) => {
                *source = undo;
                return Err(());
            },
        };

        loop {
            let undo = source.clone();
            match Option::deserialize_over(source, whitespace)? {
                Some(Literal(b'|')) => { },
                _ => {
                    *source = undo;
                    break;
                },
            }

            if whitespace {
                walk_whitespace(source);
            }

            let undo = source.clone();
            result |= match Option::deserialize_over(source, whitespace)? {
                None => return Err(()),
                Some(Literal::<&[u8]>(b"0")) => PinFlags::empty(),
                Some(Literal(b"DIGITAL_INPUT")) => PinFlags::DIGITAL_INPUT,
                Some(Literal(b"DIGITAL_OUTPUT")) => PinFlags::DIGITAL_OUTPUT,
                Some(Literal(b"ANALOG_INPUT")) => PinFlags::ANALOG_INPUT,
                Some(Literal(b"ANALOG_OUTPUT")) => PinFlags::ANALOG_OUTPUT,
                Some(Literal(b"RESERVED")) => PinFlags::RESERVED,
                Some(_) => {
                    *source = undo;
                    return Err(());
                },
            };
        }

        Ok(Some(result))
    }
}