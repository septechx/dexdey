use crate::protocol::error::ProtocolError;
use crate::protocol::packet::{ClientPacket, ServerPacket};
use crate::protocol::varint::{PacketReader, PacketWriter};
use crate::protocol::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct SRequest;

impl ServerPacket for SRequest {
    fn packet_id(_version: ProtocolVersion) -> i32 {
        0x00
    }

    fn read(_buf: &mut PacketReader, _version: ProtocolVersion) -> Result<Self, ProtocolError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone)]
pub struct CResponse {
    pub json: String,
}

impl ClientPacket for CResponse {
    fn packet_id(_version: ProtocolVersion) -> i32 {
        0x00
    }

    fn read(buf: &mut PacketReader, _version: ProtocolVersion) -> Result<Self, ProtocolError> {
        let json = buf.read_string()?.to_string();
        Ok(Self { json })
    }

    fn write(&self, buf: &mut PacketWriter, _version: ProtocolVersion) {
        buf.write_string(&self.json);
    }
}

#[derive(Debug, Clone)]
pub struct SPing {
    pub time: i64,
}

impl ServerPacket for SPing {
    fn packet_id(_version: ProtocolVersion) -> i32 {
        0x01
    }

    fn read(buf: &mut PacketReader, _version: ProtocolVersion) -> Result<Self, ProtocolError> {
        let time = buf.read_i64_be()?;
        Ok(Self { time })
    }
}

#[derive(Debug, Clone)]
pub struct CPong {
    pub time: i64,
}

impl ClientPacket for CPong {
    fn packet_id(_version: ProtocolVersion) -> i32 {
        0x01
    }

    fn read(buf: &mut PacketReader, _version: ProtocolVersion) -> Result<Self, ProtocolError> {
        let time = buf.read_i64_be()?;
        Ok(Self { time })
    }

    fn write(&self, buf: &mut PacketWriter, _version: ProtocolVersion) {
        buf.write_i64_be(self.time);
    }
}
