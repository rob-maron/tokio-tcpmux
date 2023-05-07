use std::{sync::Arc, task::Poll};

use bytes::{Bytes, BytesMut, BufMut};
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender}, io::{AsyncRead, AsyncWrite}};

use crate::messages::{Header, ControlMessage};



pub struct MuxStream {
    pub stream_id: u32,

    pub buffer_size: usize,

    pub inbound_receiver: UnboundedReceiver<Bytes>,
    pub outbound_sender: Arc<UnboundedSender<Bytes>>,
}

impl Drop for MuxStream {
    fn drop(&mut self) {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64(Header::new(ControlMessage::Close, self.stream_id, 0).0);

        self.outbound_sender.send(buf.freeze()).unwrap();
    }
}

impl AsyncRead for MuxStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        // read one message
        match self.inbound_receiver.poll_recv(cx) {
            Poll::Ready(bytes) => match bytes {
                Some(some) => {
                    buf.put(some);
                    Poll::Ready(Ok(()))
                }
                None => Poll::Ready(Ok(())),
            },

            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for MuxStream {
    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut total_length = 0;
        for buf in bufs {
            let length = buf.len();
            total_length += length;
            for i in 0..(length / self.buffer_size) {
                let start = i * self.buffer_size;
                let end = (i + 1) * self.buffer_size;

                let mut bytes = BytesMut::with_capacity(8 + self.buffer_size);
                bytes.put_u64(
                    Header::new(ControlMessage::Data, self.stream_id, (end - start) as u32).0,
                );
                bytes.put(&buf[start..end]);

                if let Err(err) = self.outbound_sender.send(bytes.freeze()) {
                    return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)));
                }
            }

            let start = length - length % self.buffer_size;
            let end = length;
            let mut bytes = BytesMut::with_capacity(8 + length % self.buffer_size);
            bytes
                .put_u64(Header::new(ControlMessage::Data, self.stream_id, (end - start) as u32).0);
            bytes.put(&buf[start..end]);

            // send bytes
            if let Err(err) = self.outbound_sender.send(bytes.freeze()) {
                return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)));
            }
        }

        Poll::Ready(Ok(total_length))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64(Header::new(ControlMessage::Close, self.stream_id, 0).0);

        self.outbound_sender.send(buf.freeze()).unwrap();

        Poll::Ready(Ok(()))
    }

    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let length = buf.len();
        for i in 0..(length / self.buffer_size) {
            let start = i * self.buffer_size;
            let end = (i + 1) * self.buffer_size;

            let mut bytes = BytesMut::with_capacity(8 + self.buffer_size);
            bytes
                .put_u64(Header::new(ControlMessage::Data, self.stream_id, (end - start) as u32).0);
            bytes.put(&buf[start..end]);

            if let Err(err) = self.outbound_sender.send(bytes.freeze()) {
                return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)));
            }
        }

        let start = length - length % self.buffer_size;
        let end = length;

        if start != end {
            let mut bytes = BytesMut::with_capacity(8 + length % self.buffer_size);
            bytes
                .put_u64(Header::new(ControlMessage::Data, self.stream_id, (end - start) as u32).0);
            bytes.put(&buf[start..end]);

            // send bytes
            if let Err(err) = self.outbound_sender.send(bytes.freeze()) {
                return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)));
            }
        }

        Poll::Ready(Ok(length))
    }
}
