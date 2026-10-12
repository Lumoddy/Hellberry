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

use config::{Configuration, read_config};
use tokio::io;

use std::env;
use std::ffi::OsString;

mod args;

mod config;

mod log;
use log::*;

#[tokio::main]
async fn main()
{
    match args::parse_args(std::env::args_os().skip(1).map(OsString::into_encoded_bytes))
    {
        Ok(Some(args::Command::Run { listen_port, broadcast_port, berry_name })) =>
        {
            let config = match read_config(&env::current_dir().unwrap()).await
            {
                Ok(x) => x,
                Err(errors) =>
                {
                    if errors.iter().all(|(_, error)| matches!(
                        error,
                        config::ReadError::Io(error) if error.kind() == io::ErrorKind::NotFound))
                    {
                        Configuration::default()
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

            let listen_port = listen_port.or(config.listen_port);
            let broadcast_port = broadcast_port.or(config.broadcast_port);
            let berry_name = berry_name.or(config.berry_name);
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

// mod packet;

// mod bead;

// mod router;

// mod config;

// mod log;
// use log::*;

// static CURRENT_DIR: OnceCell<PathBuf> = OnceCell::const_new();
// static CONFIG: OnceCell<Configuration> = OnceCell::const_new();
// static SERIAL_HANDLER: OnceCell<SerialHandler> = OnceCell::const_new();

// #[tokio::main]
// async fn main()
// {
//     let mut args = env::args_os();

//     _ = args.next();

//     match args.next().as_ref().map(|x| x.as_encoded_bytes())
//     {
//         Some(b"run") =>
//         {
//             let mut listen_port = None;
//             let mut broadcast_port = None;

//             while let Some(arg) = args.next().as_ref().map(|x| x.as_encoded_bytes())
//             {
//                 match arg
//                 {
//                     b"-listen" | b"-listen-port" => match args.next()
//                     {
//                         Some(value) =>
//                         {
//                             if let Some(value) = str::from_utf8(value.as_encoded_bytes())
//                                 .ok().and_then(|x| x.parse().ok())
//                             {
//                                 if listen_port.is_some()
//                                 {
//                                     eprintln!("{Error} Duplicate listen port.");
//                                     return;
//                                 }

//                                 listen_port = Some(value);
//                             }
//                             else
//                             {
//                                 eprintln!("{Error} Invalid listen port value \"{:?}\".", value);
//                                 return;
//                             }
//                         },
//                         None =>
//                         {
//                             eprintln!("{Error} Missing value after \"-listen-port\"");
//                             return;
//                         },
//                     },
//                     b"-broadcast" | b"-broadcast-port" => match args.next()
//                     {
//                         Some(value) =>
//                         {
//                             if let Some(value) = str::from_utf8(value.as_encoded_bytes())
//                                 .ok().and_then(|x| x.parse().ok())
//                             {
//                                 if broadcast_port.is_some()
//                                 {
//                                     eprintln!("{Error} Duplicate broadcast port.");
//                                     return;
//                                 }

//                                 broadcast_port = Some(value);
//                             }
//                             else
//                             {
//                                 eprintln!("{Error} Invalid broadcast port value \"{:?}\".", value);
//                                 return;
//                             }
//                         },
//                         None =>
//                         {
//                             eprintln!("{Error} Missing value after \"-broadcast-port\"");
//                             return;
//                         },
//                     },
//                     value =>
//                     {
//                         eprintln!("{Error} Invalid arg \"{:?}\".", value);
//                         return;
//                     },
//                 }
//             }

//             let local_broadcast_ip = local_ip_address::local_broadcast_ip().unwrap();
//             let local_ip = local_ip_address::local_ip().unwrap();

//             CURRENT_DIR.set(env::current_dir().unwrap()).unwrap();

//             let config = read_config().await.unwrap_or_default();

//             let listener = match TcpListener::bind(SocketAddrV4::new(
//                 Ipv4Addr::new(0, 0, 0, 0),
//                 listen_port.or(config.listen_port).unwrap_or(0))).await
//             {
//                 Ok(x) => x,
//                 Err(error) =>
//                 {
//                     eprintln!("{Error} Failed to start listener: {}.", error);
//                     return;
//                 },
//             };

//             eprintln!("{Success} Started listener on port {}.", listener.local_addr().unwrap().port());

//             let broadcaster = match UdpSocket::bind(SocketAddrV4::new(
//                 Ipv4Addr::new(0, 0, 0, 0),
//                 0)).await
//             {
//                 Ok(x) => x,
//                 Err(error) =>
//                 {
//                     eprintln!("{Error} Failed to start broadcaster: {}.", error);
//                     return;
//                 },
//             };

//             broadcaster.set_broadcast(true).unwrap();

//             if config.broadcast_port.is_none()
//             {
//                 eprintln!("{Error} Broadcast port is not specified.");
//                 return;
//             }

//             match broadcaster.connect(SocketAddr::new(
//                 local_broadcast_ip,
//                 broadcast_port.or(config.broadcast_port).unwrap_or(0))).await
//             {
//                 Ok(x) => x,
//                 Err(error) =>
//                 {
//                     eprintln!("{Error} Failed to connect broadcaster: {}.", error);
//                     return;
//                 },
//             }

//             eprintln!("{Success} Started broadcaster targeting {}.", broadcaster.peer_addr().unwrap());

//             CONFIG.set(config).unwrap();

//             SERIAL_HANDLER.set(SerialHandler::new()).map_err(drop).unwrap();

//             tokio::spawn(async move
//             {
//                 while timeout(
//                     Duration::from_secs_f32(0.5),
//                     shutdown_signal()).await.is_err()
//                 {
//                     SERIAL_HANDLER.get().unwrap().update().await;
//                 }
//             });

//             tokio::spawn(async move
//             {
//                 axum::serve(listener, router(&SERIAL_HANDLER.get().unwrap()))
//                     .with_graceful_shutdown(shutdown_signal())
//                     .await
//             });

//             tokio::spawn(async move
//             {
//                 let message = format!("[Hellberry:{}]", local_ip);

//                 while timeout(
//                     Duration::from_secs_f32(1.0),
//                     shutdown_signal()).await.is_err()
//                 {
//                     if let Err(x) = broadcaster.send(message.as_bytes()).await
//                     {
//                         eprintln!("{Warn} Failed to broadcast: {}.", x);
//                     }
//                 }
//             });
//         },
//         _ =>
//         {
//             eprintln!("{Success} Relays the state of the connected Hellberry Beads (usually Arduinos) through a hosted http server with websocket support.");
//             eprintln!("");
//             eprintln!("  {ANSICyan}run{ANSIReset} Starts the http server.");
//             eprintln!("    {ANSIYellow}-listen -listen_port{ANSIReset} The port the http server will listen on.");
//             eprintln!("    {ANSIYellow}-broadcast -broadcast_port{ANSIReset} The port to broadcast to on the local network.");
//             eprintln!("");
//             eprintln!("A config file at \'./config.json\' or \'./config.yaml\' can be used to specify more options:");
//             eprintln!("");
//             eprintln!("  {ANSIPurple}\"name\"{ANSIReset} The name of this Hellberry.");
//             eprintln!("  {ANSIPurple}\"logo-path\"{ANSIReset} The relative path to a logo image.");
//             eprintln!("  {ANSIPurple}\"simple-logo-path\"{ANSIReset} Single color version of the logo image.");
//             eprintln!("  {ANSIPurple}\"listen-port\"{ANSIReset} The port the http server will listen on.");
//             eprintln!("  {ANSIPurple}\"broadcast-port\"{ANSIReset} The port to broadcast to on the local network.");
//             eprintln!("");
//         },
//     }
// }

// async fn shutdown_signal()
// {
//     let ctrl_c = async
//     {
//         signal::ctrl_c().await.unwrap();
//     };

//     #[cfg(unix)]
//     let terminate = async
//     {
//         signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap().recv().await;
//     };

//     #[cfg(not(unix))]
//     let terminate = std::future::pending();

//     tokio::select!
//     {
//         _ = ctrl_c => (),
//         _ = terminate => (),
//     }
// }