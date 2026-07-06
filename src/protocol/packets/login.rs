use crate::protocol::error::ProtocolError;
use crate::protocol::packet::ServerPacket;
use crate::protocol::varint::PacketReader;
use crate::protocol::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct SLoginStart {
    pub username: String,
}

impl ServerPacket for SLoginStart {
    fn packet_id(_version: ProtocolVersion) -> i32 {
        0x00
    }

    fn read(buf: &mut PacketReader, _version: ProtocolVersion) -> Result<Self, ProtocolError> {
        let username = buf.read_string()?.to_string();
        Ok(Self { username })
    }
}
