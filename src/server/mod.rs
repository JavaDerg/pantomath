use pantomath::noise::NoiseStream;
use pantomath::proto::init::{ClientHello, ServerHello};
use std::error::Error;

pub async fn handle_client_handshake(
    stream: &mut NoiseStream,
    hello: ClientHello,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}
