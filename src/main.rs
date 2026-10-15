// use std::env;
// use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
// use std::path::PathBuf;
// use std::sync::Arc;
// use std::time::Duration;

// use bead::serial_handler::SerialHandler;
// use config::{Configuration, read_config};
// use router::router;
// use tokio::time::timeout;
// use tokio::{select, signal};
// use tokio::net::{TcpListener, UdpSocket};
// use tokio::sync::OnceCell;

use axum::routing;
use bead::serial_handler::SerialHandler;
use broadcaster::Broadcaster;
use router::Router;
use tokio::time::interval;
use tokio::{io, signal};
use tokio::net::{TcpListener, UdpSocket};

use std::convert::Infallible;
use std::env;
use std::ffi::OsString;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::ops::ControlFlow;
use std::time::Duration;

mod args;

mod config;

mod packet;

mod bead;

mod router;

mod broadcaster;

mod log;
use log::*;

#[tokio::main]
async fn main()
{
    match args::parse_args(std::env::args_os().skip(1).map(OsString::into_encoded_bytes))
    {
        Ok(Some(args::Command::Run { listen_port, broadcast_port, berry_name })) =>
        {
            let config = match config::read_config(&env::current_dir().unwrap()).await
            {
                Ok(x) => x,
                Err(errors) =>
                {
                    if errors.iter().all(|(_, error)| matches!(
                        error,
                        config::ReadError::Io(error) if error.kind() == io::ErrorKind::NotFound))
                    {
                        config::Configuration::default()
                    }
                    else if errors.iter().any(|(_, error)| matches!(
                        error,
                        config::ReadError::JsonLenient(_) | config::ReadError::Yaml(_)))
                    {
                        for (name, error) in errors
                        {
                            if matches!(error, config::ReadError::Io(_)) { continue };

                            eprintln!("{Error} Failed to parse '{name}': {error}");
                        }

                        return;
                    }
                    else
                    {
                        for (name, error) in errors
                        {
                            eprintln!("{Error} Failed to read '{name}': {error}");
                        }

                        return;
                    }
                },
            };

            let listen_port = listen_port.or(config.listen_port).unwrap_or(0);
            let broadcast_port = broadcast_port.or(config.broadcast_port).unwrap_or(0);
            let berry_name = berry_name.or(config.berry_name);

            let local_broadcast_ip = local_ip_address::local_broadcast_ip().unwrap();
            let local_ip = local_ip_address::local_ip().unwrap();

            let listener = match TcpListener::bind(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                listen_port)).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Error} Failed to start listener on port {listen_port}: {error}.");
                    return;
                },
            };

            eprintln!("{Success} Started listening on {}.", listener.local_addr().unwrap());

            let broadcaster = match UdpSocket::bind(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                0)).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Error} Failed to start broadcaster: {error}.");
                    return;
                },
            };

            broadcaster.set_broadcast(true).unwrap();

            match broadcaster.connect(SocketAddr::new(
                local_broadcast_ip,
                broadcast_port)).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Error} Failed to connect broadcaster on port {broadcast_port}: {error}.");
                    return;
                },
            }

            eprintln!("{Success} Started broadcasting to {}.", broadcaster.peer_addr().unwrap());

            tokio::spawn(Broadcaster
            {
                socket: broadcaster,
                message: Vec::from(match &berry_name
                {
                    Some(name) => format!("[Hellberry {name}:{local_ip}]"),
                    None => format!("[Hellberry:{local_ip}]"),
                }),
                interval: interval(Duration::from_secs_f32(1.0)),
                on_error: async |error|
                {
                    eprintln!("{Warn} Failed to broadcast: {error}.");
                    ControlFlow::Continue::<Infallible>(())
                }
            }
                .run());

            axum::serve(
                listener,
                routing::Router::from(Router
                {
                    config_name: berry_name,
                    logo_path: config.logo_path,
                    simple_logo_path: config.simple_logo_path,
                    serial_handler: SerialHandler::new().into(),
                }))
                .await
                .unwrap();
        },
        Ok(Some(args::Command::Help) | None) =>
        {
            show_help();
        },
        Err(error) =>
        {
            eprintln!("{Error} {error}.");
        },
    }
}

fn show_help()
{
    eprintln!("\
{Success} Relays the state of the connected Hellberry Beads (usually Arduinos) through a hosted http server with websocket support.

  {ANSICyan}run{ANSIReset} Starts the http server.
    {ANSIYellow}-listen -listen_port{ANSIReset} The port the http server will listen on.
    {ANSIYellow}-broadcast -broadcast_port{ANSIReset} The port to broadcast to on the local network.

A config file at \'./config.json\' or \'./config.yaml\' can be used to specify more options:

  {ANSIPurple}\"name\"{ANSIReset} The name of this Hellberry.
  {ANSIPurple}\"logo-path\"{ANSIReset} The relative path to a logo image.
  {ANSIPurple}\"simple-logo-path\"{ANSIReset} Single color version of the logo image.
  {ANSIPurple}\"listen-port\"{ANSIReset} The port the http server will listen on.
  {ANSIPurple}\"broadcast-port\"{ANSIReset} The port to broadcast to on the local network.
");
}

async fn shutdown_signal()
{
    let ctrl_c = async
    {
        signal::ctrl_c().await.unwrap();
    };

    #[cfg(unix)]
    let terminate = async
    {
        signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap().recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending();

    tokio::select!
    {
        _ = ctrl_c => (),
        _ = terminate => (),
    }
}