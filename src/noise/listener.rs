use crate::config::Config;
use crate::noise::handshake::NoiseHandshake;
use crate::noise::NoiseStream;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, ToSocketAddrs};

pub struct NoiseListener {
    config: &'static Config,
    listener: TcpListener,
}

impl NoiseListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A, config: &'static Config) -> std::io::Result<Self> {
        Ok(Self {
            config,
            listener: TcpListener::bind(addr).await?,
        })
    }

    // FIXME: Use better Error type
    pub async fn accept(&self) -> Result<(NoiseStream, SocketAddr), Box<dyn Error>> {
        let client = self.listener.accept().await?;
        Ok((
            NoiseHandshake::new_priv(client.0, self.config)
                .shake_priv(false)
                .await?,
            client.1,
        ))
    }
}
