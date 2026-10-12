use std::borrow::Borrow;
use std::collections::{HashMap, hash_map};
use std::hash::Hash;
use std::io::ErrorKind;
use std::time::Duration;

use tokio::io::{self, AsyncWriteExt, BufReader, BufWriter};
use tokio::time::timeout;
use tokio::sync::{Mutex, OnceCell, mpsc, oneshot};
use tokio_serial::{SerialPortBuilderExt, SerialPortType, SerialStream, UsbPortInfo};

use crate::bead::packet::outgoing;
use crate::packet;
use crate::packet::{Deserialize, Serialize};

use super::{Config, PinMode};
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

struct SerialHandlerConnection
{
    config: OnceCell<Config>,
    queue: mpsc::UnboundedSender<QueueAction>,
}

pub struct SerialHandler
{
    connected: Mutex<HashMap<String, SerialHandlerConnection>>,
}

impl SerialHandler
{
    pub fn new() -> Self
    {
        Self
        {
            connected: Mutex::new(HashMap::new()),
        }
    }

    pub async fn ping<Q: ?Sized>(&self, bead_name: &Q)
        -> Result<(), Option<PingError>>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        let lock = self.connected.lock().await;

        let Some(connection) = lock.get(bead_name)
        else { return Err(None) };

        let (tx, rx) = oneshot::channel();
        if connection.queue.send(
            QueueAction::Ping { complete: tx })
            .is_err() { return Err(None) }

        drop(lock);

        match rx.await
        {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(x)) => Err(Some(x)),
            Err(_) => Err(None),
        }
    }

    pub async fn config<Q: ?Sized>(&self, bead_name: &Q)
        -> Result<Option<Config>, ()>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        let lock = self.connected.lock().await;

        let Some(connection) = lock.get(bead_name)
        else { return Err(()) };

        Ok(connection.config.get().cloned())
    }

    pub async fn get_pin_power<Q: ?Sized>(&self, bead_name: &Q, pin_name: impl Into<String>)
        -> Result<f32, Option<GetPinPowerError>>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        let lock = self.connected.lock().await;

        let Some(connection) = lock.get(bead_name)
        else { return Err(None) };

        let (tx, rx) = oneshot::channel();
        if connection.queue.send(
            QueueAction::GetPinPower { pin_name: pin_name.into(), complete: tx })
            .is_err() { return Err(None) }

        drop(lock);

        match rx.await
        {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(x)) => Err(Some(x)),
            Err(_) => Err(None),
        }
    }

    pub async fn get_pin_mode<Q: ?Sized>(&self, bead_name: &Q, pin_name: impl Into<String>)
        -> Result<PinMode, Option<GetPinModeError>>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        let lock = self.connected.lock().await;

        let Some(connection) = lock.get(bead_name)
        else { return Err(None) };

        let (tx, rx) = oneshot::channel();
        if connection.queue.send(
            QueueAction::GetPinMode { pin_name: pin_name.into(), complete: tx })
            .is_err() { return Err(None) }

        drop(lock);

        match rx.await
        {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(x)) => Err(Some(x)),
            Err(_) => Err(None),
        }
    }

    pub async fn set_pin_power<Q: ?Sized>(&self, bead_name: &Q, pin_name: impl Into<String>, power: f32)
        -> Result<(), Option<SetPinPowerError>>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        let lock = self.connected.lock().await;

        let Some(connection) = lock.get(bead_name)
        else { return Err(None) };

        let (tx, rx) = oneshot::channel();
        if connection.queue.send(
            QueueAction::SetPinPower { pin_name: pin_name.into(), power, complete: tx })
            .is_err() { return Err(None) }

        drop(lock);

        match rx.await
        {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(x)) => Err(Some(x)),
            Err(_) => Err(None),
        }
    }

    pub async fn set_pin_mode<Q: ?Sized>(&self, bead_name: &Q, pin_name: impl Into<String>, mode: PinMode)
        -> Result<(), Option<SetPinModeError>>
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        let lock = self.connected.lock().await;

        let Some(connection) = lock.get(bead_name)
        else { return Err(None) };

        let (tx, rx) = oneshot::channel();
        if connection.queue.send(
            QueueAction::SetPinMode { pin_name: pin_name.into(), mode, complete: tx })
            .is_err() { return Err(None) }

        drop(lock);

        match rx.await
        {
            Ok(Ok(x)) => Ok(x),
            Ok(Err(x)) => Err(Some(x)),
            Err(_) => Err(None),
        }
    }

    pub async fn update(&self, mut new: impl FnMut(String, oneshot::Receiver<()>))
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
                    match tokio_serial::new(info.port_name, 9600).open_native_async()
                    {
                        Err(_) => (),
                        Ok(stream) => match connected.entry(serial_number.clone())
                        {
                            hash_map::Entry::Occupied(_) => (),
                            hash_map::Entry::Vacant(entry) =>
                            {
                                let (tx, rx) = mpsc::unbounded_channel();
                                let (close_tx, close_rx) = oneshot::channel();
                                entry.insert(SerialHandlerConnection
                                {
                                    config: <_>::default(),
                                    queue: tx,
                                });
                                new(serial_number, close_rx);
                                tokio::spawn(async move
                                {
                                    handle(rx, stream).await;
                                    _ = close_tx.send(());
                                });
                            },
                        },
                    }
                }
            }
        }
    }
}

