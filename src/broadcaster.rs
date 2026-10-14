use std::ops::ControlFlow;

use tokio::{io, select};
use tokio::net::UdpSocket;
use tokio::time::Interval;

pub struct Broadcaster<F>
{
    pub socket: UdpSocket,
    pub message: Vec<u8>,
    pub interval: Interval,
    pub on_error: F,
}

impl<F, Fut, B> Broadcaster<F>
where
    F: FnMut(io::Error) -> Fut,
    Fut: Future<Output = ControlFlow<B>>,
{
    pub async fn run(mut self) -> Option<B>
    {
        loop
        {
            select!
            {
                biased;

                _ = crate::shutdown_signal() => return None,
                _ = self.interval.tick() => (),
            }

            let mut offset = 0;

            while offset < self.message.len()
            {
                match self.socket.send(&self.message[offset..]).await
                {
                    Ok(x) => offset += x,
                    Err(x) => if let ControlFlow::Break(x) = (self.on_error)(x).await { return Some(x) },
                }
            }
        }
    }
}