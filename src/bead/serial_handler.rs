use std::collections::{HashMap, hash_map};
use std::future::{pending, poll_fn};
use std::pin::Pin;
use std::sync::Arc;
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

use super::{Config, PinFlags, PinMode};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetPinPowerError
{
    BeadClosed,
    InvalidBeadPinName,
    InvalidWriteToInput { power: u16 },
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
}

#[derive(Debug)]
struct SerialHandlerConnection
{
    closed: watch::Receiver<bool>,
    ping: watch::Sender<()>,
    ping_result: watch::Receiver<()>,
    state: watch::Receiver<SerialHandlerConnectionState>,
}

#[derive(Debug)]
struct SerialHandlerPinConnection
{
    flags: PinFlags,
    get_power: watch::Sender<()>,
    get_power_result: watch::Receiver<u16>,
    get_mode: watch::Sender<()>,
    get_mode_result: watch::Receiver<PinMode>,
    set_power: watch::Sender<u16>,
    set_power_result: watch::Receiver<Result<u16, SetPinPowerError>>,
    set_mode: watch::Sender<PinMode>,
    set_mode_result: watch::Receiver<Result<PinMode, SetPinModeError>>,
    listen_result: watch::Receiver<u16>,
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

    pub async fn update(self: Arc<Self>, mut new: impl AsyncFnMut(&str))
    {
        let available_ports = tokio_serial::available_ports().unwrap();

        let mut connected = self.connected.lock().await;

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
                        Ok(stream) => match connected.entry(serial_number)
                        {
                            hash_map::Entry::Occupied(_) => (),
                            hash_map::Entry::Vacant(entry) =>
                            {
                                new(entry.key()).await;
                                tokio::spawn(handle(entry, stream));
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

fn handle(
    entry: hash_map::VacantEntry<'_, String, SerialHandlerConnection>,
    stream: SerialStream)
    -> impl Future<Output = ()> + Send + 'static
{
    const RETRY_DELAY: Duration = Duration::from_millis(500);
    const EXPECTED_BEAD_VERSION: u16 = 1;

    let (rx_stream, tx_stream) = tokio::io::split(stream);
    let mut rx_stream = BufReader::new(rx_stream);
    let mut tx_stream = BufWriter::new(tx_stream);

    let (tx_close, rx_close) = watch::channel(false);
    let (tx_ping, mut rx_ping) = watch::channel(());
    let (tx_ping_result, rx_ping_result) = watch::channel(());
    let (tx_state, rx_state) = watch::channel(SerialHandlerConnectionState::WaitingConfig);

    entry.insert(SerialHandlerConnection
    {
        closed: rx_close,
        ping: tx_ping,
        ping_result: rx_ping_result,
        state: rx_state,
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
                set_power: watch::Receiver<u16>,
                set_power_result: watch::Sender<Result<u16, SetPinPowerError>>,
                waiting_set_power: bool,
                set_mode: watch::Receiver<PinMode>,
                set_mode_result: watch::Sender<Result<PinMode, SetPinModeError>>,
                waiting_set_mode: bool,
                listen_result: watch::Sender<u16>,
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
                    let (tx_listen_result, rx_listen_result) = watch::channel(0);

                    (
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
                            listen_result: tx_listen_result,
                        },
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
                                listen_result: rx_listen_result,
                            }))
                })
                .unzip();

            let name = config.name;

            tx_state.send_modify(|x| *x = SerialHandlerConnectionState::Active
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
                    -> impl use<'_, F> + FnMut(&mut Context<'_>) -> Poll<(usize, F::Output)>
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

                async fn ok_or_never<F, T, E>(f: F) -> T
                where
                    F: IntoFuture<Output = Result<T, E>>,
                {
                    if let Ok(x) = f.await { return x };
                    pending().await
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
                        Err(_) | Ok(Ok(_)) => return value.borrow().clone(),
                        Ok(Err(_)) => (),
                    }

                    pending().await
                }

                let mut get_power_waits = Vec::new();
                let mut get_mode_waits = Vec::new();
                let mut set_power_waits = Vec::new();
                let mut set_mode_waits = Vec::new();

                for PinInterface
                {
                    get_power, waiting_get_power,
                    get_mode, waiting_get_mode,
                    set_power, waiting_set_power,
                    set_mode, waiting_set_mode,
                    ..
                }
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
                                tx_ping_result.send_modify(|x| *x = ());
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.get_power_result.send_modify(|x| *x = power);
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.get_mode_result.send_modify(|x| *x = mode);
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.set_power_result.send_modify(|x| *x = Ok(power));
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.set_mode_result.send_modify(|x| *x = Ok(mode));
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.listen_result.send_modify(|x| *x = power);
                                    pin.waiting_get_power = false;
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.set_power_result.send_modify(|x| *x = Err(SetPinPowerError::InvalidWriteToInput { power }));
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
                                    let pin = &mut pin_interfaces[pin as usize];
                                    pin.set_mode_result.send_modify(|x| *x = Err(SetPinModeError::InvalidUnsupportedMode { mode }));
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

                        pin_interfaces[pin as usize].waiting_get_power = true;
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

                        pin_interfaces[pin as usize].waiting_get_mode = true;
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

                        pin_interfaces[pin as usize].waiting_set_power = true;
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

                        pin_interfaces[pin as usize].waiting_set_mode = true;
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

        tx_close.send_modify(|x| *x = true);
    }
}