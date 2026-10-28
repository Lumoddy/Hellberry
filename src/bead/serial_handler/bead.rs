use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{self, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::timeout;
use tokio_serial::SerialStream;

use crate::bead::{FrameError, PinFlags, PinMode, outgoing, read_frame, surround_frame};
use crate::bead::packet::incoming;
use crate::log::*;

use super::handler;

#[derive(Debug)]
pub enum Request {
    Ping { response: oneshot::Sender<Result<(), &'static str>> },
    GetPinPower { pin: Arc<str>, response: oneshot::Sender<Result<f32, &'static str>> },
    GetPinMode { pin: Arc<str>, response: oneshot::Sender<Result<PinMode, &'static str>> },
    GetPinFlags { pin: Arc<str>, response: oneshot::Sender<Result<PinFlags, &'static str>> },
    SetPinPower { pin: Arc<str>, power: f32, response: oneshot::Sender<Result<f32, &'static str>> },
    SetPinMode { pin: Arc<str>, mode: PinMode, response: oneshot::Sender<Result<PinMode, &'static str>> },
}

#[derive(Clone, Debug)]
pub struct Info {
    model: Arc<str>,
    pins: Arc<[(Arc<str>, PinFlags)]>,
}

#[derive(Clone, Debug)]
pub enum Error<T = Infallible> {
    FailedToInit { message: &'static str },
    Other(T),
}

impl<T> From<T> for Error<T> {
    fn from(value: T) -> Self { Error::Other(value) }
}

const MAX_POWER_READ: u16 = 1023;
const MAX_POWER_WRITE: u8 = 255;

pub(super) async fn handle_serial(
    bead: &Arc<str>,
    serial: SerialStream,
    events: &broadcast::Sender<handler::Event>)
    -> Result<(Info, mpsc::Sender<Request>), (SerialStream, Error<io::Error>)>
{
    let mut serial = BufReader::new(serial);
    let mut buffer = heapless::Deque::new();
    let mut info = None;
    'init: for _ in 0..20 {
        outgoing::serialize(&outgoing::Request::GetConfig { }, &mut buffer);
        surround_frame(&mut buffer);

        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
            return Err((serial.into_inner(), Error::Other(error)));
        };

        match timeout(Duration::from_millis(200), read_frame(&mut serial, &mut buffer)).await {
            Err(_) => {
                buffer.clear();
                continue 'init;
            },
            Ok(Err(FrameError::NotMagic)) => {
                _ = buffer.pop_front();
                continue 'init;
            },
            Ok(Err(FrameError::ChecksumMismatch)) => {
                buffer.clear();
                continue 'init;
            },
            Ok(Err(FrameError::FromRead(error))) => {
                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                return Err((serial.into_inner(), Error::Other(error)));
            },
            Ok(Ok(payload)) => {
                match incoming::deserialize(payload) {
                    Ok(incoming::Response::Config { model, pins }) => {
                        info = Some(Info {
                            model: Arc::from(model),
                            pins: Arc::from_iter(pins.map(|(name, flags)| (Arc::from(name), flags))),
                        });
                        break 'init;
                    },
                    Ok(incoming::Response::DenyError { message }) => {
                        eprintln!("{Error} Bead denied init: {message}.");
                        return Err((serial.into_inner(), Error::FailedToInit { message: "Denied" }));
                    },
                    Ok(incoming::Response::Panic { line, column, file }) => {
                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                        return Err((serial.into_inner(), Error::FailedToInit { message: "Panicked" }));
                    },
                    _ => {
                        continue 'init;
                    },
                }
            },
        }
    }

    let Some(info) = info else {
        eprintln!("{Error} Bead init failed too many attempts.");
        return Err((serial.into_inner(), Error::FailedToInit { message: "Failed too many attempts" }));
    };

    let (tx, mut rx) = mpsc::channel(1);

    let bead = bead.clone();
    let events = events.clone();
    let result_info = info.clone();

    events.send(handler::Event::Opened { bead: bead.clone(), info: info.clone() });

    tokio::spawn(async move {
        const RETRY_DELAY: Duration = Duration::from_millis(200);

        'and_close: while let Some(request) = rx.recv().await {
            match request {
                Request::Ping { response } => {
                    'attempt: loop {
                        outgoing::serialize(&outgoing::Request::Ping { }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}.");
                            break 'and_close;
                        };

                        match timeout(RETRY_DELAY, read_frame(&mut serial, &mut buffer)).await {
                            Err(_) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::NotMagic)) => {
                                _ = buffer.pop_front();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::ChecksumMismatch)) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::FromRead(error))) => {
                                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                                break 'and_close;
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Pong { }) => {
                                        _ = response.send(Ok(()));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::DenyError { message }) => {
                                        eprintln!("{Warn} Bead denied get pin power: {message}.");
                                        _ = response.send(Err(match message {
                                            "Invalid pin id" => "Invalid pin id",
                                            _ => "Denied get pin power",
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::Panic { line, column, file }) => {
                                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                                        _ = response.send(Err("Panicked"));
                                        break 'and_close;
                                    },
                                    Ok(incoming::Response::PinChanged { pin, power }) => {
                                        if let Some((name, _)) = info.pins.iter().find(|(name, _)| **name == *pin) {
                                            _ = events.send(handler::Event::PinListen {
                                                bead: bead.clone(),
                                                pin: name.clone(),
                                                power: power as f32 / MAX_POWER_READ as f32,
                                            });
                                        } else {
                                            eprintln!("{Warn} Bead sent event for unknown pin.");
                                        }
                                        continue 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::GetPinPower { pin, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        outgoing::serialize(&outgoing::Request::GetPinPower { pin }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}.");
                            break 'and_close;
                        };

                        match timeout(RETRY_DELAY, read_frame(&mut serial, &mut buffer)).await {
                            Err(_) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::NotMagic)) => {
                                _ = buffer.pop_front();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::ChecksumMismatch)) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::FromRead(error))) => {
                                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                                break 'and_close;
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::GetPinPowerResponse { pin: response_pin, power })
                                    if *response_pin == **pin => {
                                        _ = response.send(Ok(power.min(MAX_POWER_READ) as f32 / MAX_POWER_READ as f32));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::DenyError { message }) => {
                                        eprintln!("{Warn} Bead denied get pin power: {message}.");
                                        _ = response.send(Err(match message {
                                            "Invalid pin id" => "Invalid pin id",
                                            _ => "Denied get pin power",
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::Panic { line, column, file }) => {
                                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                                        _ = response.send(Err("Panicked"));
                                        break 'and_close;
                                    },
                                    Ok(incoming::Response::PinChanged { pin, power }) => {
                                        if let Some((name, _)) = info.pins.iter().find(|(name, _)| **name == *pin) {
                                            _ = events.send(handler::Event::PinListen {
                                                bead: bead.clone(),
                                                pin: name.clone(),
                                                power: power as f32 * MAX_POWER_READ as f32,
                                            });
                                        } else {
                                            eprintln!("{Warn} Bead sent event for unknown pin.");
                                        }
                                        continue 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::GetPinMode { pin, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        outgoing::serialize(&outgoing::Request::GetPinMode { pin }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}.");
                            break 'and_close;
                        };

                        match timeout(RETRY_DELAY, read_frame(&mut serial, &mut buffer)).await {
                            Err(_) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::NotMagic)) => {
                                _ = buffer.pop_front();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::ChecksumMismatch)) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::FromRead(error))) => {
                                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                                break 'and_close;
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::GetPinModeResponse { pin: response_pin, mode })
                                    if *response_pin == **pin => {
                                        _ = response.send(Ok(mode));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::DenyError { message }) => {
                                        eprintln!("{Warn} Bead denied get pin mode: {message}.");
                                        _ = response.send(Err(match message {
                                            "Invalid pin id" => "Invalid pin id",
                                            _ => "Denied get pin mode",
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::Panic { line, column, file }) => {
                                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                                        _ = response.send(Err("Panicked"));
                                        break 'and_close;
                                    },
                                    Ok(incoming::Response::PinChanged { pin, power }) => {
                                        if let Some((name, _)) = info.pins.iter().find(|(name, _)| **name == *pin) {
                                            _ = events.send(handler::Event::PinListen {
                                                bead: bead.clone(),
                                                pin: name.clone(),
                                                power: power as f32 * MAX_POWER_READ as f32,
                                            });
                                        } else {
                                            eprintln!("{Warn} Bead sent event for unknown pin.");
                                        }
                                        continue 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::GetPinFlags { pin, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        outgoing::serialize(&outgoing::Request::GetPinFlags { pin }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}.");
                            break 'and_close;
                        };

                        match timeout(RETRY_DELAY, read_frame(&mut serial, &mut buffer)).await {
                            Err(_) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::NotMagic)) => {
                                _ = buffer.pop_front();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::ChecksumMismatch)) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::FromRead(error))) => {
                                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                                break 'and_close;
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::GetPinFlagsResponse { pin: response_pin, flags })
                                    if *response_pin == **pin => {
                                        _ = response.send(Ok(flags));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::DenyError { message }) => {
                                        eprintln!("{Warn} Bead denied get pin flags: {message}.");
                                        _ = response.send(Err(match message {
                                            "Invalid pin id" => "Invalid pin id",
                                            _ => "Denied get pin flags",
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::Panic { line, column, file }) => {
                                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                                        _ = response.send(Err("Panicked"));
                                        break 'and_close;
                                    },
                                    Ok(incoming::Response::PinChanged { pin, power }) => {
                                        if let Some((name, _)) = info.pins.iter().find(|(name, _)| **name == *pin) {
                                            _ = events.send(handler::Event::PinListen {
                                                bead: bead.clone(),
                                                pin: name.clone(),
                                                power: power as f32 * MAX_POWER_READ as f32,
                                            });
                                        } else {
                                            eprintln!("{Warn} Bead sent event for unknown pin.");
                                        }
                                        continue 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::SetPinPower { pin, power, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        let power = (power * MAX_POWER_WRITE as f32) as u8;
                        outgoing::serialize(&outgoing::Request::SetPinPower { pin, power }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}.");
                            break 'and_close;
                        };

                        match timeout(RETRY_DELAY, read_frame(&mut serial, &mut buffer)).await {
                            Err(_) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::NotMagic)) => {
                                _ = buffer.pop_front();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::ChecksumMismatch)) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::FromRead(error))) => {
                                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                                break 'and_close;
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::SetPinPowerResponse { pin: response_pin, power })
                                    if *response_pin == **pin => {
                                        _ = response.send(Ok(power.min(MAX_POWER_WRITE) as f32 / MAX_POWER_WRITE as f32));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::DenyError { message }) => {
                                        eprintln!("{Warn} Bead denied set pin power: {message}.");
                                        _ = response.send(Err(match message {
                                            "Invalid pin id" => "Invalid pin id",
                                            _ => "Denied set pin power",
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::Panic { line, column, file }) => {
                                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                                        _ = response.send(Err("Panicked"));
                                        break 'and_close;
                                    },
                                    Ok(incoming::Response::PinChanged { pin, power }) => {
                                        if let Some((name, _)) = info.pins.iter().find(|(name, _)| **name == *pin) {
                                            _ = events.send(handler::Event::PinListen {
                                                bead: bead.clone(),
                                                pin: name.clone(),
                                                power: power as f32 * MAX_POWER_READ as f32,
                                            });
                                        } else {
                                            eprintln!("{Warn} Bead sent event for unknown pin.");
                                        }
                                        continue 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::SetPinMode { pin, mode, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        outgoing::serialize(&outgoing::Request::SetPinMode { pin, mode }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}.");
                            break 'and_close;
                        };

                        match timeout(RETRY_DELAY, read_frame(&mut serial, &mut buffer)).await {
                            Err(_) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::NotMagic)) => {
                                _ = buffer.pop_front();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::ChecksumMismatch)) => {
                                buffer.clear();
                                continue 'attempt;
                            },
                            Ok(Err(FrameError::FromRead(error))) => {
                                eprintln!("{Error} Failed to receive packet from bead: {error}.");
                                break 'and_close;
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::SetPinModeResponse { pin: response_pin, mode })
                                    if *response_pin == **pin => {
                                        _ = response.send(Ok(mode));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::DenyError { message }) => {
                                        eprintln!("{Warn} Bead denied set pin mode: {message}.");
                                        _ = response.send(Err(match message {
                                            "Invalid pin id" => "Invalid pin id",
                                            _ => "Denied set pin mode",
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::Panic { line, column, file }) => {
                                        eprintln!("{Error} Bead panicked at {file}:{line}:{column}.");
                                        _ = response.send(Err("Panicked"));
                                        break 'and_close;
                                    },
                                    Ok(incoming::Response::PinChanged { pin, power }) => {
                                        if let Some((name, _)) = info.pins.iter().find(|(name, _)| **name == *pin) {
                                            _ = events.send(handler::Event::PinListen {
                                                bead: bead.clone(),
                                                pin: name.clone(),
                                                power: power as f32 * MAX_POWER_READ as f32,
                                            });
                                        } else {
                                            eprintln!("{Warn} Bead sent event for unknown pin.");
                                        }
                                        continue 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
            }
        }

        events.send(handler::Event::Closed { bead: bead.clone() });
    });

    Ok((result_info, tx))
}
