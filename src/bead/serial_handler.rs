use std::collections::{HashMap, hash_map};
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tokio::{join, select};
use tokio::sync::Mutex;
use tokio_serial::SerialPortType::UsbPort;
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType, UsbPortInfo};

use crate::shutdown_signal;

use super::Config;

#[cfg(unix)]
type SerialPort = serialport::TTYPort;

#[cfg(windows)]
type SerialPort = serialport::COMPort;

pub struct SerialHandler
{
    connected: Mutex<HashMap<String, (Arc<Mutex<SerialPort>>, Config)>>,
}

impl SerialHandler
{
    async fn update(&self)
    {
        join_all(tokio_serial::available_ports().unwrap().into_iter().map(async move |info|
        {
            if let SerialPortType::UsbPort(UsbPortInfo
            {
                manufacturer: Some(manufacturer),
                serial_number: Some(serial_number),
                ..
            }) = info.port_type
            {
                if manufacturer.contains("arduino") || manufacturer.contains("Arduino")
                {
                    if self.connected.lock().await.contains_key(&serial_number)
                    {
                        match tokio_serial::new(info.port_name, 9600).open_native_async()
                        {
                            Err(_) => (),
                            Ok(x) =>
                            {
                                x.
                            },
                        }
                    }

                    if let hash_map::Entry::Vacant(x) = connected.entry(serial_number)
                    {
                        
                    }
                }
            }
        }))
            .await;
    }
}