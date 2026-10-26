use crate::bead::PinMode;

#[derive(Clone, Copy)]
pub enum Request<'a> {
    Ping { },
    GetConfig { },
    GetPinPower { pin: &'a str },
    GetPinMode { pin: &'a str },
    GetPinFlags { pin: &'a str },
    SetPinPower { pin: &'a str, power: u8 },
    SetPinMode { pin: &'a str, mode: PinMode },
}

pub fn serialize(response: &Request<'_>, buffer: &mut impl Extend<u8>) {
    match response {
        Request::Ping { } => {
            buffer.extend([0]);
        },
        Request::GetConfig { } => {
            buffer.extend([1]);
        },
        Request::GetPinPower { pin } => {
            buffer.extend([2]);
            buffer.extend([pin.len() as u8]);
            buffer.extend(pin.bytes());
        },
        Request::GetPinMode { pin } => {
            buffer.extend([3]);
            buffer.extend([pin.len() as u8]);
            buffer.extend(pin.bytes());
        },
        Request::GetPinFlags { pin } => {
            buffer.extend([4]);
            buffer.extend([pin.len() as u8]);
            buffer.extend(pin.bytes());
        },
        Request::SetPinPower { pin, power } => {
            buffer.extend([5]);
            buffer.extend([pin.len() as u8]);
            buffer.extend(pin.bytes());
            buffer.extend([*power]);
        },
        Request::SetPinMode { pin, mode } => {
            buffer.extend([5]);
            buffer.extend([pin.len() as u8]);
            buffer.extend(pin.bytes());
            buffer.extend([u8::from(*mode)]);
        },
    }
}
