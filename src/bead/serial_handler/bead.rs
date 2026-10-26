use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{self, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, };
use tokio::time::timeout;
use tokio_serial::SerialStream;

use crate::bead::{FrameError, PinFlags, PinMode, outgoing, read_frame, surround_frame};
use crate::bead::packet::incoming;
use crate::de::Deserialize;
use crate::log::*;

#[derive(Debug)]
pub enum Request {
    Ping { response: oneshot::Sender<()> },
    GetPinPower { pin: Arc<str>, response: oneshot::Sender<Result<f32, &'static str>> },
    GetPinMode { pin: Arc<str>, response: oneshot::Sender<Result<PinMode, &'static str>> },
    GetPinFlags { pin: Arc<str>, response: oneshot::Sender<Result<PinFlags, &'static str>> },
    SetPower { pin: Arc<str>, power: f32, response: oneshot::Sender<Result<f32, &'static str>> },
    SetMode { pin: Arc<str>, mode: PinMode, response: oneshot::Sender<Result<PinMode, &'static str>> },
}

#[derive(Clone, Debug)]
pub struct BeadInfo {
    model: Arc<str>,
    pins: Vec<(Arc<str>, PinFlags)>,
}

#[derive(Clone, Debug)]
pub enum BeadError<T = Infallible> {
    FailedToInit { message: &'static str },
    Other(T),
}

impl<T> From<T> for BeadError<T> {
    fn from(value: T) -> Self { BeadError::Other(value) }
}

const MAX_POWER_READ: u16 = 1023;
const MAX_POWER_WRITE: u8 = 255;

pub async fn handle_serial(serial: SerialStream)
    -> Result<(mpsc::Sender<Request>, BeadInfo), (SerialStream, BeadError<io::Error>)> {
    let mut serial = BufReader::new(serial);
    let mut buffer = heapless::Deque::new();
    let mut info = None;
    'ping_attempt: for _ in 0..20 {
        outgoing::serialize(&outgoing::Request::Ping { }, &mut buffer);
        surround_frame(&mut buffer);

        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
            return Err((serial.into_inner(), BeadError::Other(error)));
        };

        match timeout(Duration::from_millis(200), read_frame(&mut serial, &mut buffer)).await {
            Err(_) => {
                buffer.clear();
                continue 'ping_attempt;
            },
            Ok(Err(FrameError::NotMagic)) => {
                _ = buffer.pop_front();
                continue 'ping_attempt;
            },
            Ok(Err(FrameError::ChecksumMismatch)) => {
                buffer.clear();
                continue 'ping_attempt;
            },
            Ok(Err(FrameError::FromRead(error))) => {
                return Err((serial.into_inner(), BeadError::Other(error)));
            },
            Ok(Ok(payload)) => {
                match incoming::deserialize(payload) {
                    Ok(incoming::Response::Empty()) => {
                        'config_attempt: loop {
                            outgoing::serialize(&outgoing::Request::GetConfig { }, &mut buffer);
                            surround_frame(&mut buffer);

                            if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                                return Err((serial.into_inner(), BeadError::Other(error)));
                            };

                            'and_ignore: loop {
                                match timeout(Duration::from_millis(200), read_frame(&mut serial, &mut buffer)).await {
                                    Err(_) => {
                                        buffer.clear();
                                        continue 'config_attempt;
                                    },
                                    Ok(Err(FrameError::NotMagic)) => {
                                        _ = buffer.pop_front();
                                        continue 'config_attempt;
                                    },
                                    Ok(Err(FrameError::ChecksumMismatch)) => {
                                        buffer.clear();
                                        continue 'config_attempt;
                                    },
                                    Ok(Err(FrameError::FromRead(error))) => {
                                        return Err((serial.into_inner(), BeadError::Other(error)));
                                    },
                                    Ok(Ok(payload)) => {
                                        match incoming::deserialize(payload) {
                                            Ok(incoming::Response::Config { model, pins }) => {
                                                let model = Arc::from(model);
                                                let pins = pins.map(|(name, flags)| (Arc::from(name), flags)).collect();
                                                info = Some(BeadInfo { model, pins });
                                                break;
                                            },
                                            Ok(incoming::Response::Empty()) => {
                                                // Clear out any slow pongs.
                                                continue 'and_ignore;
                                            },
                                            _ => {
                                                continue 'config_attempt;
                                            },
                                        }
                                    },
                                }
                            }
                            break;
                        }
                        break;
                    },
                    _ => {
                        continue 'ping_attempt;
                    },
                }
            },
        }
    }

    let Some(info) = info else {
        return Err((serial.into_inner(), BeadError::FailedToInit { message: "Too many attempts" }));
    };

    let (tx, mut rx) = mpsc::channel(1);

    
    tokio::spawn(async move {
        const RETRY_DELAY: Duration = Duration::from_millis(200);

        while let Some(request) = rx.recv().await {
            match request {
                Request::Ping { response } => {
                    'attempt: loop {
                        outgoing::serialize(&outgoing::Request::Ping { }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}");
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
                                eprintln!("{Error} Failed to send packet to bead: {error}");
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Empty()) => {
                                        _ = response.send(());
                                        break 'attempt;
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
                            eprintln!("{Error} Failed to send packet to bead: {error}");
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
                                eprintln!("{Error} Failed to send packet to bead: {error}");
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Power(power)) => {
                                        _ = response.send(Ok(match power {
                                            MAX_POWER_READ.. => 1.0,
                                            x => x as f32 / MAX_POWER_READ as f32,
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Invalid pin id" }) => {
                                        _ = response.send(Err("Invalid pin id"));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Denied get pin power" }) => {
                                        _ = response.send(Err("Denied get pin power"));
                                        break 'attempt;
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
                            eprintln!("{Error} Failed to send packet to bead: {error}");
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
                                eprintln!("{Error} Failed to send packet to bead: {error}");
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Mode(mode)) => {
                                        _ = response.send(Ok(mode));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Invalid pin id" }) => {
                                        _ = response.send(Err("Invalid pin id"));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Denied get pin mode" }) => {
                                        _ = response.send(Err("Denied get pin mode"));
                                        break 'attempt;
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
                            eprintln!("{Error} Failed to send packet to bead: {error}");
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
                                eprintln!("{Error} Failed to send packet to bead: {error}");
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Flags(flags)) => {
                                        _ = response.send(Ok(flags));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Invalid pin id" }) => {
                                        _ = response.send(Err("Invalid pin id"));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Denied get pin flags" }) => {
                                        _ = response.send(Err("Denied get pin flags"));
                                        break 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::SetPower { pin, power, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        let power = (power * MAX_POWER_WRITE as f32) as u8;
                        outgoing::serialize(&outgoing::Request::SetPinPower { pin, power }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}");
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
                                eprintln!("{Error} Failed to send packet to bead: {error}");
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Power(power)) => {
                                        _ = response.send(Ok(match power {
                                            MAX_POWER_READ.. => 1.0,
                                            x => x as f32 / MAX_POWER_READ as f32,
                                        }));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Invalid pin id" }) => {
                                        _ = response.send(Err("Invalid pin id"));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Denied set pin power" }) => {
                                        _ = response.send(Err("Denied set pin power"));
                                        break 'attempt;
                                    },
                                    _ => {
                                        continue 'attempt;
                                    },
                                }
                            },
                        }
                    }
                },
                Request::SetMode { pin, mode, response } => {
                    'attempt: loop {
                        let pin = &pin;
                        outgoing::serialize(&outgoing::Request::SetPinMode { pin, mode }, &mut buffer);
                        surround_frame(&mut buffer);

                        if let Err(error) = serial.write_all(buffer.make_contiguous()).await {
                            eprintln!("{Error} Failed to send packet to bead: {error}");
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
                                eprintln!("{Error} Failed to send packet to bead: {error}");
                            },
                            Ok(Ok(payload)) => {
                                match incoming::deserialize(payload) {
                                    Ok(incoming::Response::Mode(mode)) => {
                                        _ = response.send(Ok(mode));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Invalid pin id" }) => {
                                        _ = response.send(Err("Invalid pin id"));
                                        break 'attempt;
                                    },
                                    Ok(incoming::Response::NACK { message: "Denied set pin mode" }) => {
                                        _ = response.send(Err("Denied set pin mode"));
                                        break 'attempt;
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
    });

    Ok((tx, info))
}
