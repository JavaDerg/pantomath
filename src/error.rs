use std::error::Error;
use err_derive::Error;

use std::io::Error as IoError;
use snow::Error as NoiseError;
use prost::{EncodeError, DecodeError};

#[derive(Debug, Error)]
pub enum StreamError {
    #[error(display = "IO error occurred")]
    IoError(#[error(source)] IoError),
    #[error(display = "failed to encrypt/decrypt data")]
    NoiseError(#[error(source)] NoiseError),
    #[error(display = "failed to decode packet")]
    DecodeError(#[error(source)] DecodeError),
    #[error(display = "failed to encode packet")]
    EncodeError(#[error(source)] EncodeError),
}

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error(display = "failed to decode packet")]
    DecodeError(#[error(source)] DecodeError),
    #[error(display = "failed to encode packet")]
    EncodeError(#[error(source)] EncodeError),
    #[error(display = "some error occurred")]
    Dyn(#[error(source)] Box<dyn Error>),
}