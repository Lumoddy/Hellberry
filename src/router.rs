use axum::body::Body;
use axum::extract::ws::Utf8Bytes;
use axum::extract::{WebSocketUpgrade, ws};
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::response::IntoResponse;
use axum::{Router, routing};
use tokio::fs;

use crate::bead::PinMode;
use crate::{CONFIG, CURRENT_DIR, log::*};

pub fn media_type_of_bytes(path: &[u8]) -> &'static str
{
    match path
    {
        x if x.ends_with(b".png") || x.ends_with(b".PNG") => "image/png",
        x if x.ends_with(b".jpg") || x.ends_with(b".JPG")
            || x.ends_with(b".jpeg") || x.ends_with(b".JPEG") => "image/jpeg",
        x if x.ends_with(b".webp") || x.ends_with(b".WEBP") => "image/webp",
        x if x.ends_with(b".svg") || x.ends_with(b".SVG") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

pub fn router() -> Router
{
    Router::new()
        .route("/logo", routing::get(async move || -> Response<Body>
        {
            let mut path;

            if let Some(ref x) = CONFIG.get().unwrap().logo_path
            {
                path = CURRENT_DIR.get().unwrap().clone();
                path.extend(x);
            }
            else { return StatusCode::NOT_FOUND.into_response() }

            let file = match fs::read(&path).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Warn}: Failed to connect broadcaster: {}.", error);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                },
            };

            let mime_type = media_type_of_bytes(path.as_os_str().as_encoded_bytes());

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_static(mime_type))
                .body(Body::from(file))
                .unwrap()
        }))
        .route("/simple-logo", routing::get(async move || -> Response<Body>
        {
            let mut path;

            if let Some(ref x) = CONFIG.get().unwrap().simple_logo_path
            {
                path = CURRENT_DIR.get().unwrap().clone();
                path.extend(x);
            }
            else { return StatusCode::NOT_FOUND.into_response() }

            let file = match fs::read(&path).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Warn}: Failed to connect broadcaster: {}.", error);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                },
            };

            let mime_type = media_type_of_bytes(path.as_os_str().as_encoded_bytes());

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_static(mime_type))
                .body(Body::from(file))
                .unwrap()
        }))
        .route("/ws", routing::get(async move |ws| WebSocketUpgrade::on_upgrade(ws, async move |mut ws|
        {
            #[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
            #[serde(tag = "tag", rename_all = "kebab-case")]
            enum Incoming
            {
                Ping,
                WholeConfig,
                GetBeadPinPower { bead_name: String, pin_name: String, power: u16 },
                GetBeadPinMode { bead_name: String, pin_name: String, mode: PinMode },
                SetBeadPinPower { bead_name: String, pin_name: String, power: u16 },
                SetBeadPinMode { bead_name: String, pin_name: String, mode: PinMode },
            }

            #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
            #[serde(tag = "tag", rename_all = "kebab-case")]
            enum Outgoing
            {
                Pong,
                Config
                {
                    #[serde(skip_serializing_if = "Option::is_none")]
                    name: Option<String>,
                },
                GetBeadPinPowerResponse { bead_name: String, pin_name: String, power: u16 },
                GetBeadPinModeResponse { bead_name: String, pin_name: String, mode: PinMode },
                SetBeadPinPowerResponse { bead_name: String, pin_name: String, power: u16 },
                SetBeadPinModeResponse { bead_name: String, pin_name: String, mode: PinMode },
                BeadPinListen { bead_name: String, pin_name: String, power: u16 },
                InvalidBeadPinName { pin_name: String },
                InvalidWriteToInput { pin_name: String, power: u16 },
                InvalidUnsupportedMode { pin_name: String, mode: PinMode },
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
                                name: CONFIG.get().unwrap().name.clone(),
                            }).unwrap()))).await.is_err()
                        {
                            break;
                        }
                    },
                    Ok(Incoming::GetBeadPinPower { bead_name, pin_name, power }) =>
                    {
                        todo!()
                    },
                    Ok(Incoming::GetBeadPinMode { bead_name, pin_name, mode }) =>
                    {
                        todo!()
                    },
                    Ok(Incoming::SetBeadPinPower { bead_name, pin_name, power }) =>
                    {
                        todo!()
                    },
                    Ok(Incoming::SetBeadPinMode { bead_name, pin_name, mode }) =>
                    {
                        todo!()
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
        })))
}