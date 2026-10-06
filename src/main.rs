#[cfg(not(unix))]
use std::future::pending;
use std::env;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::PathBuf;

use config::{Configuration, poll_config};
use router::router;
use tokio::signal;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{Mutex, OnceCell};

mod packet;

mod bead;

mod router;

mod config;

mod log;
use log::*;

static CURRENT_DIR: OnceCell<PathBuf> = OnceCell::const_new();
static CONFIG: OnceCell<Mutex<Configuration>> = OnceCell::const_new();

#[tokio::main]
async fn main()
{
    let mut args = env::args_os();

    _ = args.next();

    match args.next().as_ref().map(|x| x.as_encoded_bytes())
    {
        Some(b"run") =>
        {
            let mut listen_port = None;
            let mut broadcast_port = None;

            while let Some(arg) = args.next().as_ref().map(|x| x.as_encoded_bytes())
            {
                match arg
                {
                    b"-listen" | b"-listen-port" => match args.next()
                    {
                        Some(value) =>
                        {
                            if let Some(value) = str::from_utf8(value.as_encoded_bytes())
                                .ok().and_then(|x| x.parse().ok())
                            {
                                if listen_port.is_some()
                                {
                                    eprintln!("{Error}: Duplicate listen port.");
                                    return;
                                }

                                listen_port = Some(value);
                            }
                            else
                            {
                                eprintln!("{Error}: Invalid listen port value \"{:?}\".", value);
                                return;
                            }
                        },
                        None =>
                        {
                            eprintln!("{Error}: Missing value after \"-listen-port\"");
                            return;
                        },
                    },
                    b"-broadcast" | b"-broadcast-port" => match args.next()
                    {
                        Some(value) =>
                        {
                            if let Some(value) = str::from_utf8(value.as_encoded_bytes())
                                .ok().and_then(|x| x.parse().ok())
                            {
                                if broadcast_port.is_some()
                                {
                                    eprintln!("{Error}: Duplicate broadcast port.");
                                    return;
                                }

                                broadcast_port = Some(value);
                            }
                            else
                            {
                                eprintln!("{Error}: Invalid broadcast port value \"{:?}\".", value);
                                return;
                            }
                        },
                        None =>
                        {
                            eprintln!("{Error}: Missing value after \"-broadcast-port\"");
                            return;
                        },
                    },
                    value =>
                    {
                        eprintln!("{Error}: Invalid arg \"{:?}\".", value);
                        return;
                    },
                }
            }

            let local_broadcast_ip = local_ip_address::local_broadcast_ip().unwrap();

            CURRENT_DIR.set(env::current_dir().unwrap()).unwrap();

            let config = poll_config().await.unwrap_or_default();

            let listener = match TcpListener::bind(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                listen_port.or(config.listen_port).unwrap_or(0))).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Error}: Failed to start listener: {}.", error);
                    return;
                },
            };

            eprintln!("{Success}: Started listener on port {}.", listener.local_addr().unwrap().port());

            let broadcaster = match UdpSocket::bind(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                0)).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Error}: Failed to start broadcaster: {}.", error);
                    return;
                },
            };

            broadcaster.set_broadcast(true).unwrap();

            if config.broadcast_port.is_none()
            {
                eprintln!("{Error}: Broadcast port is not specified.");
                return;
            }

            match broadcaster.connect(SocketAddr::new(
                local_broadcast_ip,
                broadcast_port.or(config.broadcast_port).unwrap_or(0))).await
            {
                Ok(x) => x,
                Err(error) =>
                {
                    eprintln!("{Error}: Failed to connect broadcaster: {}.", error);
                    return;
                },
            }

            eprintln!("{Success}: Started broadcaster targeting {}.", broadcaster.peer_addr().unwrap());

            CONFIG.set(Mutex::new(config)).unwrap();

            tokio::spawn(axum::serve(listener, router())
                .with_graceful_shutdown(shutdown_signal())
                .into_future());
        },
        _ =>
        {
            eprintln!("{Success}: Relays the state of the connected Hellberry Beads (usually Arduinos) through a hosted http server with websocket support\
\
  {ANSICyan}run{ANSIReset} Starts the http server.\
    {ANSIYellow}-listen -listen_port{ANSIReset} The port the http server will listen on.\
    {ANSIYellow}-broadcast -broadcast_port{ANSIReset} The port to broadcast to on the local network.\
\
A config file at \'./config.json\' or \'./config.yaml\' can be used to specify more options:\
\
  {ANSIPurple}\"name\"{ANSIReset} The name of this Hellberry.\
  {ANSIPurple}\"logo-path\"{ANSIReset} The relative path to a logo image.\
  {ANSIPurple}\"simple-logo-path\"{ANSIReset} Single color version of the logo image.\
  {ANSIPurple}\"listen-port\"{ANSIReset} The port the http server will listen on.\
  {ANSIPurple}\"broadcast-port\"{ANSIReset} The port to broadcast to on the local network.\
");
        },
    }
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
    let terminate = pending();

    tokio::select!
    {
        _ = ctrl_c => (),
        _ = terminate => (),
    }
}