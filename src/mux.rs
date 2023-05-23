use std::{sync::Arc, time::Duration};

use bytes::{BufMut, Bytes, BytesMut};
use dashmap::DashMap;
use parking_lot::Mutex;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        oneshot, Barrier,
    },
    task::JoinHandle,
    io::{AsyncReadExt, AsyncWriteExt}, time::timeout,
};

use crate::{
    errors::Error,
    messages::{ControlMessage, Header},
    muxstream::MuxStream,
    state::StreamIdGenerator,
    unwrap_err, unwrap_err_continue, unwrap_err_return, unwrap_opt, unwrap_opt_continue,
    unwrap_opt_return,
};

#[derive(Debug, Clone)]
pub struct Config {
    connect_timeout: u64,
    accept_timeout: u64,
    max_buf_size: usize,
}

impl Config {
    pub fn new(connect_timeout: u64, accept_timeout: u64, max_buf_size: usize) -> Self {
        Config {
            connect_timeout,
            accept_timeout,
            max_buf_size,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connect_timeout: 1,
            accept_timeout: 1,
            max_buf_size: 8096,
        }
    }
}

pub struct Mux {
    config: Config,

    established_streams: Arc<DashMap<u32, UnboundedSender<Bytes>>>,
    waiting_streams: Arc<DashMap<u32, oneshot::Sender<u32>>>,
    stream_id_generator: Arc<Mutex<StreamIdGenerator>>,
    accept_barrier: Arc<Barrier>,

    outbound_sender: Arc<UnboundedSender<Bytes>>,

    join_handle_inbound: JoinHandle<Result<(), Error>>,
    join_handle_outbound: JoinHandle<Result<(), Error>>,
}

