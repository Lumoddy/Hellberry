use std::collections::{HashMap, hash_map};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::select;
use tokio::sync::{Mutex, broadcast, mpsc, };
use tokio_serial::{SerialPortBuilderExt, SerialPortType, SerialStream, UsbPortInfo};

use crate::bead::packet::outgoing;
use crate::{de::Deserialize, log::*};

use super::packet::incoming;

pub struct SerialHandler {
    connected: Mutex<HashMap<Arc<str>, mpsc::Sender<outgoing::Packet>>>,
    output: broadcast::Sender<(Arc<str>, incoming::Packet)>,
}

impl SerialHandler {
    pub fn new() -> Self {
        Self {
            connected: Mutex::new(HashMap::new()),
            output: broadcast::Sender::new(100),
        }
    }

    pub async fn update(self: Arc<Self>, mut new: impl FnMut(Arc<str>)) {
        let available_ports = tokio_serial::available_ports().unwrap();

        let mut lock = self.connected.lock().await;

        for info in available_ports {
            if let SerialPortType::UsbPort(UsbPortInfo {
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
                                        stream,
                                        self.output.clone());
                                    entry.insert(tx);
                                    tokio::spawn(fut);
                                    new(serial_number);
                                },
                            }
                        },
                    }
                }
            }
        }
    }
}

const ANALOG_READ_MAX: u16 = 1023;
const ANALOG_WRITE_MAX: u8 = 255;

fn handle(
    handler: Arc<SerialHandler>,
    id: Arc<str>,
    stream: SerialStream,
    output: broadcast::Sender<(Arc<str>, incoming::Packet)>) -> (
    mpsc::Sender<outgoing::Packet>,
    impl Future<Output = ()> + Send + 'static)
{
    const RETRY_DELAY: Duration = Duration::from_millis(500);

    let mut stream = BufReader::new(stream);

    let mut tx_buffer = String::new();
    let mut rx_buffer = String::new();

    let (tx_incoming, mut rx_incoming) = mpsc::channel(100);

    (
        tx_incoming,
        async move
        {
            'and_close:
            {
                match (&mut stream).take(1024).read_line(&mut rx_buffer).await {
                    Ok(0) => {
                        break 'and_close;
                    },
                    Ok(_) if rx_buffer.trim_ascii() == "ready" => {
                        rx_buffer.clear();
                    },
                    Ok(_) => {
                        if let Some((i, _)) = rx_buffer.char_indices().nth(32) {
                            eprintln!("{Error} Serial sent unknown init: \"{}...\".", &rx_buffer[..i].escape_debug());
                        } else {
                            eprintln!("{Error} Serial sent unknown init: \"{}\".", rx_buffer.escape_debug());
                        }
                        break 'and_close;
                    },
                    Err(x) if x.kind() == std::io::ErrorKind::InvalidData => {
                        eprintln!("{Error} Serial sent non-utf8 data.");
                    },
                    Err(x) => {
                        eprintln!("{Error} Serial reported error: {x}");
                    },
                }

                loop {
                    select! {
                        Some(packet) = rx_incoming.recv() => {
                            writeln!(tx_buffer, "{}", packet);
                            if let Err(error) = stream.write_all(tx_buffer.as_bytes()).await {
                                eprintln!("{Error} Serial reported error: {error}");
                            }
                        },
                        attempt = stream.read_line(&mut rx_buffer) => match attempt {
                            Ok(0) => {
                                break 'and_close;
                            },
                            Ok(_) => match Option::deserialize(rx_buffer.as_bytes()) {
                                Ok(None) => { },
                                Ok(Some(x)) => {
                                    _ = output.send((id.clone(), x));
                                },
                                Err(()) => {
                                    if let Some((i, _)) = rx_buffer.char_indices().nth(32) {
                                        eprintln!("{Error} Serial sent malformed packet: \"{}...\".", &rx_buffer[..i].escape_debug());
                                    } else {
                                        eprintln!("{Error} Serial sent malformed packet: \"{}\".", rx_buffer.escape_debug());
                                    }
                                    break 'and_close;
                                },
                            },
                            Err(x) if x.kind() == std::io::ErrorKind::InvalidData => {
                                eprintln!("{Error} Serial sent non-utf8 data.");
                            },
                            Err(x) => {
                                eprintln!("{Error} Serial reported error: {x}");
                            },
                        },
                    }
                }
            }

            _ = handler.connected.lock().await.remove(&id);
        })
}