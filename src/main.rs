use crate::noise::{NoiseListener, NoiseStream};
use std::time::Instant;
use tracing::Instrument;

mod config;
mod noise;
mod proto;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = &*Box::leak(Box::new(config::init().await));

    let listener = NoiseListener::bind("127.0.0.1:1337", config).await.unwrap();
    let client = tokio::spawn(async move {
        NoiseStream::connect("127.0.0.1:1337", config)
            .await
            .unwrap()
            .shake()
            .await
            .unwrap()
    });
    let (mut server, _) = listener.accept().await.unwrap();
    let mut client = client.await.unwrap();

    tokio::spawn(async move {
        let packet = proto::message::Message {
            msg: (0..1 << std::env::args().nth(1).unwrap().parse::<usize>().unwrap())
                .map(|_| 'a')
                .collect::<String>(),
        };
        loop {
            client
                .send(&packet)
                .instrument(tracing::info_span!("send pack", id = 1))
                .await
                .unwrap();
        }
    });

    let mut start = Instant::now();

    let mut total = 0usize;
    let mut total_pk = 0usize;
    for i in 1usize.. {
        let packet = server
            .recv::<proto::message::Message>()
            .instrument(tracing::info_span!("recv pack", id = i))
            .await
            .unwrap_or_else(|err| panic!("Failed at packet {}\n{:?}", i, err));
        total += packet.msg.len();
        total_pk += 1;
        let passed = Instant::now() - start;
        if passed.as_secs_f64() > 1.0 {
            tracing::info!(
                "#{:06} {:X}; avg {:4.03}MB/s {:5.1}pk/s",
                i,
                packet.msg.len(),
                total as f64 / passed.as_secs_f64() / 1_000_000.0,
                total_pk as f64 / passed.as_secs_f64()
            );

            start = Instant::now();
            total = 0;
            total_pk = 0;
        }
    }
}
