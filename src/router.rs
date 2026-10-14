use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::Utf8Bytes;
use axum::extract::{WebSocketUpgrade, ws};
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing;
use tokio::fs;

use crate::bead::serial_handler::SerialHandler;
use crate::bead::{ConfigPin, PinMode};
use crate::log::*;

pub fn media_type_of_path(path: &(impl ?Sized + AsRef<[u8]>)) -> &'static str
{
    match path.as_ref()
    {
        x if x.ends_with(b".png") || x.ends_with(b".PNG") => "image/png",
        x if x.ends_with(b".jpg") || x.ends_with(b".JPG")
            || x.ends_with(b".jpeg") || x.ends_with(b".JPEG") => "image/jpeg",
        x if x.ends_with(b".webp") || x.ends_with(b".WEBP") => "image/webp",
        x if x.ends_with(b".svg") || x.ends_with(b".SVG") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

pub struct Router
{
    config_name: Option<String>,
    logo_path: Option<PathBuf>,
    simple_logo_path: Option<PathBuf>,
    serial_handler: Arc<SerialHandler>,
}

impl From<Router> for routing::Router
{
    fn from(value: Router) -> Self
    {
        routing::Router::new()
            .route("/logo", routing::get(
            {
                let path = value.logo_path;

                async move ||
                {
                    let Some(ref path) = path else { return StatusCode::NOT_FOUND.into_response() };

                    let file = match fs::read(&path).await
                    {
                        Ok(x) => x,
                        Err(error) =>
                        {
                            eprintln!("{Warn}: Failed to connect broadcaster: {}.", error);
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        },
                    };

                    let mime_type = media_type_of_path(path.as_os_str().as_encoded_bytes());

                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime_type))
                        .body(Body::from(file))
                        .unwrap()
                }
            }))
            .route("/simple-logo", routing::get(
            {
                let path = value.simple_logo_path;

                async move ||
                {
                    let Some(ref path) = path else { return StatusCode::NOT_FOUND.into_response() };

                    let file = match fs::read(&path).await
                    {
                        Ok(x) => x,
                        Err(error) =>
                        {
                            eprintln!("{Warn}: Failed to connect broadcaster: {}.", error);
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        },
                    };

                    let mime_type = media_type_of_path(path.as_os_str().as_encoded_bytes());

                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, HeaderValue::from_static(mime_type))
                        .body(Body::from(file))
                        .unwrap()
                }
            }))
            .route("/ws", routing::get(async move |ws| WebSocketUpgrade::on_upgrade(ws,
            {
                let config_name = value.config_name;
                let handler = value.serial_handler;

                async move |mut ws|
                {
                    #[derive(Clone, Debug, serde::Deserialize, PartialEq)]
                    #[serde(tag = "type", rename_all = "kebab-case")]
                    enum Incoming
                    {
                        Ping,
                        WholeConfig,
                        GetBeadPinPower { bead_name: String, pin_name: String },
                        GetBeadPinMode { bead_name: String, pin_name: String },
                        SetBeadPinPower { bead_name: String, pin_name: String, power: f32 },
                        SetBeadPinMode { bead_name: String, pin_name: String, mode: PinMode },
                    }

                    #[derive(Clone, Debug, serde::Serialize, PartialEq)]
                    #[serde(tag = "type", rename_all = "kebab-case")]
                    enum Outgoing<'a>
                    {
                        Pong,
                        Config
                        {
                            #[serde(skip_serializing_if = "Option::is_none")]
                            name: Option<&'a str>,
                        },
                        BeadConnected { bead_name: &'a str, pins: &'a [ConfigPin] },
                        BeadDisconnected { bead_name: &'a str },
                        GetBeadPinPowerResponse { bead_name: &'a str, pin_name: &'a str, power: f32 },
                        GetBeadPinModeResponse { bead_name: &'a str, pin_name: &'a str, mode: PinMode },
                        SetBeadPinPowerResponse { bead_name: &'a str, pin_name: &'a str },
                        SetBeadPinModeResponse { bead_name: &'a str, pin_name: &'a str },
                        BeadPinListen { bead_name: &'a str, pin_name: &'a str, power: f32 },
                        InvalidBeadPinName { pin_name: &'a str },
                        InvalidWriteToInput { pin_name: &'a str, power: f32 },
                        InvalidUnsupportedMode { pin_name: &'a str, mode: PinMode },
                        InvalidIncoming,
                    }

                    while let Some(Ok(msg)) = ws.recv().await
                    {
                        let msg = match msg
                        {
                            ws::Message::Text(x) => x.into(),
                            ws::Message::Binary(x) => x,
                            _ => continue,
                        };

                        match serde_json::from_slice(&msg)
                        {
                            Ok(Incoming::Ping) =>
                            {
                                if ws.send(ws::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Outgoing::Pong).unwrap()))).await.is_err()
                                {
                                    break;
                                }
                            },
                            Ok(Incoming::WholeConfig) =>
                            {
                                if ws.send(ws::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Outgoing::Config
                                    {
                                        name: config_name.as_deref(),
                                    }).unwrap()))).await.is_err()
                                {
                                    break;
                                }
                            },
                            Ok(Incoming::GetBeadPinPower { bead_name, pin_name }) =>
                            {
                                match handler.get_pin_power(&bead_name, &pin_name).await
                                {
                                    Ok(power) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::GetBeadPinPowerResponse
                                            {
                                                bead_name: &bead_name,
                                                pin_name: &pin_name,
                                                power,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                    Err(_) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::InvalidBeadPinName
                                            {
                                                pin_name: &pin_name,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                }
                            },
                            Ok(Incoming::GetBeadPinMode { bead_name, pin_name }) =>
                            {
                                match handler.get_pin_mode(&bead_name, &pin_name).await
                                {
                                    Ok(mode) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::GetBeadPinModeResponse
                                            {
                                                bead_name: &bead_name,
                                                pin_name: &pin_name,
                                                mode,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                    Err(_) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::InvalidBeadPinName
                                            {
                                                pin_name: &pin_name,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                }
                            },
                            Ok(Incoming::SetBeadPinPower { bead_name, pin_name, power }) =>
                            {
                                match handler.set_pin_power(&bead_name, &pin_name, power).await
                                {
                                    Ok(()) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::SetBeadPinPowerResponse
                                            {
                                                bead_name: &bead_name,
                                                pin_name: &pin_name,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                    Err(_) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::InvalidBeadPinName
                                            {
                                                pin_name: &pin_name,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                }
                            },
                            Ok(Incoming::SetBeadPinMode { bead_name, pin_name, mode }) =>
                            {
                                match handler.set_pin_mode(&bead_name, &pin_name, mode).await
                                {
                                    Ok(()) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::SetBeadPinModeResponse
                                            {
                                                bead_name: &bead_name,
                                                pin_name: &pin_name,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                    Err(_) =>
                                    {
                                        if ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::InvalidBeadPinName
                                            {
                                                pin_name: &pin_name,
                                            }).unwrap()))).await.is_err()
                                        {
                                            break;
                                        }
                                    },
                                }
                            },
                            Err(_) =>
                            {
                                if ws.send(ws::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Outgoing::InvalidIncoming).unwrap()))).await.is_err()
                                {
                                    break;
                                }
                            },
                        }
                    }
                }
            })))
    }
}