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
    g
}
