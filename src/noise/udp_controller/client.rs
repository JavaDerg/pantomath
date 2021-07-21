use atomic::Atomic;
use std::ops::Deref;
use std::sync::atomic::Ordering;

pub struct Client {}

pub struct SharedSendState(Atomic<SendState>);

#[derive(Copy, Clone)]
pub enum SendState {
    Queued,
    Send,
    Acknowledged,
}

impl Deref for SharedSendState {
    type Target = SendState;

    fn deref(&self) -> &Self::Target {
        &self.0.load(Ordering::Acquire)
    }
}
