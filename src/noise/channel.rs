use crate::error::ChannelError;
use bytes::Bytes;
use flume::{Receiver, Sender};

pub struct NoiseChannel {
    sender: Sender<Control>,
    receiver: Receiver<Bytes>,
    id: ChannelId,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct ChannelId(pub u8);

pub(crate) struct IntNoiseChannel {
    pub sender: Sender<Bytes>,
    pub receiver: Receiver<Control>,
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
    pub(super) fn new_pair(id: ChannelId) -> (Self, IntNoiseChannel) {
        let (p1s, p1r) = flume::unbounded();
        let (p2s, p2r) = flume::unbounded();
        (
            Self {
                sender: p1s,
                receiver: p2r,
                id,
            },
            IntNoiseChannel {
                sender: p2s,
                receiver: p1r,
            },
        )
    }
}
