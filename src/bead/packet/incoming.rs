use crate::bead::{Config, InvalidPinModeOrUTF8, PinMode};
use crate::packet::{self, Deserialize};

use tokio::io::AsyncRead;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet
{
    Pong,
    Config(Config),
    GetPinPowerResponse { pin: u8, power: u16 },
    GetPinModeResponse { pin: u8, mode: PinMode },
    SetPinPowerResponse { pin: u8, power: u16 },
    SetPinModeResponse { pin: u8, mode: PinMode },
    PinListen { pin: u8, power: u16 },
    InvalidPinMode { byte: u8 },
    InvalidPinId { byte: u8 },
    InvalidPacketId { byte: u8 },
    InvalidWriteToInput { pin: u8, power: u16 },
    InvalidUnsupportedMode { pin: u8, mode: PinMode },
    InvalidEscape { byte: u8 },
}

impl Deserialize for Packet
{
    type Error = InvalidPinModeOrUTF8;

    async fn read_from(reader: &mut (impl AsyncRead + Unpin)) -> Result<Self, packet::Error<Self::Error>>
    {
        match u8::read_from(reader).await.map_err(packet::Error::upcast)?
        {
            0 => Ok(Packet::Pong),
            1 => Ok(Packet::Config(Config::read_from(reader).await.map_err(packet::Error::coerce)?)),
            2 => Ok(Packet::GetPinPowerResponse
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                power: u16::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            3 => Ok(Packet::GetPinModeResponse
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                mode: PinMode::read_from(reader).await?,
            }),
            4 => Ok(Packet::SetPinPowerResponse
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                power: u16::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            5 => Ok(Packet::SetPinModeResponse
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                mode: PinMode::read_from(reader).await?,
            }),
            6 => Ok(Packet::PinListen
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                power: u16::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            101 => Ok(Packet::InvalidPinMode
            {
                byte: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            102 => Ok(Packet::InvalidPinId
            {
                byte: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            103 => Ok(Packet::InvalidWriteToInput
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                power: u16::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            104 => Ok(Packet::InvalidUnsupportedMode
            {
                pin: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
                mode: PinMode::read_from(reader).await?,
            }),
            105 => Ok(Packet::InvalidEscape
            {
                byte: u8::read_from(reader).await.map_err(packet::Error::upcast)?,
            }),
            x => Ok(Packet::InvalidPacketId { byte: x }),
        }
    }
}