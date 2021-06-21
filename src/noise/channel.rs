use bytes::Bytes;
use flume::{Receiver, Sender};
use crate::error::ChannelError;

pub struct NoiseChannel {
    sender: Sender<Bytes>,
    receiver: Receiver<Bytes>,
}

pub enum Control {
    Message(Bytes),
    Failure(ChannelError, FailureResolution),
    Eof,
}

pub enum FailureResolution {
    Ignore,
    CloseChannel,
    CloseConnection,
}

struct InternalNoiseChannel {
    sender: Sender<Bytes>,
    receiver: Receiver<Bytes>,
}

impl NoiseChannel {
    pub fn new(sender: Sender<Bytes>, receiver: Receiver<Bytes>) -> Self {
        Self {
            receiver,
            sender
        }
    }

    pub fn new_pair() -> (Self, Self) {
        let (p1s, p1r) = flume::unbounded();
        let (p2s, p2r) = flume::unbounded();
        (Self::new(p1s, p2r), Self::new(p2s, p1r))
    }
}
