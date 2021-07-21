use crate::error::StreamError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::noise::framed::{Frame16TcpStream, MAX_PAYLOAD_LEN};
use crate::noise::NsRequest;
use bytes::Buf;
use bytes::Bytes;
use futures::task::ArcWake;
use futures::FutureExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

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
        in ($cx:expr, $arc:expr $(,)?);
        $(let $n:pat = $v:expr;)*
        $(~ let $n_o:pat = $v_o:expr;)?
    ) => {
        $(
            let ____________waker = futures::task::waker(Arc::new(IdWaker(
                $v,
                $arc.clone(),
                $cx.waker().clone(),
            )));
            let $n = Context::from_waker(&____________waker);
        )*
        $(
            let ____________waker = futures::task::waker(Arc::new(IdWaker(
                $v_o,
                $arc,
                $cx.waker().clone(),
            )));
            let $n_o = Context::from_waker(&____________waker);
        )?
    }
}

pub(super) struct SafeRecvSendUpdt<'a> {
    pub update_recv: flume::r#async::RecvFut<'a, NsRequest>,
    pub sendrecv: &'a mut Frame16TcpStream,
    pub send: Option<Bytes>,
    pub state: Arc<AtomicUsize>,
}

pub(super) enum RsuResponse {
    Update(NsRequest),
    Wrote,
    Read(Bytes),
}

impl<'a> SafeRecvSendUpdt<'a> {}

struct IdWaker(usize, Arc<AtomicUsize>, Waker);

pub const NONE: usize = 0;
pub const UPDATE: usize = 1;
pub const SEND: usize = 2;
pub const RECV: usize = 3;

impl<'a> Future for SafeRecvSendUpdt<'a> {
    type Output = Result<RsuResponse, StreamError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            update_recv,
            sendrecv,
            send,
            state,
        } = &mut *self;

        match state.swap(NONE, Ordering::AcqRel) {
            NONE => {
                context! {
                    in (cx, state.clone());
                    let mut update_context = UPDATE;
                    let mut send_context = SEND;
                    ~ let mut recv_context = RECV;
                }

                ready!(~ update_recv.poll_unpin(&mut update_context), |r: Result<NsRequest, flume::RecvError>| Ok(RsuResponse::Update(r.expect("Channel can not be closed"))));
                if let Some(bytes) = send {
                    let sendrecv = Pin::new(&mut **sendrecv);
                    ready!(~ sendrecv.poll_write(&mut send_context, bytes.chunk()), |r: std::io::Result<usize>| r.map(|_| RsuResponse::Wrote).map_err(|err| err.into()));
                }

                let mut buf = (0..MAX_PAYLOAD_LEN).map(|_| 0u8).collect::<Vec<_>>();
                let mut read_buf = ReadBuf::new(buf.as_mut_slice());

                let sendrecv = Pin::new(&mut **sendrecv);
                let res = sendrecv.poll_read(&mut recv_context, &mut read_buf);
                let len = read_buf.filled().len();
                let _ = read_buf;
                buf.truncate(len);
                ready!(~ res, |r: std::io::Result<()>| r.map(|_| RsuResponse::Read(Bytes::from(buf))).map_err(|err| err.into()));
            }
            UPDATE => {
                context! {
                    in (cx, state.clone());
                    ~ let mut context = UPDATE;
                }
                return update_recv
                    .poll_unpin(&mut context)
                    .map(|r| Ok(RsuResponse::Update(r.expect("Channel can not be closed"))));
            }
            SEND => {
                context! {
                    in (cx, state.clone());
                    ~ let mut context = SEND;
                }
                return Pin::new(&mut **sendrecv)
                    .poll_write(&mut context, send.as_ref().unwrap().chunk())
                    .map_ok(|_| RsuResponse::Wrote)
                    .map_err(|err| err.into());
            }
            RECV => {
                context! {
                    in (cx, state.clone());
                    ~ let mut context = RECV;
                }
                return update_recv
                    .poll_unpin(&mut context)
                    .map(|r| Ok(RsuResponse::Update(r.expect("Channel can not be closed"))));
            }
            x => unreachable!("Invalid state id supplied? {}", x),
        }
        Poll::Pending
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
