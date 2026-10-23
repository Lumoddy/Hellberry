use std::{fmt, slice};

use crate::de::{self, Array, Quoted, is_literal_char, is_whitespace};

use super::{PinFlags};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeadConfig {
    model: String,
    pins: Vec<PinConfig>,
}

impl fmt::Display for BeadConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ model: {}, pins: [{}] }}",
            self.model,
            fmt::from_fn(|f| {
                let mut first = true;
                for pin in &self.pins {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", pin)?;
                }
                Ok(())
            }))
    }
}

impl de::Deserialize<'_> for BeadConfig {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl de::Deserialize<'_> for Option<BeadConfig> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            loop {
                let undo = source.clone();
                if !source.next().copied().is_none_or(is_whitespace) {
                    *source = undo;
                    break;
                }
            }
        }

        let undo = source.clone();
        match source.next() {
            Some(b'{') => {
                let mut model = None;
                let mut pins = None;

                loop {
                    if whitespace {
                        loop {
                            let undo = source.clone();
                            if !source.next().copied().is_none_or(is_whitespace) {
                                *source = undo;
                                break;
                            }
                        }
                    }

                    match source.as_slice() {
                        [b'm', b'o', b'd', b'e', b'l', rest @ ..]
                        | [b'\"', b'm', b'o', b'd', b'e', b'l', b'\"', rest @ ..]
                        | [b'\'', b'm', b'o', b'd', b'e', b'l', b'\'', rest @ ..]
                        if model.is_none()
                            && !rest.first().copied().is_some_and(is_literal_char) =>
                        {
                            *source = rest.iter();

                            if whitespace {
                                loop {
                                    let undo = source.clone();
                                    if !source.next().copied().is_none_or(is_whitespace) {
                                        *source = undo;
                                        break;
                                    }
                                }
                            }

                            let undo = source.clone();
                            match source.next() {
                                Some(b':') => { },
                                _ => {
                                    *source = undo;
                                    break;
                                }
                            }

                            model = Some(Quoted::deserialize_over(source, whitespace)?.0);
                        },
                        [b'p', b'i', b'n', b's', rest @ ..]
                        | [b'\"', b'p', b'i', b'n', b's', b'\"', rest @ ..]
                        | [b'\'', b'p', b'i', b'n', b's', b'\'', rest @ ..]
                        if pins.is_none()
                            && !rest.first().copied().is_some_and(is_literal_char) =>
                        {
                            *source = rest.iter();

                            if whitespace {
                                loop {
                                    let undo = source.clone();
                                    if !source.next().copied().is_none_or(is_whitespace) {
                                        *source = undo;
                                        break;
                                    }
                                }
                            }

                            let undo = source.clone();
                            match source.next() {
                                Some(b':') => { },
                                _ => {
                                    *source = undo;
                                    break;
                                }
                            }

                            pins = Some(Array::deserialize_over(source, whitespace)?.0);
                        },
                        [b'}', rest @ ..]
                        if model.is_some() && pins.is_some() => {
                            *source = rest.iter();
                            break;
                        },
                        _ => {
                            return Err(())
                        },
                    }

                    if whitespace {
                        loop {
                            let undo = source.clone();
                            if !source.next().copied().is_none_or(is_whitespace) {
                                *source = undo;
                                break;
                            }
                        }
                    }

                    let undo = source.clone();
                    match source.next() {
                        Some(b',') => {
                            continue;
                        },
                        Some(b'}')
                        if model.is_some() && pins.is_some() => {
                            break;
                        },
                        _ => {
                            *source = undo;
                            return Err(())
                        },
                    }
                }

                Ok(Some(BeadConfig {
                    model: model.unwrap(),
                    pins: pins.unwrap(),
                }))
            },
            _ => {
                *source = undo;
                Ok(None)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinConfig {
    name: String,
    flags: PinFlags,
}

impl fmt::Display for PinConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ name: {}, flags: {} }}", self.name, self.flags)
    }
}

impl de::Deserialize<'_> for PinConfig {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl de::Deserialize<'_> for Option<PinConfig> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            loop {
                let undo = source.clone();
                if !source.next().copied().is_none_or(is_whitespace) {
                    *source = undo;
                    break;
                }
            }
        }

        let undo = source.clone();
        match source.next() {
            Some(b'{') => {
                let mut name = None;
                let mut flags = None;

                loop {
                    if whitespace {
                        loop {
                            let undo = source.clone();
                            if !source.next().copied().is_none_or(is_whitespace) {
                                *source = undo;
                                break;
                            }
                        }
                    }

                    match source.as_slice() {
                        [b'n', b'a', b'm', b'e', rest @ ..]
                        | [b'\"', b'n', b'a', b'm', b'e', b'\"', rest @ ..]
                        | [b'\'', b'n', b'a', b'm', b'e', b'\'', rest @ ..]
                        if name.is_none()
                            && !rest.first().copied().is_some_and(is_literal_char) =>
                        {
                            *source = rest.iter();

                            if whitespace {
                                loop {
                                    let undo = source.clone();
                                    if !source.next().copied().is_none_or(is_whitespace) {
                                        *source = undo;
                                        break;
                                    }
                                }
                            }

                            let undo = source.clone();
                            match source.next() {
                                Some(b':') => { },
                                _ => {
                                    *source = undo;
                                    break;
                                }
                            }

                            name = Some(Quoted::deserialize_over(source, whitespace)?.0);
                        },
                        [b'p', b'i', b'n', b's', rest @ ..]
                        | [b'\"', b'p', b'i', b'n', b's', b'\"', rest @ ..]
                        | [b'\'', b'p', b'i', b'n', b's', b'\'', rest @ ..]
                        if flags.is_none()
                            && !rest.first().copied().is_some_and(is_literal_char) =>
                        {
                            *source = rest.iter();

                            if whitespace {
                                loop {
                                    let undo = source.clone();
                                    if !source.next().copied().is_none_or(is_whitespace) {
                                        *source = undo;
                                        break;
                                    }
                                }
                            }

                            let undo = source.clone();
                            match source.next() {
                                Some(b':') => { },
                                _ => {
                                    *source = undo;
                                    break;
                                }
                            }

                            flags = Some(PinFlags::deserialize_over(source, whitespace)?);
                        },
                        [b'}', rest @ ..]
                        if name.is_some() && flags.is_some() => {
                            *source = rest.iter();
                            break;
                        },
                        _ => {
                            return Err(())
                        },
                    }

                    if whitespace {
                        loop {
                            let undo = source.clone();
                            if !source.next().copied().is_none_or(is_whitespace) {
                                *source = undo;
                                break;
                            }
                        }
                    }

                    let undo = source.clone();
                    match source.next() {
                        Some(b',') => {
                            continue;
                        },
                        Some(b'}')
                        if name.is_some() && flags.is_some() => {
                            break;
                        },
                        _ => {
                            *source = undo;
                            return Err(())
                        },
                    }
                }

                Ok(Some(PinConfig {
                    name: name.unwrap(),
                    flags: flags.unwrap(),
                }))
            },
            _ => {
                *source = undo;
                Ok(None)
            }
        }
    }
}