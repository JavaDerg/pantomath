use crate::error::StreamError;
use crate::noise::handshake::NoiseHandshake;
use crate::noise::NoiseStream;
use sodiumoxide::crypto::box_::SecretKey;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, ToSocketAddrs};

pub struct NoiseListener {
    inner: TcpListener,
    pk: SecretKey,
}

impl NoiseListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A, pk: SecretKey) -> std::io::Result<Self> {
        Ok(Self {
            inner: TcpListener::bind(addr).await?,
            pk,
        })
    }

    pub async fn accept(&self) -> Result<(NoiseStream, SocketAddr), StreamError> {
        let client = self.inner.accept().await?;
        Ok((
            NoiseHandshake::new_priv(client.0, &self.pk)
                .shake_priv(false)
                .await?,
            client.1,
        ))
    }
}
