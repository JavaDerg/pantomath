use crate::noise::{NoiseListener, NoiseStream};
use bytes::BufMut;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod config;
mod noise;
mod proto;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = &*Box::leak(Box::new(config::init().await));

    // let listener = noise::listener::NoiseListener;

    let listener = NoiseListener::bind("127.0.0.1:1337", config).await.unwrap();
    let client = tokio::spawn(async move {
        NoiseStream::connect("127.0.0.1:1337", config)
            .await
            .unwrap()
            .shake()
            .await
            .unwrap()
    });
    let server = listener.accept().await.unwrap();
    let client = client.await.unwrap();

    // client.write(b"Hello world!").await.unwrap();
    // client.write(b"Hello world!").await.unwrap();
    // client.write(b"Hello world!").await.unwrap();
    //
    // for _ in 0..3 {
    //     let mut buf = [0u8; 256];
    //     let r = server.read(&mut buf[..]).await.unwrap();
    //     println!(
    //         "client says: {} {:?}",
    //         r,
    //         String::from_utf8_lossy(&buf[..r])
    //     );
    // }
    //
    // server.write(b"Hello world!").await.unwrap();
    // server.write(b"Hello world!").await.unwrap();
    // server.write(b"Hello world!").await.unwrap();
    //
    // for _ in 0..3 {
    //     let mut buf = [0u8; 256];
    //     let r = client.read(&mut buf[..]).await.unwrap();
    //     println!(
    //         "server says: {} {:?}",
    //         r,
    //         String::from_utf8_lossy(&buf[..r])
    //     );
    // }
}
