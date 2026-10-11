use std::collections::{HashMap, hash_map};
use std::convert::Infallible;
use std::future::{Ready, pending};
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tokio::io::{self, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::time::{Interval, Timeout, interval, timeout};
use tokio::{join, select};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_serial::SerialPortType::UsbPort;
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType, SerialStream, UsbPortInfo};

use crate::bead::packet::outgoing;
use crate::{packet, shutdown_signal};
use crate::packet::{Deserialize, Reset, Serialize};

use super::{Config, InvalidPinModeOrUTF8, PinMode};
use super::packet::incoming;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeadClosed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingError
{
    BeadClosed,
}

impl From<BeadClosed> for PingError
{
    fn from(_: BeadClosed) -> Self { Self::BeadClosed }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WholeConfigError
{
    BeadClosed,
}

impl From<BeadClosed> for WholeConfigError
{
    fn from(_: BeadClosed) -> Self { Self::BeadClosed }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetPinPowerError
{
    BeadClosed,
    InvalidBeadPinName,
}

impl From<BeadClosed> for GetPinPowerError
{
    fn from(_: BeadClosed) -> Self { Self::BeadClosed }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetPinModeError
{
    BeadClosed,
    InvalidBeadPinName,
}

impl From<BeadClosed> for GetPinModeError
{
    fn from(_: BeadClosed) -> Self { Self::BeadClosed }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetPinPowerError
{
    BeadClosed,
    InvalidBeadPinName,
    InvalidWriteToInput,
}

impl From<BeadClosed> for SetPinPowerError
{
    fn from(_: BeadClosed) -> Self { Self::BeadClosed }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetPinModeError
{
    BeadClosed,
    InvalidBeadPinName,
    InvalidUnsupportedMode,
}

impl From<BeadClosed> for SetPinModeError
{
    fn from(_: BeadClosed) -> Self { Self::BeadClosed }
}

#[derive(Debug)]
pub enum QueueAction
{
    Ping
    {
        complete: oneshot::Sender<Result<(), PingError>>,
    },
    WholeConfig
    {
        complete: oneshot::Sender<Result<Config, WholeConfigError>>,
    },
    GetPinPower
    {
        pin_name: String,
        complete: oneshot::Sender<Result<f32, GetPinPowerError>>,
    },
    GetPinMode
    {
        pin_name: String,
        complete: oneshot::Sender<Result<PinMode, GetPinModeError>>,
    },
    SetPinPower
    {
        pin_name: String,
        power: f32,
        complete: oneshot::Sender<Result<(), SetPinPowerError>>,
    },
    SetPinMode
    {
        pin_name: String,
        mode: PinMode,
        complete: oneshot::Sender<Result<(), SetPinModeError>>,
    },
}

pub struct SerialHandler
{
    connected: Mutex<HashMap<String, mpsc::UnboundedSender<QueueAction>>>,
}

impl SerialHandler
{
    async fn update(&self)
    {
        let mut connected = self.connected.lock().await;

        for info in tokio_serial::available_ports().unwrap()
        {
            if let SerialPortType::UsbPort(UsbPortInfo
            {
                manufacturer: Some(manufacturer),
                serial_number: Some(serial_number),
                ..
            }) = info.port_type
            {
                if manufacturer.contains("arduino") || manufacturer.contains("Arduino")
                {
                    match tokio_serial::new(&info.port_name, 9600).open_native_async()
                    {
                        Err(_) => (),
                        Ok(stream) => match connected.entry(info.port_name)
                        {
                            hash_map::Entry::Occupied(_) => (),
                            hash_map::Entry::Vacant(entry) =>
                            {
                                let (tx, rx) = mpsc::unbounded_channel();
                                entry.insert(tx);
                                tokio::spawn(handle(rx, stream));
                            },
                        },
                    }
                }
            }
        }
    }
}

async fn handle(mut rx: mpsc::UnboundedReceiver<QueueAction>, stream: SerialStream) -> io::Result<()>
{
    let mut stream = BufWriter::new(BufReader::new(stream));

    let config = loop
    {
        match timeout(
            Duration::from_secs_f32(0.5),
            incoming::Packet::read_from(&mut stream)).await
        {
            Ok(Ok(incoming::Packet::Config(x))) => break x,
            Ok(Err(packet::Error::IO(x))) => return Err(x),
            Ok(Err(packet::Error::Reset(_))) => continue,
            _ =>
            {
                outgoing::Packet::WholeConfig.write_to(&mut stream).await?;
                stream.flush().await?;
            },
        }
    };

    while let Some(recv) = rx.recv().await
    {
        match recv
        {
            QueueAction::Ping { complete } =>
            {
                outgoing::Packet::Ping.write_to(&mut stream).await?;
                stream.flush().await?;

                _ = complete.send(loop
                {
                    match timeout(
                        Duration::from_secs_f32(0.5),
                        incoming::Packet::read_from(&mut stream)).await
                    {
                        Ok(Ok(incoming::Packet::Pong)) =>
                        {
                            break Ok(());
                        },
                        Ok(Err(packet::Error::IO(x))) if x.kind() == ErrorKind::UnexpectedEof =>
                        {
                            break Err(BeadClosed.into());
                        },
                        Ok(Err(packet::Error::IO(x))) =>
                        {
                            return Err(x);
                        },
                        Ok(Err(packet::Error::Reset(_))) => continue,
                        _ =>
                        {
                            outgoing::Packet::Ping.write_to(&mut stream).await?;
                            stream.flush().await?;
                        },
                    }
                });
            },
            QueueAction::WholeConfig { complete } =>
            {
                _ = complete.send(Ok(config.clone()));
            },
            QueueAction::GetPinPower { pin_name, complete } =>
            {
                outgoing::Packet::Ping.write_to(&mut stream).await?;
                stream.flush().await?;

                _ = complete.send(loop
                {
                    match timeout(
                        Duration::from_secs_f32(0.5),
                        incoming::Packet::read_from(&mut stream)).await
                    {
                        Ok(Ok(incoming::Packet::GetPinPowerResponse { pin, power })) =>
                        {
                            break Ok(());
                        },
                        Ok(Err(packet::Error::IO(x))) if x.kind() == ErrorKind::UnexpectedEof =>
                        {
                            break Err(BeadClosed.into());
                        },
                        Ok(Err(packet::Error::IO(x))) =>
                        {
                            return Err(x);
                        },
                        Ok(Err(packet::Error::Reset(_))) => continue,
                        _ =>
                        {
                            outgoing::Packet::Ping.write_to(&mut stream).await?;
                            stream.flush().await?;
                        },
                    }
                });
            },
            QueueAction::GetPinMode { pin_name, complete } =>
            {

            },
            QueueAction::SetPinPower { pin_name, power, complete } =>
            {

            },
            QueueAction::SetPinMode { pin_name, mode, complete } =>
            {

            },
        }
    }

    Ok(())
}