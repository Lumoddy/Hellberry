use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use axum::extract::{Path, Query, WebSocketUpgrade, ws};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing;
use tokio::select;
use tokio::time::timeout;

use crate::bead::{self, PinMode, SerialHandler};
use crate::im::Ash;
use crate::log::*;
use crate::packet::{incoming, outgoing};
use crate::de::{Array, Deserialize, Quoted};

pub struct Router {
    pub config_name: Option<String>,
    pub serial_handler: Arc<SerialHandler>,
}

impl From<Router> for routing::Router {
    fn from(value: Router) -> Self {
        const RETRY: Duration = Duration::from_millis(500);

        let config = match value.config_name {
            Some(ref name) => format!("{{ name: {} }}", name),
            None => format!("{{ }}"),
        };

        let handler = value.serial_handler;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct ParamPin {
            power: f32,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct ParamMode {
            mode: String,
        }

        routing::Router::new()
            .route("/ws", {
                let config_name = value.config_name.map(Ash::from);
                let handler = handler.clone();
                let mut tx_buffer = String::new();
                routing::get(async move |ws| WebSocketUpgrade::on_upgrade(ws, async move |mut ws| {
                    'and_close: loop {
                        let response = select! {
                            Some(id) = handler.next_connect() => {
                                Some(outgoing::Packet::BeadConnected { bead: id.into() })
                            },
                            Some(id) = handler.next_disconnect() => {
                                Some(outgoing::Packet::BeadDisconnected { bead: id.into() })
                            },
                            Some((id, packet)) = handler.recv() => match packet {
                                bead::incoming::Packet::Pong { } => {
                                    Some(outgoing::Packet::BeadPong { bead: id.into() })
                                },
                                bead::incoming::Packet::PinPowerResponse { pin, power } => {
                                    let power = power as f32 / bead::ANALOG_READ_MAX as f32;
                                    Some(outgoing::Packet::BeadPinPowerResponse { bead: id.into(), pin, power })
                                },
                                bead::incoming::Packet::PinModeResponse { pin, mode } => {
                                    Some(outgoing::Packet::BeadPinModeResponse { bead: id.into(), pin, mode })
                                },
                                bead::incoming::Packet::PinFlagsResponse { pin, flags } => {
                                    Some(outgoing::Packet::BeadPinFlagsResponse { bead: id.into(), pin, flags })
                                },
                                bead::incoming::Packet::MalformedPacket { pos } => {
                                    Some(outgoing::Packet::MalformedPacket { pos })
                                },
                                bead::incoming::Packet::UnsupportedPinMode { pin, mode } => {
                                    Some(outgoing::Packet::UnsupportedBeadPinMode { bead: id.into(), pin, mode })
                                },
                                bead::incoming::Packet::PinGetPowerDenied { pin } => {
                                    Some(outgoing::Packet::BeadPinGetPowerDenied { bead: id.into(), pin })
                                },
                                bead::incoming::Packet::PinSetPowerDenied { pin } => {
                                    Some(outgoing::Packet::BeadPinSetPowerDenied { bead: id.into(), pin })
                                },
                                bead::incoming::Packet::Config { config } => {
                                    Some(outgoing::Packet::BeadConfig { bead: id.into(), config })
                                },
                            },
                            recv = ws.recv() => match recv {
                                None => break 'and_close,
                                Some(Ok(msg)) => {
                                    let msg = match msg {
                                        ws::Message::Text(bytes) => Some(bytes.into()),
                                        ws::Message::Binary(bytes) => Some(bytes),
                                        ws::Message::Close(_) => break 'and_close,
                                        _ => None,
                                    };

                                    match msg {
                                        None => None,
                                        Some(msg) => match incoming::Packet::deserialize(&msg.to_vec()) {
                                            Err((_, ())) => None,
                                            Ok(incoming::Packet::Ping { }) => {
                                                Some(outgoing::Packet::Pong { })
                                            },
                                            Ok(incoming::Packet::Config { }) => {
                                                Some(outgoing::Packet::Config { name: config_name.clone() })
                                            },
                                            Ok(incoming::Packet::BeadPing { bead }) => {
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::Ping { }).await;
                                                None
                                            },
                                            Ok(incoming::Packet::BeadConfig { bead }) => {
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::Config { }).await;
                                                None
                                            },
                                            Ok(incoming::Packet::GetBeadPinPower { bead, pin }) => {
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::GetPinPower { pin: pin.into_static() }).await; 
                                                None
                                            },
                                            Ok(incoming::Packet::GetBeadPinMode { bead, pin }) => {
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::GetPinMode { pin: pin.into_static() }).await; 
                                                None
                                            },
                                            Ok(incoming::Packet::BeadPinFlags { bead, pin }) => {
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::PinFlags { pin: pin.into_static() }).await; 
                                                None
                                            },
                                            Ok(incoming::Packet::SetBeadPinPower { bead, pin, power }) => {
                                                let power = (power.clamp(0.0, 1.0) * bead::ANALOG_WRITE_MAX as f32) as u8;
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::SetPinPower { pin: pin.into_static(), power }).await; 
                                                None
                                            },
                                            Ok(incoming::Packet::SetBeadPinMode { bead, pin, mode }) => {
                                                _ = handler.send(bead.as_ref(), bead::outgoing::Packet::SetPinMode { pin: pin.into_static(), mode }).await; 
                                                None
                                            },
                                        }
                                    }
                                },
                                Some(Err(x)) => {
                                    eprintln!("{Error} Failed to receive from websocket: {x}");
                                    break;
                                },
                            },
                        };

                        if let Some(response) = response {
                            writeln!(tx_buffer, "{}", response).unwrap();
                            if let Err(error) = ws.send(ws::Message::Text(Utf8Bytes::from(&tx_buffer))).await {
                                eprintln!("{Error} Serial reported error: {error}");
                                break 'and_close;
                            }
                            tx_buffer.clear();
                        }
                    }
                }))
            })
            .route("/ping", {
                routing::get(async move || "pong")
            })
            .route("/config", {
                routing::get(async move || config)
            })
            .route("/beads", {
                let handler = handler.clone();
                routing::any(async move |()| {
                    Array(handler.beads().await.into_iter().map(Quoted).collect::<Vec<_>>()).to_string()
                })
            })
            .route("/beads/{bead}/ping", {
                let handler = handler.clone();
                routing::any(async move |Path(bead): Path<String>| {
                    'and_retry: loop {
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::Ping { }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::Pong { })))
                            if recv_bead.as_ref() == bead.as_str() => {
                                return "pong".into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
            .route("/beads/{bead}/config", {
                let handler = handler.clone();
                routing::any(async move |Path(bead): Path<String>| {
                    'and_retry: loop {
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::Config { }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::Config { config })))
                            if recv_bead.as_ref() == bead.as_str() => {
                                return config.to_string().into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
            .route("/beads/{bead}/pins/{pin}/getPower", {
                let handler = handler.clone();
                routing::any(async move |Path((bead, pin)): Path<(String, String)>| {
                    'and_retry: loop {
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::GetPinPower {
                            pin: Arc::<str>::from(pin.as_str()).into(),
                        }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::PinGetPowerDenied { pin: recv_pin })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                return (StatusCode::BAD_REQUEST, "error.getPowerDenied()").into_response();
                            },
                            Ok(Some((recv_bead, bead::incoming::Packet::PinPowerResponse { pin: recv_pin, power })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                let power = power as f32 / bead::ANALOG_READ_MAX as f32;
                                return power.to_string().into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
            .route("/beads/{bead}/pins/{pin}/getMode", {
                let handler = handler.clone();
                routing::any(async move |Path((bead, pin)): Path<(String, String)>| {
                    'and_retry: loop {
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::GetPinMode {
                            pin: Arc::<str>::from(pin.as_str()).into(),
                        }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::PinModeResponse { pin: recv_pin, mode })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                return mode.to_string().into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
            .route("/beads/{bead}/pins/{pin}/flags", {
                let handler = handler.clone();
                routing::any(async move |Path((bead, pin)): Path<(String, String)>| {
                    'and_retry: loop {
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::PinFlags {
                            pin: Arc::<str>::from(pin.as_str()).into(),
                        }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::PinFlagsResponse { pin: recv_pin, flags })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                return flags.to_string().into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
            .route("/beads/{bead}/pins/{pin}/setPower", {
                let handler = handler.clone();
                routing::any(async move |Path((bead, pin)): Path<(String, String)>, Query(ParamPin { power })| {
                    'and_retry: loop {
                        let raw_power = (power.clamp(0.0, 1.0) * bead::ANALOG_WRITE_MAX as f32) as u8;
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::SetPinPower {
                            pin: Arc::<str>::from(pin.as_str()).into(),
                            power: raw_power,
                        }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::PinSetPowerDenied { pin: recv_pin })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                return StatusCode::BAD_REQUEST.into_response();
                            },
                            Ok(Some((recv_bead, bead::incoming::Packet::PinPowerResponse { pin: recv_pin, power })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                let power = power as f32 / bead::ANALOG_READ_MAX as f32;
                                return power.to_string().into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
            .route("/beads/{bead}/pins/{pin}/setMode", {
                let handler = handler.clone();
                routing::any(async move |Path((bead, pin)): Path<(String, String)>, Query(ParamMode { mode })| {
                    let mode = match mode.as_str() {
                        "ignored" => PinMode::Ignored,
                        "digital-input" => PinMode::DigitalInput,
                        "digital-output" => PinMode::DigitalOutput,
                        "analog-input" => PinMode::AnalogInput,
                        "analog-output" => PinMode::AnalogOutput,
                        _ => return StatusCode::BAD_REQUEST.into_response(),
                    };

                    'and_retry: loop {
                        let Some(Ok(())) = handler.send(bead.as_str(), bead::outgoing::Packet::SetPinMode {
                            pin: Arc::<str>::from(pin.as_str()).into(),
                            mode,
                        }).await else {
                            return StatusCode::NOT_FOUND.into_response();
                        };

                        match timeout(RETRY, handler.recv()).await {
                            Ok(Some((recv_bead, bead::incoming::Packet::UnsupportedPinMode { pin: recv_pin, .. })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                return StatusCode::BAD_REQUEST.into_response();
                            },
                            Ok(Some((recv_bead, bead::incoming::Packet::PinModeResponse { pin: recv_pin, mode })))
                            if recv_bead.as_ref() == bead && recv_pin.as_ref() == pin => {
                                return mode.to_string().into_response();
                            },
                            Ok(Some(_)) | Err(_) => continue 'and_retry,
                            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                        }
                    }
                })
            })
    }
}