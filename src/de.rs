use std::borrow::Cow;
use std::{fmt, slice};

use crate::im::Ash;

pub const fn is_literal_char(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_')
}

pub fn walk_whitespace(source: &mut slice::Iter<'_, u8>) {
    loop {
        let undo = source.clone();
        if !source.next().is_none_or(u8::is_ascii_whitespace) {
            *source = undo;
            break;
        }
    }
}

pub trait Deserialize<'a>: Sized {
    type Error: Default;

    fn deserialize(source: impl IntoIterator<IntoIter = slice::Iter<'a, u8>>)
        -> Result<Self, (usize, Self::Error)> {
        let mut iter = source.into_iter();
        let source = iter.clone();
        let value = Self::deserialize_over(&mut iter, true).map_err(|x| (source.len() - iter.len(), x))?;
        for byte in iter {
            if !u8::is_ascii_whitespace(byte) {
                return Err(<_>::default());
            }
        }
        Ok(value)
    }

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error>;
}

macro_rules! impl_int {
    ($($ty:ident: $size:literal),*$(,)?) => {
        $(
            impl Deserialize<'_> for Option<$ty> {
                type Error = ();

                fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
                    if whitespace {
                        walk_whitespace(source);
                    }

                    let undo_all = source.clone();
                    let mut buffer = [0u8; $size];
                    let mut i = 0;
                    let mut is_first = true;

                    loop {
                        let undo = source.clone();
                        match source.next() {
                            Some(&byte @ (b'0'..=b'9' | b'-')) if byte != b'-' || is_first => {
                                if i >= buffer.len() {
                                    return Err(());
                                }
                                is_first = false;
                                buffer[i] = byte;
                                i += 1;
                            },
                            Some(&byte) if is_first || is_literal_char(byte) => {
                                return Ok(None);
                            },
                            _ => {
                                *source = undo;
                                break;
                            },
                        }
                    }

                    match core::str::from_utf8(&buffer[..i]).unwrap().parse::<$ty>() {
                        Ok(value) => Ok(Some(value)),
                        Err(_) => {
                            *source = undo_all;
                            Err(())
                        },
                    }
                }
            }

            impl Deserialize<'_> for $ty {
                type Error = ();

                fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
                    Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
                }
            }
        )*
    };
}

impl_int!(u8: 3, u16: 5, u32: 10, u64: 20, u128: 39, i8: 4, i16: 6, i32: 11, i64: 20, i128: 40, isize: 20, usize: 20);

impl<'a> Deserialize<'a> for f32 {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl<'a> Deserialize<'a> for Option<f32> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo_all = source.clone();
        let mut buffer = [0u8; 64];
        let mut i = 0;
        let mut is_first = true;

        loop {
            let undo = source.clone();
            match source.next() {
                Some(&byte @ (b'0'..=b'9' | b'-' | b'.' | b'e' | b'E')) if byte != b'-' || is_first => {
                    if i >= buffer.len() {
                        return Err(());
                    }
                    is_first = false;
                    buffer[i] = byte;
                    i += 1;
                },
                Some(&byte) if is_first || is_literal_char(byte) => {
                    return Ok(None);
                },
                _ => {
                    *source = undo;
                    break;
                },
            }
        }

        match core::str::from_utf8(&buffer[..i]).unwrap().parse::<f32>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                *source = undo_all;
                Err(())
            },
        }
    }
}

