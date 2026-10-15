use std::borrow::Borrow;
use std::collections::{HashMap, hash_map};
use std::future::{pending, poll_fn};
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use pin_vec::PinEach;
use tokio::io::{self, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::select;
use tokio::time::timeout;
use tokio::sync::{Mutex, watch};
use tokio_serial::{SerialPortBuilderExt, SerialPortType, SerialStream, UsbPortInfo};

use crate::bead::packet::outgoing;
use crate::packet;
use crate::packet::{Deserialize, Serialize};
use crate::log::*;

use super::{Config, ConfigPin, PinFlags, PinMode};
use super::packet::incoming;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingError
{
    BeadClosed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WholeConfigError
{
    BeadClosed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetPinPowerError
{
    BeadClosed,
    InvalidBeadPinName,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetPinModeError
{
    BeadClosed,
    InvalidBeadPinName,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SetPinPowerError
{
    BeadClosed,
    InvalidBeadPinName,
    InvalidWriteToInput { power: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetPinModeError
{
    BeadClosed,
    InvalidBeadPinName,
    InvalidUnsupportedMode { mode: PinMode },
}

#[derive(Debug, Default)]
enum SerialHandlerConnectionState
{
    #[default]
    WaitingConfig,
    Active
    {
        name: String,
        pins: Vec<(String, SerialHandlerPinConnection)>,
    },
    Closed,
}

#[derive(Debug)]
struct SerialHandlerConnection
{
    ping: watch::Sender<()>,
    ping_result: watch::Receiver<()>,
    state_result: watch::Receiver<SerialHandlerConnectionState>,
}

#[derive(Debug)]
struct SerialHandlerPinConnection
{
    flags: PinFlags,
    get_power: watch::Sender<()>,
    get_power_result: watch::Receiver<u16>,
    get_mode: watch::Sender<()>,
    get_mode_result: watch::Receiver<PinMode>,
    set_power: watch::Sender<u8>,
    set_power_result: watch::Receiver<Result<u8, SetPinPowerError>>,
    set_mode: watch::Sender<PinMode>,
    set_mode_result: watch::Receiver<Result<PinMode, SetPinModeError>>,
}

pub struct SerialHandler
{
    next_closed_result: watch::Sender<String>,
    next_listen_result: watch::Sender<(String, String, f32)>,
    connected: Mutex<HashMap<String, SerialHandlerConnection>>,
}

const EXPECTED_BEAD_VERSION: u16 = 1;

impl SerialHandler
{
    pub fn new() -> Self
    {
        Self
        {
            next_closed_result: <_>::default(),
            next_listen_result: <_>::default(),
            connected: Mutex::new(HashMap::new()),
        }
    }

    pub async fn ping<B: ?Sized>(&self, bead: &B)
        -> Option<Result<(), PingError>>
    where
        String: Borrow<B>,
        B: Hash + Eq,
    {
        let mut ping_result =
        {
            let lock = self.connected.lock().await;

            let Some(SerialHandlerConnection { ping, ping_result, .. }) = lock.get(bead)
            else { return None };

            _ = ping.send_replace(());
            ping_result.clone()
        };

        match ping_result.changed().await
        {
            Ok(()) => Some(Ok(())),
            Err(_) => Some(Err(PingError::BeadClosed)),
        }
    }

    pub async fn config<B: ?Sized>(&self, bead: &B)
        -> Option<Result<Config, WholeConfigError>>
    where
        String: Borrow<B>,
        B: Hash + Eq,
    {
        let mut state_result =
        {
            let lock = self.connected.lock().await;

            let Some(SerialHandlerConnection { state_result, .. }) = lock.get(bead)
            else { return None };

            state_result.clone()
        };

        let Ok(lock) = state_result.wait_for(|x| matches!(x, SerialHandlerConnectionState::Active { .. })).await
        else { return Some(Err(WholeConfigError::BeadClosed)) };

        let SerialHandlerConnectionState::Active { name, pins } = &*lock
        else { unreachable!() };

        Some(Ok(Config
        {
            version: EXPECTED_BEAD_VERSION,
            name: name.clone(),
            pins: pins.iter()
                .map(|(name, pin)| ConfigPin { name: name.clone(), flags: pin.flags })
                .collect(),
        }))
    }

    pub async fn get_pin_power<B: ?Sized, P: ?Sized>(&self, bead: &B, pin: &P)
        -> Option<Result<f32, GetPinPowerError>>
    where
        String: Borrow<B>,
        B: Hash + Eq,
        P: PartialEq<str>,
    {
        let mut state_result =
        {
            let lock = self.connected.lock().await;

            let Some(SerialHandlerConnection { state_result, .. }) = lock.get(bead)
            else { return None };

            state_result.clone()
        };

        let mut get_power_result =
        {
            let Ok(lock) = state_result.wait_for(|x| matches!(x, SerialHandlerConnectionState::Active { .. })).await
            else { return Some(Err(GetPinPowerError::BeadClosed)) };

            let SerialHandlerConnectionState::Active { pins, .. } = &*lock
            else { unreachable!() };

            let Some(pin) = pins.iter().position(|(name, _)| *pin == *name.as_str())
            else { return Some(Err(GetPinPowerError::InvalidBeadPinName)) };

            let SerialHandlerPinConnection { get_power, get_power_result, .. } = &pins[pin].1;

            _ = get_power.send_replace(());
            get_power_result.clone()
        };

        match get_power_result.changed().await
        {
            Ok(()) => Some(Ok(*watch::Receiver::borrow(&get_power_result) as f32 / ANALOG_READ_MAX as f32)),
            Err(_) => Some(Err(GetPinPowerError::BeadClosed)),
        }
    }

    pub async fn get_pin_mode<B: ?Sized, P: ?Sized>(&self, bead: &B, pin: &P)
        -> Option<Result<PinMode, GetPinModeError>>
    where
        String: Borrow<B>,
        B: Hash + Eq,
        P: PartialEq<str>,
    {
        let mut state_result =
        {
            let lock = self.connected.lock().await;

            let Some(SerialHandlerConnection { state_result, .. }) = lock.get(bead)
            else { return None };

            state_result.clone()
        };

        let mut get_mode_result =
        {
            let Ok(lock) = state_result.wait_for(|x| matches!(x, SerialHandlerConnectionState::Active { .. })).await
            else { return Some(Err(GetPinModeError::BeadClosed)) };

            let SerialHandlerConnectionState::Active { pins, .. } = &*lock
            else { unreachable!() };

            let Some(pin) = pins.iter().position(|(name, _)| *pin == *name.as_str())
            else { return Some(Err(GetPinModeError::InvalidBeadPinName)) };

            let SerialHandlerPinConnection { get_mode, get_mode_result, .. } = &pins[pin].1;

            _ = get_mode.send_replace(());
            get_mode_result.clone()
        };

        match get_mode_result.changed().await
        {
            Ok(()) => Some(Ok(*watch::Receiver::borrow(&get_mode_result))),
            Err(_) => Some(Err(GetPinModeError::BeadClosed)),
        }
    }

    pub async fn set_pin_power<B: ?Sized, P: ?Sized>(&self, bead: &B, pin: &P, power: f32)
        -> Option<Result<f32, SetPinPowerError>>
    where
        String: Borrow<B>,
        B: Hash + Eq,
        P: PartialEq<str>,
    {
        let mut state_result =
        {
            let lock = self.connected.lock().await;

            let Some(SerialHandlerConnection { state_result, .. }) = lock.get(bead)
            else { return None };

            state_result.clone()
        };

        let mut set_power_result =
        {
            let Ok(lock) = state_result.wait_for(|x| matches!(x, SerialHandlerConnectionState::Active { .. })).await
            else { return Some(Err(SetPinPowerError::BeadClosed)) };

            let SerialHandlerConnectionState::Active { pins, .. } = &*lock
            else { unreachable!() };

            let Some(pin) = pins.iter().position(|(name, _)| *pin == *name.as_str())
            else { return Some(Err(SetPinPowerError::InvalidBeadPinName)) };

            let SerialHandlerPinConnection { set_power, set_power_result, .. } = &pins[pin].1;

            _ = set_power.send_replace((power.clamp(0.0, 1.0) * ANALOG_WRITE_MAX as f32) as u8);
            set_power_result.clone()
        };

        match set_power_result.changed().await
        {
            Ok(()) => Some((*watch::Receiver::borrow(&set_power_result)).map(|x| x as f32 / ANALOG_WRITE_MAX as f32)),
            Err(_) => Some(Err(SetPinPowerError::BeadClosed)),
        }
    }

    pub async fn set_pin_mode<B: ?Sized, P: ?Sized>(&self, bead: &B, pin: &P, mode: PinMode)
        -> Option<Result<PinMode, SetPinModeError>>
    where
        String: Borrow<B>,
        B: Hash + Eq,
        P: PartialEq<str>,
    {
        let mut state_result =
        {
            let lock = self.connected.lock().await;

            let Some(SerialHandlerConnection { state_result, .. }) = lock.get(bead)
            else { return None };

            state_result.clone()
        };

        let mut set_mode_result =
        {
            let Ok(lock) = state_result.wait_for(|x| matches!(x, SerialHandlerConnectionState::Active { .. })).await
            else { return Some(Err(SetPinModeError::BeadClosed)) };

            let SerialHandlerConnectionState::Active { pins, .. } = &*lock
            else { unreachable!() };

            let Some(pin) = pins.iter().position(|(name, _)| *pin == *name.as_str())
            else { return Some(Err(SetPinModeError::InvalidBeadPinName)) };

            let SerialHandlerPinConnection { set_mode, set_mode_result, .. } = &pins[pin].1;

            _ = set_mode.send_replace(mode);
            set_mode_result.clone()
        };

        match set_mode_result.changed().await
        {
            Ok(()) => Some(*watch::Receiver::borrow(&set_mode_result)),
            Err(_) => Some(Err(SetPinModeError::BeadClosed)),
        }
    }

    pub async fn next_closed(&self) -> Result<String, ()>
    {
        let mut result = self.next_closed_result.subscribe();
        match result.changed().await
        {
            Ok(()) => Ok(result.borrow().clone()),
            Err(_) => Err(()),
        }
    }

    pub async fn next_listen(&self) -> Result<(String, String, f32), ()>
    {
        let mut result = self.next_listen_result.subscribe();
        match result.changed().await
        {
            Ok(()) => Ok(result.borrow().clone()),
            Err(_) => Err(()),
        }
    }

    pub async fn update(&self, mut new: impl FnMut(String))
    {
        let available_ports = tokio_serial::available_ports().unwrap();

        let mut lock = self.connected.lock().await;

        for info in available_ports
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
                        Ok(stream) => match lock.entry(serial_number.clone())
                        {
                            hash_map::Entry::Occupied(_) => (),
                            hash_map::Entry::Vacant(entry) =>
                            {
                                tokio::spawn(handle(
                                    entry,
                                    stream,
                                    self.next_closed_result.clone(),
                                    self.next_listen_result.clone()));
                                new(serial_number);
                            },
                        },
                    }
                }
            }
        }
    }
}

const ANALOG_READ_MAX: u16 = 1023;
const ANALOG_WRITE_MAX: u8 = 255;

fn handle(
    entry: hash_map::VacantEntry<'_, String, SerialHandlerConnection>,
    stream: SerialStream,
    next_close: watch::Sender<String>,
    next_listen: watch::Sender<(String, String, f32)>)
    -> impl Future<Output = ()> + Send + 'static
{
    const RETRY_DELAY: Duration = Duration::from_millis(500);

    let (rx_stream, tx_stream) = tokio::io::split(stream);
    let mut rx_stream = BufReader::new(rx_stream);
    let mut tx_stream = BufWriter::new(tx_stream);

    let (tx_ping, mut rx_ping) = watch::channel(());
    let (tx_ping_result, rx_ping_result) = watch::channel(());
    let (tx_state_result, rx_state_result) = watch::channel(SerialHandlerConnectionState::WaitingConfig);

    let id = entry.key().clone();

    entry.insert(SerialHandlerConnection
    {
        ping: tx_ping,
        ping_result: rx_ping_result,
        state_result: rx_state_result,
    });

    async fn send_to(writer: &mut (impl AsyncWrite + Unpin), value: impl Serialize) -> io::Result<()>
    {
        value.write_to(writer).await?;
        writer.flush().await
    }

    async move
    {
        'and_close:
        {
            let config = loop
            {
                match timeout(RETRY_DELAY, incoming::Packet::read_from(&mut rx_stream)).await
                {
                    Ok(Ok(incoming::Packet::Config(x @ Config { version, .. }))) =>
                    {
                        if version == EXPECTED_BEAD_VERSION
                        {
                            break x;
                        }
                        else
                        {
                            eprintln!("{Error} Failed to read config of bead: \
                                Unsupported version, expected '{EXPECTED_BEAD_VERSION}' got '{version}'.");
                            break 'and_close;
                        }
                    },
                    Ok(Err(packet::Error::Reset(_))) =>
                    {
                        continue;
                    },
                    Ok(Err(packet::Error::InvalidEscape { byte })) =>
                    {
                        eprintln!("{Warn} Bead before config sent invalid escape byte '0x{byte:0<2X}'.");
                    },
                    Ok(Err(packet::Error::Other(_))) =>
                    {
                        eprintln!("{Warn} Bead before config sent malformed packet.");
                    },
                    Ok(Err(packet::Error::IO(x))) =>
                    {
                        eprintln!("{Error} Failed to read config of bead: {x}.");
                        break 'and_close;
                    },
                    _ =>
                    {
                        if let Err(x) = send_to(&mut tx_stream, outgoing::Packet::WholeConfig).await
                        {
                            eprintln!("{Error} Failed to poll config of bead: {x}.");
                            break 'and_close;
                        }
                    },
                }
            };

            #[derive(Debug)]
            struct PinInterface
            {
                get_power: watch::Receiver<()>,
                get_power_result: watch::Sender<u16>,
                waiting_get_power: bool,
                get_mode: watch::Receiver<()>,
                get_mode_result: watch::Sender<PinMode>,
                waiting_get_mode: bool,
                set_power: watch::Receiver<u8>,
                set_power_result: watch::Sender<Result<u8, SetPinPowerError>>,
                waiting_set_power: bool,
                set_mode: watch::Receiver<PinMode>,
                set_mode_result: watch::Sender<Result<PinMode, SetPinModeError>>,
                waiting_set_mode: bool,
            }

            let (mut pin_interfaces, pins): (Vec<_>, Vec<_>) = config.pins.into_iter()
                .map(|pin|
                {
                    let (tx_get_power, rx_get_power) = watch::channel(());
                    let (tx_get_power_result, rx_get_power_result) = watch::channel(0);
                    let (tx_get_mode, rx_get_mode) = watch::channel(());
                    let (tx_get_mode_result, rx_get_mode_result) = watch::channel(PinMode::DigitalInput);
                    let (tx_set_power, rx_set_power) = watch::channel(0);
                    let (tx_set_power_result, rx_set_power_result) = watch::channel(Ok(0));
                    let (tx_set_mode, rx_set_mode) = watch::channel(PinMode::DigitalInput);
                    let (tx_set_mode_result, rx_set_mode_result) = watch::channel(Ok(PinMode::DigitalInput));

                    (
                        (
                            pin.name.clone(),
                            PinInterface
                            {
                                get_power: rx_get_power,
                                get_power_result: tx_get_power_result,
                                waiting_get_power: false,
                                get_mode: rx_get_mode,
                                get_mode_result: tx_get_mode_result,
                                waiting_get_mode: false,
                                set_power: rx_set_power,
                                set_power_result: tx_set_power_result,
                                waiting_set_power: false,
                                set_mode: rx_set_mode,
                                set_mode_result: tx_set_mode_result,
                                waiting_set_mode: false,
                            }),
                        (
                            pin.name,
                            SerialHandlerPinConnection
                            {
                                flags: pin.flags,
                                get_power: tx_get_power,
                                get_power_result: rx_get_power_result,
                                get_mode: tx_get_mode,
                                get_mode_result: rx_get_mode_result,
                                set_power: tx_set_power,
                                set_power_result: rx_set_power_result,
                                set_mode: tx_set_mode,
                                set_mode_result: rx_set_mode_result,
                            }))
                })
                .unzip();

            let name = config.name;

            _ = tx_state_result.send_replace(SerialHandlerConnectionState::Active
            {
                name: name.clone(),
                pins,
            });

            let mut deserializer_value = incoming::Packet::read_from(&mut rx_stream);
            let mut deserializer = unsafe { Pin::new_unchecked(&mut deserializer_value) };

            let mut waiting_for_pong = false;

            loop
            {
                fn poll_many<F>(source: &mut PinEach<Vec<F>>)
                    -> impl FnMut(&mut Context<'_>) -> Poll<(usize, F::Output)>
                where
                    F: Future,
                {
                    move |cx|
                    {
                        for (i, wait) in source.iter_pin_mut().enumerate()
                        {
                            if let Poll::Ready(x) = wait.poll(cx) { return Poll::Ready((i, x)) };
                        }

                        Poll::Pending
                    }
                }

                async fn wait_ok_or_timeout<T: Clone>(
                    value: &mut watch::Receiver<T>,
                    do_timeout: bool) -> T
                {
                    match match do_timeout
                    {
                        false => Ok(value.changed().await),
                        true => timeout(RETRY_DELAY, value.changed()).await,
                    }
                    {
                        Err(_) | Ok(Ok(_)) => return watch::Receiver::borrow(value).clone(),
                        Ok(Err(_)) => (),
                    }

                    pending().await
                }

                let mut get_power_waits = Vec::new();
                let mut get_mode_waits = Vec::new();
                let mut set_power_waits = Vec::new();
                let mut set_mode_waits = Vec::new();

                for (_, PinInterface
                {
                    get_power, waiting_get_power,
                    get_mode, waiting_get_mode,
                    set_power, waiting_set_power,
                    set_mode, waiting_set_mode,
                    ..
                })
                in pin_interfaces.iter_mut()
                {
                    get_power_waits.push(wait_ok_or_timeout(get_power, *waiting_get_power));
                    get_mode_waits.push(wait_ok_or_timeout(get_mode, *waiting_get_mode));
                    set_power_waits.push(wait_ok_or_timeout(set_power, *waiting_set_power));
                    set_mode_waits.push(wait_ok_or_timeout(set_mode, *waiting_set_mode));
                }

                let mut get_power_waits = PinEach::from(get_power_waits);
                let mut get_mode_waits = PinEach::from(get_mode_waits);
                let mut set_power_waits = PinEach::from(set_power_waits);
                let mut set_mode_waits = PinEach::from(set_mode_waits);

                select!
                {
                    biased;

                    packet = poll_fn(|cx| deserializer.as_mut().poll(cx)) =>
                    {
                        drop(deserializer_value);
                        deserializer_value = incoming::Packet::read_from(&mut rx_stream);
                        deserializer = unsafe { Pin::new_unchecked(&mut deserializer_value) };

                        drop((get_power_waits, get_mode_waits, set_power_waits, set_mode_waits));

                        match packet
                        {
                            Ok(incoming::Packet::Pong) =>
                            {
                                _ = tx_ping_result.send_replace(());
                            },
                            Ok(incoming::Packet::Config(_)) =>
                            {
                                eprintln!("{Warn} Bead sent second config.");
                            },
                            Ok(incoming::Packet::GetPinPowerResponse { pin, power }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    let (_, pin) = &mut pin_interfaces[pin as usize];
                                    _ = pin.get_power_result.send_replace(power);
                                    pin.waiting_get_power = false;
                                }
                            },
                            Ok(incoming::Packet::GetPinModeResponse { pin, mode }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    let (_, pin) = &mut pin_interfaces[pin as usize];
                                    _ = pin.get_mode_result.send_replace(mode);
                                    pin.waiting_get_power = false;
                                }
                            },
                            Ok(incoming::Packet::SetPinPowerResponse { pin, power }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    let (_, pin) = &mut pin_interfaces[pin as usize];
                                    _ = pin.set_power_result.send_replace(Ok(power));
                                    pin.waiting_get_power = false;
                                }
                            },
                            Ok(incoming::Packet::SetPinModeResponse { pin, mode }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    let (_, pin) = &mut pin_interfaces[pin as usize];
                                    _ = pin.set_mode_result.send_replace(Ok(mode));
                                    pin.waiting_get_power = false;
                                }
                            },
                            Ok(incoming::Packet::PinListen { pin, power }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    next_listen.send_modify(|x|
                                    {
                                        x.0.clone_from(&name);
                                        x.1.clone_from(&pin_interfaces[pin as usize].0);
                                        x.2 = power.min(ANALOG_READ_MAX) as f32 / ANALOG_READ_MAX as f32;
                                    });
                                }
                            },
                            Ok(incoming::Packet::InvalidPinMode { byte }) =>
                            {
                                eprintln!("{Warn} Bead '{name}' reported invalid pin mode '0x{byte:0<2X}'.");
                            },
                            Ok(incoming::Packet::InvalidPinId { byte }) =>
                            {
                                eprintln!("{Warn} Bead '{name}' reported invalid pin id '0x{byte:0<2X}'.");
                            },
                            Ok(incoming::Packet::InvalidPacketId { byte }) =>
                            {
                                eprintln!("{Warn} Bead '{name}' reported invalid packet id '0x{byte:0<2X}'.");
                            },
                            Ok(incoming::Packet::InvalidWriteToInput { pin, power }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    let power = power.min(ANALOG_WRITE_MAX) as f32 / ANALOG_WRITE_MAX as f32;
                                    let (_, pin) = &mut pin_interfaces[pin as usize];
                                    _ = pin.set_power_result.send_replace(Err(SetPinPowerError::InvalidWriteToInput { power }));
                                    pin.waiting_get_power = false;
                                }
                            },
                            Ok(incoming::Packet::InvalidUnsupportedMode { pin, mode }) =>
                            {
                                if pin as usize > pin_interfaces.len()
                                {
                                    eprintln!("{Warn} Bead '{name}' send invalid pin '0x{pin:0<2X}'.");
                                }
                                else
                                {
                                    let (_, pin) = &mut pin_interfaces[pin as usize];
                                    _ = pin.set_mode_result.send_replace(Err(SetPinModeError::InvalidUnsupportedMode { mode }));
                                    pin.waiting_get_power = false;
                                }
                            },
                            Ok(incoming::Packet::InvalidEscape { byte }) =>
                            {
                                eprintln!("{Warn} Bead '{name}' reported invalid escape byte '0x{byte:0<2X}'.");
                            },
                            Err(packet::Error::Reset(_)) =>
                            {
                                continue;
                            },
                            Err(packet::Error::InvalidEscape { byte }) =>
                            {
                                eprintln!("{Warn} Bead '{name}' sent invalid escape byte '0x{byte:0<2X}'.");
                            },
                            Err(packet::Error::Other(_)) =>
                            {
                                eprintln!("{Warn} Bead '{name}' sent malformed packet.");
                            },
                            Err(packet::Error::IO(x)) =>
                            {
                                match x.kind()
                                {
                                    io::ErrorKind::UnexpectedEof =>
                                    {
                                        break 'and_close;
                                    },
                                    _ =>
                                    {
                                        eprintln!("{Error} Failed to read config of bead: {x}.");
                                        break 'and_close;
                                    },
                                }
                            },
                        }
                    },
                    () = wait_ok_or_timeout(&mut rx_ping, waiting_for_pong) =>
                    {
                        drop((get_power_waits, get_mode_waits, set_power_waits, set_mode_waits));

                        waiting_for_pong = true;
                        if let Err(x) = send_to(
                            &mut tx_stream,
                            outgoing::Packet::Ping).await
                        {
                            eprintln!("{Error} Failed to send packet to bead: {x}.");
                            break 'and_close;
                        }
                    },
                    (pin, ()) = poll_fn(poll_many(&mut get_power_waits)) =>
                    {
                        drop((get_power_waits, get_mode_waits, set_power_waits, set_mode_waits));

                        pin_interfaces[pin as usize].1.waiting_get_power = true;
                        if let Err(x) = send_to(
                            &mut tx_stream,
                            outgoing::Packet::GetPinPower { pin: pin as u8 }).await
                        {
                            eprintln!("{Error} Failed to send packet to bead: {x}.");
                            break 'and_close;
                        }
                    },
                    (pin, ()) = poll_fn(poll_many(&mut get_mode_waits)) =>
                    {
                        drop((get_power_waits, get_mode_waits, set_power_waits, set_mode_waits));

                        pin_interfaces[pin as usize].1.waiting_get_mode = true;
                        if let Err(x) = send_to(
                            &mut tx_stream,
                            outgoing::Packet::GetPinMode { pin: pin as u8 }).await
                        {
                            eprintln!("{Error} Failed to send packet to bead: {x}.");
                            break 'and_close;
                        }
                    },
                    (pin, power) = poll_fn(poll_many(&mut set_power_waits)) =>
                    {
                        drop((get_power_waits, get_mode_waits, set_power_waits, set_mode_waits));

                        pin_interfaces[pin as usize].1.waiting_set_power = true;
                        if let Err(x) = send_to(
                            &mut tx_stream,
                            outgoing::Packet::SetPinPower { pin: pin as u8, power }).await
                        {
                            eprintln!("{Error} Failed to send packet to bead: {x}.");
                            break 'and_close;
                        }
                    },
                    (pin, mode) = poll_fn(poll_many(&mut set_mode_waits)) =>
                    {
                        drop((get_power_waits, get_mode_waits, set_power_waits, set_mode_waits));

                        pin_interfaces[pin as usize].1.waiting_set_mode = true;
                        if let Err(x) = send_to(
                            &mut tx_stream,
                            outgoing::Packet::SetPinMode { pin: pin as u8, mode }).await
                        {
                            eprintln!("{Error} Failed to send packet to bead: {x}.");
                            break 'and_close;
                        }
                    },
                }
            }
        }

        _ = tx_state_result.send_replace(SerialHandlerConnectionState::Closed);
        _ = next_close.send_replace(id);
    }
}