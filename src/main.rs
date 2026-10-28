use axum::routing;
use bead::handler;
use router::Router;
use tokio::time::interval;
use tokio::{select, signal};
use tokio::net::{TcpListener, UdpSocket};

use std::ffi::OsString;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

mod args;

mod bead;

mod router;

mod broadcast;

mod log;
use log::*;

#[tokio::main]
async fn main() {
    match args::parse_args(std::env::args_os().skip(1).map(OsString::into_encoded_bytes)) {
        Ok(Some(args::Command::Run { listen_port, broadcast_port, berry_name })) => {
            let local_broadcast_ip = local_ip_address::local_broadcast_ip().unwrap();
            let local_ip = local_ip_address::local_ip().unwrap();

            let listen_port = listen_port.unwrap_or(0);
            let broadcast_port = broadcast_port.unwrap_or(0);

            let listener = match TcpListener::bind(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                listen_port)).await
            {
                Ok(x) => x,
                Err(error) => {
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
                Err(error) => {
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
                Err(error) => {
                    eprintln!("{Error} Failed to connect broadcaster on port {broadcast_port}: {error}.");
                    return;
                },
            }

            match broadcaster.peer_addr()
            {
                Ok(addr) => {
                    eprintln!("{Success} Started broadcasting to {}.", addr);
                },
                Err(error) => {
                    eprintln!("{Error} Failed to verify broadcaster on port {broadcast_port}: {error}.");
                    return;
                },
            }

            tokio::spawn({
                let message = broadcast::message(
                    berry_name.as_deref(),
                    SocketAddr::new(local_ip, broadcast_port));
                let mut interval = interval(Duration::from_secs_f32(1.0));
                async move {
                    loop {
                        select! {
                            _ = interval.tick() => {
                                if let Err(error) = broadcaster.send(message.as_bytes()).await {
                                    eprintln!("{Error} Failed to broadcast message: {error}.");
                                }
                            },
                            () = shutdown_signal() => {
                                break;
                            },
                        }
                    }
                }
            });

            axum::serve(
                listener,
                routing::Router::from(Router {
                    config_name: berry_name,
                    serial_handler: Arc::new(handler::Handler::new()),
                }))
                .with_graceful_shutdown(shutdown_signal())
                .await
                .unwrap();
        },
        Ok(Some(args::Command::Help) | None) => {
            show_help();
        },
        Err(error) => {
            eprintln!("{Error} {error}.");
        },
    }
}

fn show_help()
{
    eprintln!("\
{Success} Relays the state of the connected \"Hellberry Beads\" (usually Arduinos) through a hosted http server with websocket support.

  {ANSICyan}run{ANSIReset} Starts the server.
    {ANSIYellow}-listen -listen_port{ANSIReset} The port the http server will listen on.
    {ANSIYellow}-broadcast -broadcast_port{ANSIReset} The port to broadcast to on the local network.
    {ANSIYellow}-name -berry-name{ANSIReset} The name of this Hellberry.

  {ANSICyan}help{ANSIReset} Shows this help message.

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