impl<'a> Deserialize<'a> for bool {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl<'a> Deserialize<'a> for Option<bool> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        let undo = source.clone();
        match Option::deserialize_over(source, whitespace)? {
            None => Ok(None),
            Some(Literal::<&[u8]>(b"true")) => Ok(Some(true)),
            Some(Literal(b"false")) => Ok(Some(false)),
            Some(_) => {
                *source = undo;
                Err(())
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Literal<T>(pub T);

impl<T: fmt::Display> fmt::Display for Literal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl<'a> Deserialize<'a> for Literal<Vec<u8>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .map(|Literal(x)| Literal(<&[u8]>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Literal<Cow<'a, [u8]>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .map(|Literal(x)| Literal(<&[u8]>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Literal<Ash<'a, [u8]>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .map(|Literal(x)| Literal(<&[u8]>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Literal<&'a [u8]> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<Vec<u8>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Literal(x)| Literal(<&[u8]>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<Cow<'a, [u8]>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Literal(x)| Literal(<&[u8]>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<Ash<'a, [u8]>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Literal(x)| Literal(<&[u8]>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<&'a [u8]>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        match source.next() {
            Some(&byte) if is_literal_char(byte) => {
                let value = undo.as_slice();
                let mut count = 1;
                loop {
                    let undo = source.clone();
                    match source.next() {
                        Some(&byte) if is_literal_char(byte) => {
                            count += 1;
                        },
                        _ => {
                            *source = undo;
                            break;
                        },
                    }
                }
                Ok(Some(Literal(&value[..count])))
            },
            _ => {
                *source = undo;
                Ok(None)
            },
        }
    }
}

impl Deserialize<'_> for Literal<String> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .map(|Literal(x)| Literal(<&str>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Literal<Cow<'a, str>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .map(|Literal(x)| Literal(<&str>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Literal<Ash<'a, str>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .map(|Literal(x)| Literal(<&str>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Literal<&'a str> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Literal::deserialize_over(source, whitespace)
            .and_then(|Literal(x)| str::from_utf8(x).map_err(drop).map(Literal))
    }
}

impl Deserialize<'_> for Option<Literal<String>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Literal(x)| Literal(<&str>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<Cow<'a, str>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Literal(x)| Literal(<&str>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<Ash<'a, str>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Literal(x)| Literal(<&str>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Literal<&'a str>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .and_then(|x| x
                .map(|Literal(x)| str::from_utf8(x).map_err(drop).map(Literal))
                .transpose())
    }
}

impl Deserialize<'_> for Literal<u8> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl Deserialize<'_> for Option<Literal<u8>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        Ok(source.next().copied().map(Literal))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quoted<T>(pub T);

impl<T: fmt::Display> fmt::Display for Quoted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl<'a> Deserialize<'a> for Quoted<Vec<u8>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .map(|Quoted(x)| Quoted(<&[u8]>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Quoted<Cow<'a, [u8]>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .map(|Quoted(x)| Quoted(<&[u8]>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Quoted<Ash<'a, [u8]>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .map(|Quoted(x)| Quoted(<&[u8]>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Quoted<&'a [u8]> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or(()))
    }
}

impl Deserialize<'_> for Option<Quoted<Vec<u8>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Quoted(x)| Quoted(<&[u8]>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Quoted<Cow<'a, [u8]>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Quoted(x)| Quoted(<&[u8]>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Quoted<Ash<'a, [u8]>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Quoted(x)| Quoted(<&[u8]>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Quoted<&'a [u8]>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        match source.next() {
            Some(&open @ (b'"' | b'\'')) => {
                let value = source.as_slice();
                let mut count = 0;
                loop {
                    let undo = source.clone();
                    match source.next() {
                        Some(&byte) if byte == open => {
                            break;
                        },
                        Some(_) => {
                            count += 1;
                        },
                        None => {
                            *source = undo;
                            return Err(());
                        },
                    }
                }
                Ok(Some(Quoted(&value[..count])))
            },
            _ => {
                *source = undo;
                Ok(None)
            },
        }
    }
}

impl Deserialize<'_> for Quoted<String> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .map(|Quoted(x)| Quoted(<&str>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Quoted<Cow<'a, str>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .map(|Quoted(x)| Quoted(<&str>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Quoted<Ash<'a, str>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .map(|Quoted(x)| Quoted(<&str>::into(x)))
    }
}

impl<'a> Deserialize<'a> for Quoted<&'a str> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Quoted::deserialize_over(source, whitespace)
            .and_then(|Quoted(x)| str::from_utf8(x).map_err(drop).map(Quoted))
    }
}

impl Deserialize<'_> for Option<Quoted<String>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'_, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Quoted(x)| Quoted(<&str>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Quoted<Cow<'a, str>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Quoted(x)| Quoted(<&str>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Quoted<Ash<'a, str>>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .map(|x| x
                .map(|Quoted(x)| Quoted(<&str>::into(x))))
    }
}

impl<'a> Deserialize<'a> for Option<Quoted<&'a str>> {
    type Error = ();

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace)
            .and_then(|x| x
                .map(|Quoted(x)| str::from_utf8(x).map_err(drop).map(Quoted))
                .transpose())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Field<T>(pub T);

impl<T: fmt::Display> fmt::Display for Field<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ".{}", self.0)
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Field<T> {
    type Error = T::Error;

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or_else(<_>::default))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<Field<T>> {
    type Error = T::Error;

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        match source.next() {
            Some(b'.') => {
                T::deserialize_over(source, whitespace).map(|x| Some(Field(x)))
            },
            Some(b'[') => {
                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                match source.next() {
                    Some(&open @ (b'"' | b'\'')) => {
                        match T::deserialize_over(source, whitespace) {
                            Ok(value) => {
                                let undo = source.clone();
                                match source.next() {
                                    Some(&byte) if byte == open => { },
                                    _ => {
                                        *source = undo;
                                        return Err(<_>::default());
                                    },
                                }

                                if whitespace {
                                    walk_whitespace(source);
                                }

                                let undo = source.clone();
                                match source.next() {
                                    Some(b']') => { },
                                    _ => {
                                        *source = undo;
                                        return Err(<_>::default());
                                    },
                                }

                                Ok(Some(Field(value)))
                            },
                            Err(x) => {
                                *source = undo;
                                Err(x)
                            },
                        }
                    },
                    _ => {
                        *source = undo;
                        Err(<_>::default())
                    },
                }
            },
            _ => {
                *source = undo;
                Ok(None)
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array<T>(pub T);

impl<T> fmt::Display for Array<T>
where
    for<'a> &'a T: IntoIterator<Item: fmt::Display> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for elm in &self.0 {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{}", elm)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Array<Vec<T>> {
    type Error = T::Error;

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or_else(<_>::default))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<Array<Vec<T>>> {
    type Error = T::Error;

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        if whitespace {
            walk_whitespace(source);
        }

        let undo = source.clone();
        match source.next() {
            Some(b'[') => {
                let mut result = Vec::new();

                if whitespace {
                    walk_whitespace(source);
                }

                let undo = source.clone();
                match source.next() {
                    Some(b']') => return Ok(Some(Array(result))),
                    _ => {
                        *source = undo;
                    },
                }

                loop {
                    result.push(T::deserialize_over(source, whitespace)?);

                    if whitespace {
                        walk_whitespace(source);
                    }

                    let undo = source.clone();
                    match source.next() {
                        Some(b']') => break,
                        Some(b',') => {
                            if whitespace {
                                walk_whitespace(source);
                            }

                            let undo = source.clone();
                            match source.next() {
                                Some(b']') => break,
                                _ => {
                                    *source = undo;
                                },
                            }
                        },
                        _ => {
                            *source = undo;
                        },
                    }
                }

                Ok(Some(Array(result)))
            },
            _ => {
                *source = undo;
                Ok(None)
            },
        }
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Array<Cow<'_, [T]>>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    type Error = T::Error;

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).and_then(|x| x.ok_or_else(<_>::default))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<Array<Cow<'_, [T]>>>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    type Error = T::Error;

    fn deserialize_over(source: &mut slice::Iter<'a, u8>, whitespace: bool) -> Result<Self, Self::Error> {
        Option::deserialize_over(source, whitespace).map(|x| x.map(|Array(x)| Array(Cow::Owned(x))))
    }
}