impl Mux {
    pub fn new(tcp_stream: TcpStream, config: Config) -> Mux {
        let (mut read_half, mut write_half) = tcp_stream.into_split();

        let established_streams: Arc<DashMap<u32, UnboundedSender<Bytes>>> =
            Arc::new(DashMap::new());
        let waiting_streams: Arc<DashMap<u32, oneshot::Sender<u32>>> = Arc::new(DashMap::new());
        let accept_barrier = Arc::new(Barrier::new(2));

        let stream_id_generator = Arc::new(Mutex::new(StreamIdGenerator::new()));

        let (outbound_sender, mut outbound_receiver) = unbounded_channel::<Bytes>();
        let outbound_sender = Arc::new(outbound_sender);

        let waiting_streams_ = waiting_streams.clone();
        let established_streams_ = established_streams.clone();
        let stream_id_generator_ = stream_id_generator.clone();
        let accept_barrier_ = accept_barrier.clone();
        let join_handle_inbound: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            loop {
                let header = Header(unwrap_err_continue!(
                    read_half.read_u64().await,
                    Error::ReadError,
                    "failed to read header from tcp stream"
                ));
                let stream_id = header.stream_id();

                match header.control() {
                    ControlMessage::Data => {
                        let data_size = header.data_size();
                        let mut buf: BytesMut = BytesMut::zeroed(data_size as usize);

                        let stream = unwrap_opt_continue!(
                            established_streams_.get(&stream_id),
                            Error::StreamDoesNotExistError,
                            "peer tried to send data to nonexistent stream"
                        );

                        let stream = stream.value();

                        unwrap_err_continue!(
                            read_half.read_exact(&mut buf).await,
                            Error::ReadError,
                            "failed to read data from tcp stream"
                        );

                        unwrap_err_continue!(
                            stream.send(buf.freeze()),
                            Error::WriteError,
                            "failed to send data to stream"
                        );
                    }

                    ControlMessage::Open => {
                        let waiting_streams__ = waiting_streams_.clone();
                        let accept_barrier__ = accept_barrier_.clone();

                        tokio::spawn(async move {
                            unwrap_err!(
                                timeout(
                                    Duration::from_secs(config.accept_timeout),
                                    accept_barrier__.wait()
                                )
                                .await,
                                Error::WaitError,
                                "timeout"
                            );
                            let stream_wait = unwrap_opt!(
                                waiting_streams__.remove(&0),
                                Error::NotAcceptingError,
                                "not currently accepting connections"
                            );
                            unwrap_err!(
                                stream_wait.1.send(stream_id),
                                Error::WriteError,
                                "failed to notify stream of open"
                            );
                        });
                    }

                    ControlMessage::OpenAck => {
                        let stream_wait = unwrap_opt_continue!(
                            waiting_streams_.remove(&stream_id),
                            Error::NotAcceptingError,
                            "peer tried to acknowledge nonexistent stream"
                        );
                        unwrap_err_continue!(
                            stream_wait.1.send(stream_id),
                            Error::WriteError,
                            "failed to notify stream of open"
                        );
                    }

                    ControlMessage::Close => {
                        unwrap_opt_continue!(
                            established_streams_.remove(&stream_id),
                            Error::StreamDoesNotExistError,
                            "peer tried to close nonexistent stream"
                        );
                        stream_id_generator_.lock().release_id(stream_id);
                    }
                }
            }
        });

        let join_handle_outbound: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            loop {
                let bytes = unwrap_opt_return!(
                    outbound_receiver.recv().await,
                    Error::ReadError,
                    "failed to read from internal bytes buffer: channel closed"
                );

                unwrap_err_continue!(
                    write_half.write_all(&bytes).await,
                    Error::WriteError,
                    "failed to write to tcp stream"
                );
            }
        });

        Self {
            config,

            established_streams,
            waiting_streams,
            stream_id_generator,
            accept_barrier,

            outbound_sender,

            join_handle_inbound,
            join_handle_outbound,
        }
    }

    pub async fn connect(&self) -> Result<MuxStream, Error> {
        let stream_id = self.stream_id_generator.lock().next();

        let (wait_sender, wait_receiver) = oneshot::channel::<u32>();
        let (inbound_sender, inbound_receiver) = unbounded_channel::<Bytes>();

        self.waiting_streams.insert(stream_id, wait_sender);
        self.established_streams.insert(stream_id, inbound_sender);

        unwrap_err_return!(
            {
                let mut buf = BytesMut::with_capacity(8);
                buf.put_u64(Header::new(ControlMessage::Open, stream_id, 0).0);

                self.outbound_sender.send(buf.freeze())
            },
            Error::WriteError,
            "failed to queue open message",
            {
                self.waiting_streams.remove(&stream_id);
                self.stream_id_generator.lock().release_id(stream_id);
                self.established_streams.remove(&stream_id);
            }
        );

        unwrap_err_return!(
            unwrap_err_return!(
                timeout(
                    Duration::from_secs(self.config.connect_timeout as u64),
                    wait_receiver
                )
                .await,
                Error::WaitError,
                "timeout",
                {
                    self.waiting_streams.remove(&stream_id);
                    self.stream_id_generator.lock().release_id(stream_id);
                    self.established_streams.remove(&stream_id);
                }
            ),
            Error::WaitError,
            "failed to wait for open acknowledgement",
            {
                self.waiting_streams.remove(&stream_id);
                self.stream_id_generator.lock().release_id(stream_id);
                self.established_streams.remove(&stream_id);
            }
        );

        Ok(MuxStream {
            stream_id,

            buffer_size: self.config.max_buf_size,

            inbound_receiver,
            outbound_sender: self.outbound_sender.clone(),
        })
    }

    pub async fn accept(&self) -> Result<MuxStream, Error> {
        let stream_id = 0;

        let (wait_sender, wait_receiver) = oneshot::channel::<u32>();
        let (inbound_sender, inbound_receiver) = unbounded_channel::<Bytes>();

        self.waiting_streams.insert(stream_id, wait_sender);
        self.accept_barrier.wait().await;

        let stream_id = unwrap_err_return!(
            wait_receiver.await,
            Error::WaitError,
            "failed to wait for open",
            {
                self.waiting_streams.remove(&stream_id);
            }
        );
        self.stream_id_generator.lock().acquire_id(stream_id);
        self.established_streams.insert(stream_id, inbound_sender);

        unwrap_err_return!(
            {
                let mut buf = BytesMut::with_capacity(8);
                buf.put_u64(Header::new(ControlMessage::OpenAck, stream_id, 0).0);

                self.outbound_sender.send(buf.freeze())
            },
            Error::WriteError,
            "failed to queue open message",
            {
                self.waiting_streams.remove(&stream_id);
                self.stream_id_generator.lock().release_id(stream_id);
                self.established_streams.remove(&stream_id);
            }
        );

        Ok(MuxStream {
            stream_id,

            buffer_size: self.config.max_buf_size,

            inbound_receiver,
            outbound_sender: self.outbound_sender.clone(),
        })
    }
}

impl Drop for Mux {
    fn drop(&mut self) {
        for stream in self.established_streams.iter() {
            let mut buf = BytesMut::with_capacity(8);
            buf.put_u64(Header::new(ControlMessage::Close, *stream.key(), 0).0);

            self.outbound_sender.send(buf.freeze());
        }
        self.join_handle_inbound.abort();
        self.join_handle_outbound.abort();
    }
}
