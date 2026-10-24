use std::borrow::Borrow;
use std::collections::{HashMap, hash_map};
use std::fmt::Write;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::select;
use tokio::sync::{Mutex, broadcast, mpsc, };
use tokio::time::timeout;
use tokio_serial::{SerialPortBuilderExt, SerialPortType, SerialStream, UsbPortInfo};

use crate::bead::packet::{outgoing, incoming};
use crate::{de::Deserialize, log::*};

pub struct SerialHandler {
    connected: Mutex<HashMap<Arc<str>, mpsc::Sender<outgoing::Packet<'static>>>>,
    recv: broadcast::Sender<(Arc<str>, incoming::Packet<'static>)>,
    connect: broadcast::Sender<Arc<str>>,
    disconnect: broadcast::Sender<Arc<str>>,
}

impl SerialHandler {
    pub fn new() -> Self {
        Self {
            connected: Mutex::new(HashMap::new()),
            recv: broadcast::Sender::new(100),
            connect: broadcast::Sender::new(4),
            disconnect: broadcast::Sender::new(4),
        }
    }

    pub async fn next_connect(&self) -> Option<Arc<str>> {
        self.connect.subscribe().recv().await.ok()
    }

    pub async fn next_disconnect(&self) -> Option<Arc<str>> {
        self.disconnect.subscribe().recv().await.ok()
    }

    pub async fn send<B: ?Sized>(&self, bead: &B, packet: outgoing::Packet<'static>)
        -> Option<Result<(), mpsc::error::SendError<outgoing::Packet<'static>>>>
    where
        Arc<str>: Borrow<B>,
        B: Hash + Eq,
    {
        let lock = self.connected.lock().await;
        let Some(sender) = lock.get(bead) else {
            return None;
        };
        let sender = sender.clone();
        drop(lock);
        Some(sender.send(packet).await)
    }

    pub async fn recv(&self) -> Option<(Arc<str>, incoming::Packet<'static>)>
    {
        self.recv.subscribe().recv().await.ok()
    }

    pub async fn update(self: &Arc<Self>) {
        let available_ports = tokio_serial::available_ports().unwrap();

        let mut lock = self.connected.lock().await;

        for info in available_ports {
            let SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x2341,
                serial_number: Some(serial_number),
                ..
            }) = info.port_type else {
                continue;
            };

            match tokio_serial::new(info.port_name, 9600).open_native_async()
            {
                Err(_) => (),
                Ok(stream) => {
                    let serial_number = Arc::<str>::from(serial_number);

                    match lock.entry(serial_number.clone())
                    {
                        hash_map::Entry::Occupied(_) => (),
                        hash_map::Entry::Vacant(entry) =>
                        {
                            let (tx, fut) = handle(
                                self.clone(),
                                serial_number.clone(),
                                stream);

                            entry.insert(tx);
                            tokio::spawn(fut);
                        },
                    }
                },
            }
        }
    }

