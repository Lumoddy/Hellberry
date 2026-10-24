use std::borrow::Cow;
use std::{fmt, slice};

use crate::de::{self, Array, Literal, Quoted, walk_whitespace};
use crate::im::Ash;

use super::{PinFlags};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeadConfig<'a> {
    model: Ash<'a, str>,
    pins: Cow<'a, [PinConfig<'a>]>,
}

impl BeadConfig<'_> {
    pub fn into_static<'a>(self) -> BeadConfig<'a> {
        let BeadConfig { model, pins } = self;
        BeadConfig {
            model: model.into_static(),
            pins: match pins {
                Cow::Borrowed(borrowed) => borrowed.into_iter().map(|x| x.clone().into_static()).collect(),
                Cow::Owned(owned) => owned.into_iter().map(|x| x.into_static()).collect(),
            },
        }
    }
}

impl fmt::Display for BeadConfig<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ model: \"{}\", pins: [{}] }}",
            self.model,
            fmt::from_fn(|f| {
                let mut first = true;
                for pin in self.pins.iter() {
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

impl<'a> de::Deserialize<'a> for BeadConfig<'a> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl<'a> de::Deserialize<'a> for Option<BeadConfig<'a>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        match Option::deserialize_over(source, whitespace)? {
            None => return Ok(None),
            Some(Literal(b'{')) => {
                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Literal::<&[u8]>(b"model") = Literal::deserialize_over(source, whitespace)? else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Some(b':') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let Quoted(model) = Quoted::deserialize_over(source, whitespace)?;

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Some(b',') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Literal::<&[u8]>(b"pins") = Literal::deserialize_over(source, whitespace)? else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Some(b':') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let Array(pins) = Array::deserialize_over(source, whitespace)?;

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
                let Some(b'}') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                Ok(Some(BeadConfig { model, pins }))
            },
            Some(_) => {
                *source = undo;
                Err(())
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinConfig<'a> {
    name: Ash<'a, str>,
    flags: PinFlags,
}

impl PinConfig<'_> {
    pub fn into_static<'a>(self) -> PinConfig<'a> {
        let PinConfig { name, flags } = self;
        PinConfig { name: name.into_static(), flags }
    }
}

impl fmt::Display for PinConfig<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ name: \"{}\", flags: {} }}", self.name, self.flags)
    }
}

impl<'a> de::Deserialize<'a> for PinConfig<'a> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl<'a> de::Deserialize<'a> for Option<PinConfig<'a>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        match Option::deserialize_over(source, whitespace)? {
            None => return Ok(None),
            Some(Literal(b'{')) => {
                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Literal::<&[u8]>(b"name") = Literal::deserialize_over(source, whitespace)? else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Some(b':') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let Quoted(name) = Quoted::deserialize_over(source, whitespace)?;

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Some(b',') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Literal::<&[u8]>(b"flags") = Literal::deserialize_over(source, whitespace)? else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                let Some(b':') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                if whitespace {
                    walk_whitespace(source);
                }

                let flags = PinFlags::deserialize_over(source, whitespace)?;

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
                let Some(b'}') = source.next() else {
                    *source = undo;
                    return Err(());
                };

                Ok(Some(PinConfig { name, flags }))
            },
            Some(_) => {
                *source = undo;
                Err(())
            },
        }
    }
}