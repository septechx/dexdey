use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};

use crate::plugin::{Event, EventBus, fire_event};
use crate::protocol::decode::PacketDecoder;
use crate::protocol::frame::{FramedReader, FramedWriter};

fn hex_dump(buf: &[u8]) -> String {
    use std::fmt::Write;
    let n = buf.len().min(32);
    let mut s = String::with_capacity(n * 3 + 10);
    for &b in &buf[..n] {
        write!(&mut s, "{b:02X} ").unwrap();
    }
    if buf.len() > 32 {
        write!(&mut s, "… (+{})", buf.len() - 32).unwrap();
    }
    s
}

pub(crate) struct Proxy {
    event_bus: EventBus,
    port: u16,
}

impl Proxy {
    pub(crate) fn new(event_bus: EventBus, port: u16) -> Self {
        Self { event_bus, port }
    }

    pub(crate) async fn start(&self) -> Result<()> {
        fire_event(|env| -> Result<_> {
            Ok(self.event_bus.fire_event(env, Event::proxy_initialize()?))
        })?;

        trace!("Starting proxy on port {}", self.port);
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
        info!("Started proxy on port {}", self.port);

        loop {
            let (socket, addr) = listener.accept().await?;
            tokio::spawn(Self::handle_client(socket, addr));
        }
    }

    async fn handle_client(socket: TcpStream, addr: SocketAddr) {
        info!("Accepted connection from {}", addr);

        let upstream = match TcpStream::connect("127.0.0.1:25566").await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to connect to upstream: {e}");
                return;
            }
        };

        let (client_read, client_write) = socket.into_split();
        let (upstream_read, upstream_write) = upstream.into_split();

        let decoder = Arc::new(Mutex::new(PacketDecoder::new()));

        let dec_cs = decoder.clone();
        let cs_task = tokio::spawn(async move {
            let mut reader = FramedReader::new(client_read);
            let mut writer = FramedWriter::new(upstream_write);

            loop {
                let frame = match reader.read_frame().await {
                    Ok(f) => f,
                    Err(e) => {
                        debug!("CS connection closed: {e}");
                        break;
                    }
                };

                let mut dec = dec_cs.lock().await;
                match dec.decode_serverbound(&frame) {
                    Ok(pkt) => {
                        debug!("CS [{}] {:?} ({}B)", dec.state(), pkt, frame.len());
                    }
                    Err(e) => {
                        warn!("CS decode error: {e}");
                    }
                }
                drop(dec);

                trace!("CS → upstream ({}B) {}", frame.len(), hex_dump(&frame));
                if let Err(e) = writer.write_frame(&frame).await {
                    warn!("CS write error: {e}");
                    break;
                }
            }
        });

        let sc_task = tokio::spawn(async move {
            let mut reader = FramedReader::new(upstream_read);
            let mut writer = FramedWriter::new(client_write);

            loop {
                let frame = match reader.read_frame().await {
                    Ok(f) => f,
                    Err(e) => {
                        debug!("SC connection closed: {e}");
                        break;
                    }
                };

                trace!("SC ← upstream ({}B) {}", frame.len(), hex_dump(&frame));
                if let Err(e) = writer.write_frame(&frame).await {
                    warn!("SC write error: {e}");
                    break;
                }
            }
        });

        tokio::select! {
            _ = cs_task => trace!("CS task finished"),
            _ = sc_task => trace!("SC task finished"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProxyError {
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
    #[error("TCP error: {0}")]
    Io(#[from] tokio::io::Error),
}

type Result<T> = std::result::Result<T, ProxyError>;