    pub async fn beads(&self) -> Vec<Arc<str>> {
        let lock = self.connected.lock().await;
        lock.keys().cloned().collect()
    }
}

pub const ANALOG_READ_MAX: u16 = 1023;
pub const ANALOG_WRITE_MAX: u8 = 255;

fn handle(handler: Arc<SerialHandler>, id: Arc<str>, stream: SerialStream) -> (
    mpsc::Sender<outgoing::Packet<'static>>,
    impl Future<Output = ()> + Send + 'static)
{
    let mut stream = BufReader::new(stream);

    let mut tx_buffer = String::new();
    let mut rx_buffer = String::new();

    let (tx_incoming, mut rx_incoming) = mpsc::channel(100);

    _ = handler.connect.send(id.clone());

    (
        tx_incoming,
        async move
        {
            'and_close:
            {
                match timeout(Duration::from_millis(500), (&mut stream).take(1024).read_line(&mut rx_buffer)).await {
                    Ok(Ok(0)) => {
                        break 'and_close;
                    },
                    Ok(Ok(_)) if rx_buffer.trim_ascii() == "ready" => {
                        rx_buffer.clear();
                    },
                    Ok(Ok(_)) => {
                        if let Some((i, _)) = rx_buffer.char_indices().nth(32) {
                            eprintln!("{Error} Serial sent unknown init: \"{}...\".", &rx_buffer[..i].escape_debug());
                        } else {
                            eprintln!("{Error} Serial sent unknown init: \"{}\".", rx_buffer.escape_debug());
                        }
                        break 'and_close;
                    },
                    Ok(Err(x)) if x.kind() == std::io::ErrorKind::InvalidData => {
                        eprintln!("{Error} Serial sent non-utf8 data.");
                        break 'and_close;
                    },
                    Ok(Err(x)) => {
                        eprintln!("{Error} Serial reported error: {x}");
                        break 'and_close;
                    },
                    Err(_) => {
                        writeln!(tx_buffer, "{}", outgoing::Packet::Ping { }).unwrap();
                        loop {
                            if let Err(error) = stream.write_all(tx_buffer.as_bytes()).await {
                                eprintln!("{Error} Serial reported error: {error}");
                                break 'and_close;
                            }

                            match timeout(Duration::from_millis(500), (&mut stream).take(1024).read_line(&mut rx_buffer)).await {
                                Ok(Ok(0)) => {
                                    break 'and_close;
                                },
                                Ok(Ok(_)) if rx_buffer.trim_ascii() == "pong" => {
                                    break;
                                },
                                Ok(Ok(_)) => {
                                    if let Some((i, _)) = rx_buffer.char_indices().nth(32) {
                                        eprintln!("{Error} Serial sent unknown init: \"{}...\".", &rx_buffer[..i].escape_debug());
                                    } else {
                                        eprintln!("{Error} Serial sent unknown init: \"{}\".", rx_buffer.escape_debug());
                                    }
                                    break 'and_close;
                                },
                                Ok(Err(x)) if x.kind() == std::io::ErrorKind::InvalidData => {
                                    eprintln!("{Error} Serial sent non-utf8 data.");
                                    break 'and_close;
                                },
                                Ok(Err(x)) => {
                                    eprintln!("{Error} Serial reported error: {x}");
                                    break 'and_close;
                                },
                                Err(_) => {
                                    continue;
                                },
                            }
                        }
                        rx_buffer.clear();
                    },
                }

                loop {
                    select! {
                        biased;

                        Some(packet) = rx_incoming.recv() => {
                            tx_buffer.clear();
                            writeln!(tx_buffer, "{}", packet).unwrap();
                            if let Err(error) = stream.write_all(tx_buffer.as_bytes()).await {
                                eprintln!("{Error} Serial reported error: {error}");
                                break 'and_close;
                            }
                        },
                        attempt = stream.read_line(&mut rx_buffer) => match attempt {
                            Ok(0) => {
                                break 'and_close;
                            },
                            Ok(_) => match Option::deserialize(rx_buffer.as_bytes()) {
                                Ok(None) => { },
                                Ok(Some(x)) => {
                                    match x {
                                        incoming::Packet::MalformedPacket { pos } => {
                                            let pos = pos.min(tx_buffer.len());
                                            eprintln!("{Warn} Serial reported malformed packet: \"{}\" {ANSIRed}<- Here ->{ANSIReset} \"{}\".", &tx_buffer[..pos].escape_debug(), &tx_buffer[pos..].escape_debug());
                                        },
                                        _ => { },
                                    }
                                    _ = handler.recv.send((id.clone(), x.into_static()));
                                    rx_buffer.clear();
                                },
                                Err((i, ())) => {
                                    eprintln!("{Error} Serial sent malformed packet: \"{}\" {ANSIRed}<- Here ->{ANSIReset} \"{}\".", &rx_buffer[..i].escape_debug(), &rx_buffer[i..].escape_debug());
                                    break 'and_close;
                                },
                            },
                            Err(x) if x.kind() == std::io::ErrorKind::InvalidData => {
                                eprintln!("{Error} Serial sent non-utf8 data.");
                                break 'and_close;
                            },
                            Err(x) => {
                                eprintln!("{Error} Serial reported error: {x}");
                                break 'and_close;
                            },
                        },
                    }
                }
            }

            _ = handler.connected.lock().await.remove(&id);
            _ = handler.disconnect.send(id.clone());
        })
}