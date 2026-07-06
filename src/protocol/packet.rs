use crate::protocol::error::ProtocolError;
use crate::protocol::varint::{PacketReader, PacketWriter};
use crate::protocol::version::ProtocolVersion;

pub trait ServerPacket: Sized {
    fn read(buf: &mut PacketReader, version: ProtocolVersion) -> Result<Self, ProtocolError>;

    fn packet_id(version: ProtocolVersion) -> i32;
}

pub trait ClientPacket: Sized {
    fn read(buf: &mut PacketReader, version: ProtocolVersion) -> Result<Self, ProtocolError>;

    fn write(&self, buf: &mut PacketWriter, version: ProtocolVersion);

    fn packet_id(version: ProtocolVersion) -> i32;

    fn encode(&self, version: ProtocolVersion) -> Result<Vec<u8>, ProtocolError> {
        let mut writer = PacketWriter::new();
        writer.write_varint(Self::packet_id(version));
        self.write(&mut writer, version);
        Ok(writer.finish())
    }
}
