use bytes::BytesMut;

use crate::protocol::ConnectionState;
use crate::protocol::error::ProtocolError;
use crate::protocol::packet::ServerPacket;
use crate::protocol::packets::handshake::SHandshake;
use crate::protocol::packets::login::SLoginStart;
use crate::protocol::packets::status::{SPing, SRequest};
use crate::protocol::varint::{PacketReader, decode_varint, encode_varint};
use crate::protocol::version::ProtocolVersion;

#[derive(Debug)]
pub enum DecodedServerPacket {
    Handshake(SHandshake),
    StatusRequest(SRequest),
    StatusPing(SPing),
    LoginStart(SLoginStart),
    Unknown { id: i32, data: Vec<u8> },
}

impl DecodedServerPacket {
    pub fn packet_id(&self) -> i32 {
        match self {
            Self::Handshake(_) => 0x00,
            Self::StatusRequest(_) => 0x00,
            Self::StatusPing(_) => 0x01,
            Self::LoginStart(_) => 0x00,
            Self::Unknown { id, .. } => *id,
        }
    }

    pub fn encode(&self, _version: ProtocolVersion) -> Vec<u8> {
        match self {
            Self::Unknown { id, data } => {
                let mut out = Vec::new();
                encode_varint(&mut out, *id);
                out.extend_from_slice(data);
                out
            }
            _ => {
                let packet_id = self.packet_id();
                let mut out = Vec::new();
                encode_varint(&mut out, packet_id);
                out
            }
        }
    }
}

pub struct PacketDecoder {
    state: ConnectionState,
    version: ProtocolVersion,
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Handshake,
            version: ProtocolVersion::V1_21,
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn version(&self) -> ProtocolVersion {
        self.version
    }

    pub fn decode_serverbound(
        &mut self,
        frame: &BytesMut,
    ) -> Result<DecodedServerPacket, ProtocolError> {
        let mut buf: &[u8] = frame.as_ref();
        let packet_id = decode_varint(&mut buf)?;
        let mut reader = PacketReader::new(buf);

        match self.state {
            ConnectionState::Handshake => match packet_id {
                0x00 => {
                    let pkt = SHandshake::read(&mut reader, self.version)?;
                    reader.ensure_consumed()?;
                    self.version = ProtocolVersion::from_protocol(pkt.protocol_version)
                        .unwrap_or(self.version);
                    self.state = pkt.next_state;
                    Ok(DecodedServerPacket::Handshake(pkt))
                }
                _ => Err(ProtocolError::UnknownPacketId {
                    id: packet_id,
                    state: self.state,
                    direction: crate::protocol::Direction::Serverbound,
                }),
            },
            ConnectionState::Status => match packet_id {
                0x00 => {
                    let pkt = SRequest::read(&mut reader, self.version)?;
                    reader.ensure_consumed()?;
                    Ok(DecodedServerPacket::StatusRequest(pkt))
                }
                0x01 => {
                    let pkt = SPing::read(&mut reader, self.version)?;
                    reader.ensure_consumed()?;
                    Ok(DecodedServerPacket::StatusPing(pkt))
                }
                _ => Err(ProtocolError::UnknownPacketId {
                    id: packet_id,
                    state: self.state,
                    direction: crate::protocol::Direction::Serverbound,
                }),
            },
            ConnectionState::Login => match packet_id {
                0x00 => {
                    let pkt = SLoginStart::read(&mut reader, self.version)?;
                    Ok(DecodedServerPacket::LoginStart(pkt))
                }
                _ => Ok(DecodedServerPacket::Unknown {
                    id: packet_id,
                    data: reader.read_remaining().to_vec(),
                }),
            },
            _ => Ok(DecodedServerPacket::Unknown {
                id: packet_id,
                data: reader.read_remaining().to_vec(),
            }),
        }
    }

    pub fn decode_clientbound(&mut self, _frame: &BytesMut) -> Result<(), ProtocolError> {
        Ok(())
    }
}

impl Default for PacketDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::varint::encode_varint;
    use bytes::BytesMut;

    fn encode_frame(packet_id: i32, payload: &[u8]) -> BytesMut {
        let mut data = Vec::new();
        encode_varint(&mut data, packet_id);
        data.extend_from_slice(payload);
        BytesMut::from(&data[..])
    }

    #[test]
    fn test_decode_handshake_status() {
        let mut decoder = PacketDecoder::new();
        assert_eq!(decoder.state(), ConnectionState::Handshake);

        // Build handshake: protocol=767 (1.21), "localhost", port 25565, next_state=1 (status)
        let mut payload = Vec::new();
        encode_varint(&mut payload, 767); // protocol version
        encode_varint(&mut payload, 9); // "localhost".len()
        payload.extend_from_slice(b"localhost");
        payload.extend_from_slice(&[0x63, 0xDD]); // 25565 as u16 big-endian
        encode_varint(&mut payload, 1); // next state = status

        let frame = encode_frame(0x00, &payload);
        let pkt = decoder.decode_serverbound(&frame).unwrap();
        assert!(matches!(pkt, DecodedServerPacket::Handshake(_)));
        assert_eq!(decoder.state(), ConnectionState::Status);
        assert_eq!(decoder.version(), ProtocolVersion::V1_21);

        // Now decode status request
        let frame = encode_frame(0x00, &[]);
        let pkt = decoder.decode_serverbound(&frame).unwrap();
        assert!(matches!(pkt, DecodedServerPacket::StatusRequest(_)));

        // Decode ping
        let mut payload = Vec::new();
        payload.extend_from_slice(&42u64.to_be_bytes());
        let frame = encode_frame(0x01, &payload);
        let pkt = decoder.decode_serverbound(&frame).unwrap();
        assert!(matches!(pkt, DecodedServerPacket::StatusPing(_)));
    }

    #[test]
    fn test_decode_handshake_login_unknown() {
        let mut decoder = PacketDecoder::new();

        // Handshake with next_state=2 (login)
        let mut payload = Vec::new();
        encode_varint(&mut payload, 767);
        encode_varint(&mut payload, 9);
        payload.extend_from_slice(b"localhost");
        payload.extend_from_slice(&[0x63, 0xDD]);
        encode_varint(&mut payload, 2); // next state = login

        let frame = encode_frame(0x00, &payload);
        let pkt = decoder.decode_serverbound(&frame).unwrap();
        assert!(matches!(pkt, DecodedServerPacket::Handshake(_)));
        assert_eq!(decoder.state(), ConnectionState::Login);

        // In Login state, unknown packets should be returned as Unknown
        let frame = encode_frame(0x99, b"hello");
        let pkt = decoder.decode_serverbound(&frame).unwrap();
        assert!(matches!(pkt, DecodedServerPacket::Unknown { id: 0x99, .. }));
    }
}
