use axum::body::Body;
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::response::IntoResponse;
use axum::{Router, routing};
use tokio::fs;

use crate::{CONFIG, CURRENT_DIR, log::*};

pub fn router() -> Router
{
    Router::new()
        .route("/logo", routing::get(async || -> Response<Body>
        {
            let mut path;

            match CONFIG.get().unwrap().lock().await
            {
                lock =>
                {
                    let Some(ref x) = lock.logo_path
                    else { return StatusCode::NOT_FOUND.into_response() };

                    path = CURRENT_DIR.get().unwrap().clone();
                    path.extend(x);
                },
            }

            let file = match fs::read(&path).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Warn}: Failed to connect broadcaster: {}.", error);
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                },
            };

            let mime_type = match path.as_os_str().as_encoded_bytes()
            {
                x if x.ends_with(b".png") || x.ends_with(b".PNG") => "image/png",
                x if x.ends_with(b".jpg") || x.ends_with(b".jpeg")
                    || x.ends_with(b".JPG") || x.ends_with(b".JPEG") => "image/jpeg",
                x if x.ends_with(b".webp") || x.ends_with(b".WEBP") => "image/webp",
                x if x.ends_with(b".svg") || x.ends_with(b".SVG") => "image/svg+xml",
                _ => "application/octet-stream",
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_static(mime_type))
                .body(Body::from(file))
                .unwrap()
        }))
}