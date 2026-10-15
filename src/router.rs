use std::future::{pending, poll_fn};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::body::Body;
use axum::extract::ws::Utf8Bytes;
use axum::extract::{WebSocketUpgrade, ws};
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing;
use tokio::time::interval;
use tokio::{fs, select};

use crate::bead::serial_handler::{GetPinModeError, GetPinPowerError, PingError, SerialHandler, SetPinModeError, SetPinPowerError};
use crate::bead::{Config, ConfigPin, PinMode};
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
    pub config_name: Option<String>,
    pub logo_path: Option<PathBuf>,
    pub simple_logo_path: Option<PathBuf>,
    pub serial_handler: Arc<SerialHandler>,
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
                        Ping { bead_name: String },
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
                        Pong { bead_name: &'a str },
                        Config
                        {
                            #[serde(skip_serializing_if = "Option::is_none")]
                            name: Option<&'a str>,
                        },
                        BeadConnected { bead_name: &'a str, pins: &'a [ConfigPin] },
                        BeadDisconnected { bead_name: &'a str },
                        BeadDisconnectedInterruption { bead_name: &'a str },
                        GetBeadPinPowerResponse { bead_name: &'a str, pin_name: &'a str, power: f32 },
                        GetBeadPinModeResponse { bead_name: &'a str, pin_name: &'a str, mode: PinMode },
                        SetBeadPinPowerResponse { bead_name: &'a str, pin_name: &'a str },
                        SetBeadPinModeResponse { bead_name: &'a str, pin_name: &'a str },
                        BeadPinListen { bead_name: &'a str, pin_name: &'a str, power: f32 },
                        InvalidBeadName { bead_name: &'a str },
                        InvalidBeadPinName { bead_name: &'a str, pin_name: &'a str },
                        InvalidWriteToInput { bead_name: &'a str, pin_name: &'a str, power: f32 },
                        InvalidUnsupportedMode { bead_name: &'a str, pin_name: &'a str, mode: PinMode },
                        InvalidIncoming,
                    }

                    let mut update_interval = interval(Duration::from_secs_f32(0.5));
                    let mut setup_list = Vec::new();

                    loop
                    {
                        fn poll_many<F>(source: &mut Vec<Pin<Box<F>>>)
                            -> impl FnMut(&mut Context<'_>) -> Poll<F::Output>
                        where
                            F: Future,
                        {
                            move |cx|
                            {
                                for f in source.iter_mut()
                                {
                                    if let Poll::Ready(x) = f.as_mut().poll(cx) { return Poll::Ready(x) };
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

                        select!
                        {
                            biased;

                            msg = ws.recv() =>
                            {
                                let msg = match msg
                                {
                                    Some(Ok(ws::Message::Text(x))) => x.into(),
                                    Some(Ok(ws::Message::Binary(x))) => x,
                                    Some(Ok(ws::Message::Close(_))) | None => break,
                                    Some(Ok(ws::Message::Ping(_) | ws::Message::Pong(_))) => continue,
                                    Some(Err(x)) =>
                                    {
                                        eprintln!("{Error}: Failed to receive message from websocket: {}.", x);
                                        break;
                                    },
                                };

                                match serde_json::from_slice(&msg)
                                {
                                    Ok(Incoming::Ping { bead_name }) =>
                                    {
                                        match handler.ping(&bead_name).await
                                        {
                                            None =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadName
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Ok(())) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::Pong
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(PingError::BeadClosed)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::BeadDisconnectedInterruption
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                        }
                                    },
                                    Ok(Incoming::WholeConfig) =>
                                    {
                                        if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::Config
                                            {
                                                name: config_name.as_deref(),
                                            }).unwrap()))).await
                                        {
                                            break;
                                        }
                                    },
                                    Ok(Incoming::GetBeadPinPower { bead_name, pin_name }) =>
                                    {
                                        match handler.get_pin_power(&bead_name, &pin_name).await
                                        {
                                            None =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Ok(power)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::GetBeadPinPowerResponse
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                        power,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(GetPinPowerError::BeadClosed)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::BeadDisconnectedInterruption
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(GetPinPowerError::InvalidBeadPinName)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
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
                                            None =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Ok(mode)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::GetBeadPinModeResponse
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                        mode,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(GetPinModeError::BeadClosed)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::BeadDisconnectedInterruption
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(GetPinModeError::InvalidBeadPinName)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
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
                                            None =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Ok(_)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::SetBeadPinPowerResponse
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(SetPinPowerError::BeadClosed)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::BeadDisconnectedInterruption
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(SetPinPowerError::InvalidBeadPinName)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(SetPinPowerError::InvalidWriteToInput { power })) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidWriteToInput
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                        power,
                                                    }).unwrap()))).await
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
                                            None =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Ok(_)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::SetBeadPinModeResponse
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(SetPinModeError::BeadClosed)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::BeadDisconnectedInterruption
                                                    {
                                                        bead_name: &bead_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(SetPinModeError::InvalidBeadPinName)) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidBeadPinName
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                            Some(Err(SetPinModeError::InvalidUnsupportedMode { mode })) =>
                                            {
                                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                                    serde_json::to_string(&Outgoing::InvalidUnsupportedMode
                                                    {
                                                        bead_name: &bead_name,
                                                        pin_name: &pin_name,
                                                        mode,
                                                    }).unwrap()))).await
                                                {
                                                    break;
                                                }
                                            },
                                        }
                                    },
                                    Err(_) =>
                                    {
                                        if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                            serde_json::to_string(&Outgoing::InvalidIncoming).unwrap()))).await
                                        {
                                            break;
                                        }
                                    },
                                }
                            },
                            config = poll_fn(poll_many(&mut setup_list)) =>
                            {
                                let config: Config = config;

                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Outgoing::BeadConnected
                                    {
                                        bead_name: &config.name,
                                        pins: &config.pins,
                                    }).unwrap()))).await
                                {
                                    break;
                                }
                            },
                            Ok(bead_name) = handler.next_closed() =>
                            {
                                eprintln!("{Success} Bead '{bead_name}' closed.");

                                _ = ws.send(ws::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Outgoing::BeadDisconnected
                                    {
                                        bead_name: &bead_name,
                                    }).unwrap()))).await;

                                break;
                            },
                            Ok((bead_name, pin_name, power)) = handler.next_listen() =>
                            {
                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&Outgoing::BeadPinListen
                                    {
                                        bead_name: &bead_name,
                                        pin_name: &pin_name,
                                        power,
                                    }).unwrap()))).await
                                {
                                    break;
                                }
                            },
                            _ = update_interval.tick() =>
                            {
                                let mut new = Vec::new();

                                handler.update(|x| new.push(x)).await;

                                for new in new
                                {
                                    let handler = handler.clone();
                                    setup_list.push(Box::pin(ok_or_never(async move
                                    {
                                        handler.config(&new).await.unwrap()
                                    })));
                                }
                            },
                        }
                    }
                }
            })))
    }
}