use std::slice;

use crate::bead::{PinFlags, PinMode};

#[derive(Clone, Debug)]
pub enum Response<'a> {
    Pong { },
    Config { model: &'a str, pins: PinConfigIter<'a> },
    GetPinPowerResponse { pin: &'a str, power: u16 },
    GetPinModeResponse { pin: &'a str, mode: PinMode },
    GetPinFlagsResponse { pin: &'a str, flags: PinFlags },
    SetPinPowerResponse { pin: &'a str, power: u8 },
    SetPinModeResponse { pin: &'a str, mode: PinMode },
    PinChanged { pin: &'a str, power: u16 },
    PacketError { message: &'a str },
    DenyError { message: &'a str },
    Panic { line: u32, column: u32, file: &'a str },
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Error {
    UnexpectedEndOfPacket,
    UnexpectedExcessData,
    UnsupportedVersion,
    InvalidPinMode,
    InvalidUTF8,
    InvalidPacketId,
}

pub fn deserialize(payload: &[u8]) -> Result<Response<'_>, Error> {
    match payload {
        [0, rest @ ..] => {
            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::Pong { })
        },
        [1, rest @ ..] => {
            let [version, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            if *version != 1 {
                return Err(Error::UnsupportedVersion);
            }

            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((model, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(model) = str::from_utf8(model) else {
                return Err(Error::InvalidUTF8);
            };

            let (pins, rest) = PinConfigIter::walk(rest)?;

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::Config { model, pins })
        },
        [2, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((pin, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(pin) = str::from_utf8(pin) else {
                return Err(Error::InvalidUTF8);
            };

            let [byte0, byte1, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let power = u16::from_be_bytes([*byte0, *byte1]);

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::GetPinPowerResponse { pin, power })
        },
        [3, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((pin, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(pin) = str::from_utf8(pin) else {
                return Err(Error::InvalidUTF8);
            };

            let [mode, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(mode) = PinMode::try_from(*mode) else {
                return Err(Error::InvalidPinMode);
            };

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::GetPinModeResponse { pin, mode })
        },
        [4, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((pin, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(pin) = str::from_utf8(pin) else {
                return Err(Error::InvalidUTF8);
            };

            let [flags, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some(flags) = PinFlags::from_bits(*flags) else {
                return Err(Error::InvalidPinMode);
            };

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::GetPinFlagsResponse { pin, flags })
        },
        [5, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((pin, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(pin) = str::from_utf8(pin) else {
                return Err(Error::InvalidUTF8);
            };

            let [power, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let power = *power;

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::SetPinPowerResponse { pin, power })
        },
        [6, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((pin, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(pin) = str::from_utf8(pin) else {
                return Err(Error::InvalidUTF8);
            };

            let [mode, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(mode) = PinMode::try_from(*mode) else {
                return Err(Error::InvalidPinMode);
            };

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::SetPinModeResponse { pin, mode })
        },
        [7, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((pin, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(pin) = str::from_utf8(pin) else {
                return Err(Error::InvalidUTF8);
            };

            let [byte0, byte1, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let power = u16::from_be_bytes([*byte0, *byte1]);

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::PinChanged { pin, power })
        },
        [201, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((message, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(message) = str::from_utf8(message) else {
                return Err(Error::InvalidUTF8);
            };

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::PacketError { message })
        },
        [202, rest @ ..] => {
            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((message, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(message) = str::from_utf8(message) else {
                return Err(Error::InvalidUTF8);
            };

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::PacketError { message })
        },
        [255, rest @ ..] => {
            let [byte0, byte1, byte2, byte3, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let line = u32::from_be_bytes([*byte0, *byte1, *byte2, *byte3]);

            let [byte0, byte1, byte2, byte3, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let column = u32::from_be_bytes([*byte0, *byte1, *byte2, *byte3]);

            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((file, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(file) = str::from_utf8(file) else {
                return Err(Error::InvalidUTF8);
            };

            if !rest.is_empty() {
                return Err(Error::UnexpectedExcessData);
            };

            Ok(Response::Panic { line, column, file })
        },
        [_, ..] => {
            Err(Error::InvalidPacketId)
        },
        [] => {
            Err(Error::UnexpectedEndOfPacket)
        },
    }
}

#[derive(Clone, Debug)]
pub struct PinConfigIter<'a> {
    inner: slice::Iter<'a, u8>,
}

impl<'a> PinConfigIter<'a> {
    fn walk(rest: &'a [u8]) -> Result<(Self, &'a [u8]), Error> {
        let [len, rest @ ..] = rest else {
            return Err(Error::UnexpectedEndOfPacket);
        };

        let mut outer_rest = rest;

        for _ in 0..*len {
            let rest = outer_rest;

            let [len, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Some((name, rest)) = rest.split_at_checked(*len as usize) else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            let Ok(_) = str::from_utf8(name) else {
                return Err(Error::InvalidUTF8);
            };

            let [_, rest @ ..] = rest else {
                return Err(Error::UnexpectedEndOfPacket);
            };

            outer_rest = rest;
        }

        let iter_inner = &rest[..rest.len() - outer_rest.len()];
        let rest = outer_rest;

        Ok((PinConfigIter { inner: iter_inner.iter() }, rest))
    }
}

impl<'a> Iterator for PinConfigIter<'a> {
    type Item = (&'a str, PinFlags);

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.inner.as_slice();

        let [len, rest @ ..] = rest else {
            unreachable!();
        };

        let Some((name, rest)) = rest.split_at_checked(*len as usize) else {
            unreachable!();
        };

        let [flags, rest @ ..] = rest else {
            unreachable!();
        };

        let name = str::from_utf8(name).unwrap();
        let flags = PinFlags::from_bits_truncate(*flags);

        self.inner = rest.iter();

        Some((name, flags))
    }
}
