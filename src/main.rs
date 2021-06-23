use pantomath::noise::channel::ChannelId;
use pantomath::noise::{NoiseListener, NoiseStream};
use pantomath::proto::init::hello::Kind as HelloKind;
use sodiumoxide::crypto::box_::SecretKey;
use std::error::Error;
use std::time::Instant;
use tracing::Instrument;

mod config;
mod server;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = &*Box::leak(Box::new(config::init().await));

    let listener = NoiseListener::bind("127.0.0.1:1337", SecretKey(config.keypair.private.clone()))
        .await
        .unwrap();

    tokio::spawn(async {
        let stream = NoiseStream::connect("127.0.0.1:1337", None)
            .await
            .unwrap()
            .shake()
            .await
            .unwrap();
    });

    loop {
        let (mut stream, ip_addr) = match listener.accept().await {
            Ok(client) => client,
            Err(err) => {
                tracing::error!("Error accepting client; err={}", err);
                continue;
            }
        };
        tokio::spawn(
            async move {
                let result: Result<(), Box<dyn Error>> = async move {
                    let handshake = stream
                        .recv::<pantomath::proto::init::Hello>(ChannelId(0))
                        .await?;
                    match handshake
                        .unwrap()
                        .kind
                        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidData))?
                    {
                        HelloKind::Client(client) => (),
                        HelloKind::Server(server) => (),
                    }
                    Ok(())
                }
                .await;
                if let Err(err) = result {
                    // TODO: Rephrase this
                    tracing::error!("Error working with client; err={}", err);
                }
            }
            .instrument(tracing::info_span!(
                "client",
                ip_addr = format!("{:?}", ip_addr).as_str()
            )),
        );
    }
}
