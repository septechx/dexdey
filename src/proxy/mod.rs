use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};

use crate::config::Config;
use crate::plugin::{Event, EventBus, fire_event};
use crate::protocol::decode::PacketDecoder;
use crate::protocol::forwarding;
use crate::protocol::frame::{FramedReader, FramedWriter};
use crate::protocol::varint::decode_varint;

fn hex_dump(buf: &[u8]) -> String {
    use std::fmt::Write;
    let n = buf.len().min(32);
    let mut s = String::with_capacity(n * 3 + 10);
    for &b in &buf[..n] {
        write!(&mut s, "{b:02X} ").unwrap();
    }
    if buf.len() > 32 {
        write!(&mut s, "\u{2026} (+{})", buf.len() - 32).unwrap();
    }
    s
}

struct PlayerInfo {
    ip: String,
    username: Option<String>,
}

impl PlayerInfo {
    fn new(ip: String) -> Self {
        Self { ip, username: None }
    }
}

pub(crate) struct Proxy {
    event_bus: EventBus,
    config: Arc<Config>,
    port: u16,
}

impl Proxy {
    pub(crate) fn new(event_bus: EventBus, config: Config, port: u16) -> Self {
        Self {
            event_bus,
            config: Arc::new(config),
            port,
        }
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
            let config = self.config.clone();
            tokio::spawn(Self::handle_client(socket, addr, config));
        }
    }

    async fn handle_client(socket: TcpStream, addr: SocketAddr, config: Arc<Config>) {
        info!("Accepted connection from {}", addr);

        let upstream = match TcpStream::connect("127.0.0.1:25566").await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to connect to upstream: {e}");
                return;
            }
        };

        let addr_str = addr.ip().to_string();
        let (client_read, client_write) = socket.into_split();
        let (upstream_read, upstream_write) = upstream.into_split();

        let player_info = Arc::new(Mutex::new(PlayerInfo::new(addr_str)));
        let up_writer = Arc::new(Mutex::new(FramedWriter::new(upstream_write)));

        let decoder = Arc::new(Mutex::new(PacketDecoder::new()));

        let cs_player = player_info.clone();
        let cs_writer = up_writer.clone();
        let cs_decoder = decoder.clone();
        let cs_task = tokio::spawn(async move {
            let mut reader = FramedReader::new(client_read);
            loop {
                let frame = match reader.read_frame().await {
                    Ok(f) => f,
                    Err(e) => {
                        debug!("CS connection closed: {e}");
                        break;
                    }
                };

                let mut dec = cs_decoder.lock().await;
                match dec.decode_serverbound(&frame) {
                    Ok(pkt) => {
                        debug!("CS [{}] {:?} ({}B)", dec.state(), pkt, frame.len());
                        if let crate::protocol::decode::DecodedServerPacket::LoginStart(ls) = &pkt {
                            let mut info = cs_player.lock().await;
                            info.username = Some(ls.username.clone());
                        }
                    }
                    Err(e) => {
                        warn!("CS decode error: {e}");
                    }
                }
                drop(dec);

                trace!(
                    "CS \u{2192} upstream ({}B) {}",
                    frame.len(),
                    hex_dump(&frame)
                );
                let mut w = cs_writer.lock().await;
                if let Err(e) = w.write_frame(&frame).await {
                    warn!("CS write error: {e}");
                    break;
                }
            }
        });

        let sc_player = player_info.clone();
        let sc_writer = up_writer.clone();
        let sc_config = config.clone();
        let http_client = reqwest::Client::new();
        let sc_task = tokio::spawn(async move {
            let mut reader = FramedReader::new(upstream_read);
            let mut client_writer = FramedWriter::new(client_write);
            let mut compression: Option<i32> = None;
            loop {
                let frame = match reader.read_frame().await {
                    Ok(f) => f,
                    Err(e) => {
                        debug!("SC connection closed: {e}");
                        break;
                    }
                };

                // Detect LoginCompressionS2CPacket (0x03 CB) before compression is enabled
                if compression.is_none() {
                    let mut buf: &[u8] = &frame;
                    if let Ok(pid) = decode_varint(&mut buf)
                        && pid == 0x03
                        && let Ok(threshold) = decode_varint(&mut buf)
                    {
                        compression = Some(threshold);
                        debug!("Detected compression threshold={}", threshold);
                    }
                }

                if let Some((msg_id, requested_version)) =
                    forwarding::try_parse_velocity_plugin_msg(&frame, compression)
                {
                    let (username, ip) = {
                        let info = sc_player.lock().await;
                        (info.username.clone(), info.ip.clone())
                    };
                    if let Some(username) = username {
                        let uuid = match forwarding::fetch_uuid(&http_client, &username).await {
                            Ok(u) => {
                                debug!("Fetched online UUID for {}: {}", username, u);
                                u
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to fetch online UUID for {}, falling back to offline: \
                                     {e}",
                                    username
                                );
                                forwarding::offline_player_uuid(&username)
                            }
                        };
                        let data = forwarding::create_forwarding_data(
                            &sc_config.forwarding_secret,
                            &ip,
                            &uuid,
                            &username,
                            &[],
                            requested_version,
                        );
                        let response =
                            forwarding::encode_login_plugin_response(msg_id, &data, compression);
                        debug!(
                            "Responded to Velocity forwarding request for {} (msg_id={})",
                            username, msg_id
                        );
                        let mut w = sc_writer.lock().await;
                        if let Err(e) = w.write_frame(&response).await {
                            warn!("SC response write error: {e}");
                            break;
                        }
                    } else {
                        warn!(
                            "No player info available for Velocity forwarding (msg_id={})",
                            msg_id
                        );
                        if let Err(e) = client_writer.write_frame(&frame).await {
                            warn!("SC write error: {e}");
                            break;
                        }
                    }
                } else {
                    trace!(
                        "SC \u{2190} upstream ({}B) {}",
                        frame.len(),
                        hex_dump(&frame)
                    );
                    if let Err(e) = client_writer.write_frame(&frame).await {
                        warn!("SC write error: {e}");
                        break;
                    }
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