const ANALOG_READ_MAX: u16 = 1023;
const ANALOG_WRITE_MAX: u16 = 255;

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
                        Ok(Ok(incoming::Packet::Pong)) => break Ok(()),
                        Ok(Err(packet::Error::Reset(_))) => continue,
                        Ok(Err(packet::Error::IO(x))) => match x.kind()
                        {
                            ErrorKind::UnexpectedEof => break Err(BeadClosed.into()),
                            _ => return Err(x),
                        },
                        _ =>
                        {
                            outgoing::Packet::Ping.write_to(&mut stream).await?;
                            stream.flush().await?;
                        },
                    }
                });
            },
            QueueAction::GetPinPower { pin_name, complete } =>
            {
                if let Some(pin) = config.pins.iter().position(|pin| pin.name == pin_name)
                {
                    let pin = pin as u8;

                    outgoing::Packet::GetPinPower { pin }.write_to(&mut stream).await?;
                    stream.flush().await?;

                    _ = complete.send(loop
                    {
                        match timeout(
                            Duration::from_secs_f32(0.5),
                            incoming::Packet::read_from(&mut stream)).await
                        {
                            Ok(Ok(incoming::Packet::GetPinPowerResponse { pin: response_pin, power }))
                            if response_pin == pin => break Ok(power.min(ANALOG_READ_MAX) as f32 / ANALOG_READ_MAX as f32),
                            Ok(Err(packet::Error::Reset(_))) => continue,
                            Ok(Err(packet::Error::IO(x))) => match x.kind()
                            {
                                ErrorKind::UnexpectedEof => break Err(BeadClosed.into()),
                                _ => return Err(x),
                            },
                            _ =>
                            {
                                outgoing::Packet::GetPinPower { pin }.write_to(&mut stream).await?;
                                stream.flush().await?;
                            },
                        }
                    });
                }
                else
                {
                    _ = complete.send(Err(GetPinPowerError::InvalidBeadPinName));
                }
            },
            QueueAction::GetPinMode { pin_name, complete } =>
            {
                if let Some(pin) = config.pins.iter().position(|pin| pin.name == pin_name)
                {
                    let pin = pin as u8;

                    outgoing::Packet::GetPinMode { pin }.write_to(&mut stream).await?;
                    stream.flush().await?;

                    _ = complete.send(loop
                    {
                        match timeout(
                            Duration::from_secs_f32(0.5),
                            incoming::Packet::read_from(&mut stream)).await
                        {
                            Ok(Ok(incoming::Packet::GetPinModeResponse { pin: response_pin, mode }))
                            if response_pin == pin => break Ok(mode),
                            Ok(Err(packet::Error::Reset(_))) => continue,
                            Ok(Err(packet::Error::IO(x))) => match x.kind()
                            {
                                ErrorKind::UnexpectedEof => break Err(BeadClosed.into()),
                                _ => return Err(x),
                            },
                            _ =>
                            {
                                outgoing::Packet::GetPinMode { pin }.write_to(&mut stream).await?;
                                stream.flush().await?;
                            },
                        }
                    });
                }
                else
                {
                    _ = complete.send(Err(GetPinModeError::InvalidBeadPinName));
                }
            },
            QueueAction::SetPinPower { pin_name, power, complete } =>
            {
                if let Some(pin) = config.pins.iter().position(|pin| pin.name == pin_name)
                {
                    let pin = pin as u8;
                    let power = ((power * (ANALOG_WRITE_MAX as f32 + 1.0)).floor() as u16).min(ANALOG_WRITE_MAX);

                    outgoing::Packet::SetPinPower { pin, power }.write_to(&mut stream).await?;
                    stream.flush().await?;

                    _ = complete.send(loop
                    {
                        match timeout(
                            Duration::from_secs_f32(0.5),
                            incoming::Packet::read_from(&mut stream)).await
                        {
                            Ok(Ok(incoming::Packet::SetPinPowerResponse { pin: response_pin }))
                            if response_pin == pin => break Ok(()),
                            Ok(Err(packet::Error::Reset(_))) => continue,
                            Ok(Err(packet::Error::IO(x))) => match x.kind()
                            {
                                ErrorKind::UnexpectedEof => break Err(BeadClosed.into()),
                                _ => return Err(x),
                            },
                            _ =>
                            {
                                outgoing::Packet::SetPinPower { pin, power }.write_to(&mut stream).await?;
                                stream.flush().await?;
                            },
                        }
                    });
                }
                else
                {
                    _ = complete.send(Err(SetPinPowerError::InvalidBeadPinName));
                }
            },
            QueueAction::SetPinMode { pin_name, mode, complete } =>
            {
                if let Some(pin) = config.pins.iter().position(|pin| pin.name == pin_name)
                {
                    let pin = pin as u8;

                    outgoing::Packet::SetPinMode { pin, mode }.write_to(&mut stream).await?;
                    stream.flush().await?;

                    _ = complete.send(loop
                    {
                        match timeout(
                            Duration::from_secs_f32(0.5),
                            incoming::Packet::read_from(&mut stream)).await
                        {
                            Ok(Ok(incoming::Packet::SetPinModeResponse { pin: response_pin }))
                            if response_pin == pin => break Ok(()),
                            Ok(Err(packet::Error::Reset(_))) => continue,
                            Ok(Err(packet::Error::IO(x))) => match x.kind()
                            {
                                ErrorKind::UnexpectedEof => break Err(BeadClosed.into()),
                                _ => return Err(x),
                            },
                            _ =>
                            {
                                outgoing::Packet::SetPinMode { pin, mode }.write_to(&mut stream).await?;
                                stream.flush().await?;
                            },
                        }
                    });
                }
                else
                {
                    _ = complete.send(Err(SetPinModeError::InvalidBeadPinName));
                }
            },
        }
    }

    Ok(())
}