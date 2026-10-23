use std::net::SocketAddr;

pub fn message(name: Option<&str>, addr: SocketAddr) -> String {
    match name {
        Some(name) => format!("[Hellberry {name}:{addr}]"),
        None => format!("[Hellberry:{addr}]"),
    }
}