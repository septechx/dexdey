use std::net::SocketAddr;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, trace, warn};

use crate::plugin::{Event, EventBus, fire_event};

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

        let (mut ri, mut wi) = socket.into_split();
        let (mut ru, mut wu) = upstream.into_split();

        let client_to_upstream = io::copy(&mut ri, &mut wu);
        let upstream_to_client = io::copy(&mut ru, &mut wi);

        tokio::select! {
            r = client_to_upstream => {
                if let Err(e) = r {
                    warn!("client->upstream error: {e}");
                }
            }
            r = upstream_to_client => {
                if let Err(e) = r {
                    warn!("upstream->client error: {e}");
                }
            }
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
