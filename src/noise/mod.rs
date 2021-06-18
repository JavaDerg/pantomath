pub mod framed;
mod handshake;
pub mod listener;

use crate::config::Config;
use crate::noise::framed::Frame16TcpStream;
use crate::noise::handshake::NoiseHandshake;
pub use listener::NoiseListener;
use snow::TransportState;
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct NoiseStream {
    stream: Frame16TcpStream,
    noise: TransportState,
    config: &'static Config,
}

impl NoiseStream {
    pub async fn connect<A: ToSocketAddrs>(
        addr: A,
        config: &'static Config,
    ) -> std::io::Result<NoiseHandshake> {
        Ok(NoiseHandshake::new(TcpStream::connect(addr).await?, config))
    }
}
