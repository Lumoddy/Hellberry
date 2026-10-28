use std::collections::HashMap;
use std::future::{pending, poll_fn};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::extract::ws::Utf8Bytes;
use axum::extract::{WebSocketUpgrade, ws};
use axum::routing;
use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};
use tokio::sync::{Mutex, mpsc};
use tokio::time::interval;
use tokio::{io, select};
use tokio_serial::SerialPortBuilderExt;

use crate::bead::{bead, handler};
use crate::log::*;

pub struct Router {
    pub config_name: Option<String>,
    pub serial_handler: Arc<handler::Handler>,
}

impl From<Router> for routing::Router {
    fn from(value: Router) -> Self {
        routing::Router::new()
            .route("/ws", routing::get(async move |ws| WebSocketUpgrade::on_upgrade(ws, {
                let config_name = value.config_name;
                let handler = value.serial_handler;

                async move |mut ws| {
                    let mut update_interval = interval(Duration::from_secs_f32(0.5));
                    let mut setup_list = <Vec<(
                        Arc<str>,
                        Pin<Box<dyn Future<Output = Result<
                            (mpsc::Sender<bead::Request>, bead::Info),
                            (tokio_serial::SerialStream, bead::Error<io::Error>)>>>>)>>::new();

                    loop {
                        async fn first_of<'a, I, F>(mut f: impl FnMut() -> I)
                            -> (usize, F::Output)
                        where
                            I: IntoIterator<Item = Pin<&'a mut F>>,
                            F: Future + 'a,
                        {
                            poll_fn(move |cx| {
                                for (i, f) in f().into_iter().enumerate() {
                                    if let Poll::Ready(x) = f.poll(cx) {
                                        return Poll::Ready((i, x));
                                    }
                                }

                                Poll::Pending
                            })
                                .await
                        }

                        async fn ok_or_never<F, T, E>(f: F) -> T
                        where
                            F: IntoFuture<Output = Result<T, E>>,
                        {
                            match f.await {
                                Ok(x) => return x,
                                Err(_) => pending().await,
                            }
                        }

                        select! {
                            biased;

                            msg = ws.recv() => {
                                let msg = match msg {
                                    Some(Ok(ws::Message::Text(x))) => x.into(),
                                    Some(Ok(ws::Message::Binary(x))) => x,
                                    Some(Ok(ws::Message::Close(_))) | None => break,
                                    Some(Ok(ws::Message::Ping(_) | ws::Message::Pong(_))) => continue,
                                    Some(Err(x)) => {
                                        eprintln!("{Error}: Failed to receive message from websocket: {}.", x);
                                        break;
                                    },
                                };
                            },
                            (name, config) = poll_fn(move |cx| {
                                for (i, (name, f)) in setup_list.iter_mut().enumerate() {
                                    if let Poll::Ready(x) = f.as_mut().poll(cx) {
                                        let name = name.clone();
                                        setup_list.swap_remove(i);
                                        return Poll::Ready((name, x));
                                    }
                                }

                                Poll::Pending
                            }) => {

                                if let Err(_) = ws.send(ws::Message::Text(Utf8Bytes::from(
                                    format!("beads.connect(\"{name}\")")))).await {
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

                                const ARDUINO_VID: u16 = 0x2341;
                                const ARDUINO_PID: u16 = 0x0043;

                                match tokio_serial::available_ports() {
                                    Ok(available_ports) => {
                                        for info in available_ports {
                                            let SerialPortInfo {
                                                port_name,
                                                port_type: SerialPortType::UsbPort(UsbPortInfo {
                                                    vid: ARDUINO_VID,
                                                    pid: ARDUINO_PID,
                                                    serial_number,
                                                    ..
                                                }),
                                            } = info else {
                                                continue;
                                            };

                                            let name = Arc::<str>::from(port_name.as_str());
                                            match tokio_serial::new(port_name, 9600).open_native_async() {
                                                Ok(serial) => {
                                                    setup_list.push((name, Box::pin(bead::handle_serial(serial))));
                                                },
                                                Err(error) => {
                                                    eprintln!("{Error} Failed to connect to {name}.");
                                                },
                                            }
                                        }
                                    },
                                    Err(error) => {
                                        eprintln!("{Error} Failed to search for connected serial ports.");
                                    },
                                }
                            },
                        }
                    }
                }
            })))
    }
}