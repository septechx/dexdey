use crate::protocol::ConnectionState;
use crate::protocol::error::ProtocolError;
use crate::protocol::packet::ServerPacket;
use crate::protocol::varint::PacketReader;
use crate::protocol::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct SHandshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: ConnectionState,
}

impl ServerPacket for SHandshake {
    fn packet_id(_version: ProtocolVersion) -> i32 {
        0x00
    }

    fn read(buf: &mut PacketReader, _version: ProtocolVersion) -> Result<Self, ProtocolError> {
        let protocol_version = buf.read_varint()?;
        let server_address = buf.read_string()?.to_string();
        let server_port = buf.read_u16_be()?;
        let next_state = match buf.read_varint()? {
            1 => ConnectionState::Status,
            2 => ConnectionState::Login,
            _ => {
                return Err(ProtocolError::Decode("invalid next state"));
            }
        };

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}
