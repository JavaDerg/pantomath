use crate::error::StreamError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::noise::framed::Frame16TcpStream;
use crate::noise::NsRequest;
use futures::task::ArcWake;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

macro_rules! ready {
    (~ $e:expr, $handle:expr $(,)?) => {
        match $e {
            std::task::Poll::Ready(t) => return std::task::Poll::Ready($handle(t)),
            std::task::Poll::Pending => (),
        }
    };
    (~ $e:expr $(,)?) => {
        ready!(~ $e, |x| x)
    };
    // Taken from tokio
    ($e:expr $(,)?) => {
        match $e {
            std::task::Poll::Ready(t) => t,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }
    };
}

macro_rules! context {
    (
        in (
            $cx:expr,
            $arc:expr,
            $cx:expr $(,)?
        );
        $(let $n:ident = $v:expr;)*
    ) => {
        $(
            let ____________waker = futures::task::waker(Arc::new(IdWaker(
                $v,
                arc,
                $cx.waker().clone(),
            )));
            let $n = Context::from_waker(&____________waker);
        )*
    }
}

pub struct SafeRecvSendUpdt<'a> {
    update_recv: &'a mut flume::r#async::RecvFut<'a, NsRequest>,
    sendrecv: &'a mut Frame16TcpStream,
    send: bool,
    state: Arc<AtomicUsize>,
}

pub enum RsuResponse {
    Update(NsRequest),
}

impl<'a> SafeRecvSendUpdt<'a> {}

struct IdWaker(usize, Arc<AtomicUsize>, Waker);

const NONE: usize = 0;
const UPDATE: usize = 1;

impl<'a> Future for SafeRecvSendUpdt<'a> {
    type Output = Result<RsuResponse, StreamError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            update_recv,
            sendrecv,
            send,
            state,
        } = &mut *self;

        let update = Pin::new(&mut **update_recv);
        context! {
            in cx;
            let update_context = UPDATE;
        }

        match state.load(Ordering::Acquire) {
            NONE => {
                ready!(~ update.poll(update_context), |r: Result<NsRequest, flume::RecvError>| Ok(RsuResponse::Update(r.expect("Channel can not be closed"))));
            }
            _ => (),
        }
        todo!()
    }
}

impl ArcWake for IdWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if arc_self
            .1
            .compare_exchange(0, arc_self.0, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            arc_self.2.wake_by_ref();
        }
    }
}
