use std::sync::Arc;

use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_serial::SerialPortBuilderExt;

use crate::bead::{PinFlags, PinMode};
use crate::log::*;

use super::bead;

#[derive(Clone, Debug)]
pub enum Event {
    Opened { bead: Arc<str>, info: bead::Info },
    Closed { bead: Arc<str> },
    PinListen { bead: Arc<str>, pin: Arc<str>, power: f32 },
}

#[derive(Debug)]
pub struct Handler {
    connected: Mutex<Vec<(Arc<str>, bead::Info, mpsc::Sender<bead::Request>)>>,
    events: broadcast::Sender<Event>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            connected: Mutex::new(Vec::new()),
            events: broadcast::Sender::new(64),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub async fn update_serial(&self) {
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
                            serial_number: Some(serial_number),
                            ..
                        }),
                    } = info else {
                        continue;
                    };

                    let mut lock = self.connected.lock().await;

                    if lock.iter().any(|(name, ..)| **name == *serial_number) {
                        continue;
                    }

                    let name = Arc::<str>::from(serial_number);
                    match tokio_serial::new(&port_name, 9600).open_native_async() {
                        Ok(serial) => {
                            if let Ok((info, sender)) = bead::handle_serial(&name, serial, &self.events).await {
                                lock.push((name, info, sender));
                            }
                        },
                        Err(error) => {
                            eprintln!("{Error} Failed to connect to {port_name}: {error}.");
                        },
                    }

                    return;
                }
            },
            Err(error) => {
                eprintln!("{Error} Failed to search for connected serial ports: {error}.");
            },
        }
    }

    pub async fn get_bead_info(&self, bead: &str) -> Option<bead::Info> {
        self.connected.lock().await.iter()
            .find(|(name, ..)| **name == *bead)
            .map(|(_, info, _)| info.clone())
    }

    pub async fn ping_bead(&self, bead: &str)
        -> Option<Result<(), &'static str>> {
        let lock = self.connected.lock().await;
        let Some((.., sender)) = lock.iter().find(|(name, ..)| **name == *bead) else {
            return None;
        };

        let sender = sender.clone();
        drop(lock);
        let (response, wait) = oneshot::channel();
        sender.send(bead::Request::Ping { response }).await.ok()?;
        wait.await.ok()
    }

    pub async fn get_bead_pin_power(&self, bead: &str, pin: Arc<str>)
        -> Option<Result<f32, &'static str>> {
        let lock = self.connected.lock().await;
        let Some((.., sender)) = lock.iter().find(|(name, ..)| **name == *bead) else {
            return None;
        };

        let sender = sender.clone();
        drop(lock);
        let (response, wait) = oneshot::channel();
        sender.send(bead::Request::GetPinPower { pin, response }).await.ok()?;
        wait.await.ok()
    }

    pub async fn get_bead_pin_mode(&self, bead: &str, pin: Arc<str>)
        -> Option<Result<PinMode, &'static str>> {
        let lock = self.connected.lock().await;
        let Some((.., sender)) = lock.iter().find(|(name, ..)| **name == *bead) else {
            return None;
        };

        let sender = sender.clone();
        drop(lock);
        let (response, wait) = oneshot::channel();
        sender.send(bead::Request::GetPinMode { pin, response }).await.ok()?;
        wait.await.ok()
    }

    pub async fn get_bead_pin_flags(&self, bead: &str, pin: Arc<str>)
        -> Option<Result<PinFlags, &'static str>> {
        let lock = self.connected.lock().await;
        let Some((.., sender)) = lock.iter().find(|(name, ..)| **name == *bead) else {
            return None;
        };

        let sender = sender.clone();
        drop(lock);
        let (response, wait) = oneshot::channel();
        sender.send(bead::Request::GetPinFlags { pin, response }).await.ok()?;
        wait.await.ok()
    }

    pub async fn set_bead_pin_power(&self, bead: &str, pin: Arc<str>, power: f32)
        -> Option<Result<f32, &'static str>> {
        let lock = self.connected.lock().await;
        let Some((.., sender)) = lock.iter().find(|(name, ..)| **name == *bead) else {
            return None;
        };

        let sender = sender.clone();
        drop(lock);
        let (response, wait) = oneshot::channel();
        sender.send(bead::Request::SetPinPower { pin, power, response }).await.ok()?;
        wait.await.ok()
    }

    pub async fn set_bead_pin_mode(&self, bead: &str, pin: Arc<str>, mode: PinMode)
        -> Option<Result<PinMode, &'static str>> {
        let lock = self.connected.lock().await;
        let Some((.., sender)) = lock.iter().find(|(name, ..)| **name == *bead) else {
            return None;
        };

        let sender = sender.clone();
        drop(lock);
        let (response, wait) = oneshot::channel();
        sender.send(bead::Request::SetPinMode { pin, mode, response }).await.ok()?;
        wait.await.ok()
    }
}