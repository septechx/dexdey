use bytes::BufMut;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::protocol::error::ProtocolError;

const MAX_VARINT_SIZE: usize = 5;

pub fn decode_varint(buf: &mut &[u8]) -> Result<i32, ProtocolError> {
    let mut val = 0i32;
    for i in 0..MAX_VARINT_SIZE {
        if buf.is_empty() {
            return Err(ProtocolError::VarIntTooLarge);
        }
        let byte = buf[0];
        *buf = &buf[1..];
        val |= ((byte & 0x7F) as i32) << (i * 7);
        if byte & 0x80 == 0 {
            return Ok(val);
        }
    }
    Err(ProtocolError::VarIntTooLarge)
}

pub fn encode_varint(buf: &mut impl BufMut, mut value: i32) {
    loop {
        if value & !0x7F == 0 {
            buf.put_u8(value as u8);
            return;
        }
        buf.put_u8((value as u8 & 0x7F) | 0x80);
        value = ((value as u32) >> 7) as i32;
    }
}

pub fn varint_size(value: i32) -> usize {
    let mut size = 1;
    let mut v = (value as u32) >> 7;
    while v != 0 {
        size += 1;
        v >>= 7;
    }
    size
}

pub async fn read_varint_async<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<i32, ProtocolError> {
    let mut val = 0i32;
    for i in 0..MAX_VARINT_SIZE {
        let byte = reader.read_u8().await?;
        val |= ((byte & 0x7F) as i32) << (i * 7);
        if byte & 0x80 == 0 {
            return Ok(val);
        }
    }
    Err(ProtocolError::VarIntTooLarge)
}

pub struct PacketReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PacketReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn ensure_consumed(&self) -> Result<(), ProtocolError> {
        if self.remaining() != 0 {
            return Err(ProtocolError::UnconsumedBytes {
                remaining: self.remaining(),
            });
        }
        Ok(())
    }

    pub fn read_varint(&mut self) -> Result<i32, ProtocolError> {
        let mut val = 0i32;
        for i in 0..MAX_VARINT_SIZE {
            if self.pos >= self.buf.len() {
                return Err(ProtocolError::VarIntTooLarge);
            }
            let byte = self.buf[self.pos];
            self.pos += 1;
            val |= ((byte & 0x7F) as i32) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(val);
            }
        }
        Err(ProtocolError::VarIntTooLarge)
    }

    pub fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        if self.pos >= self.buf.len() {
            return Err(ProtocolError::Decode("buffer underflow"));
        }
        let val = self.buf[self.pos];
        self.pos += 1;
        Ok(val)
    }

    pub fn read_i16_be(&mut self) -> Result<i16, ProtocolError> {
        if self.pos + 2 > self.buf.len() {
            return Err(ProtocolError::Decode("buffer underflow"));
        }
        let val = i16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    pub fn read_u16_be(&mut self) -> Result<u16, ProtocolError> {
        if self.pos + 2 > self.buf.len() {
            return Err(ProtocolError::Decode("buffer underflow"));
        }
        let val = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    pub fn read_i32_be(&mut self) -> Result<i32, ProtocolError> {
        if self.pos + 4 > self.buf.len() {
            return Err(ProtocolError::Decode("buffer underflow"));
        }
        let val = i32::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(val)
    }

    pub fn read_i64_be(&mut self) -> Result<i64, ProtocolError> {
        if self.pos + 8 > self.buf.len() {
            return Err(ProtocolError::Decode("buffer underflow"));
        }
        let val = i64::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
            self.buf[self.pos + 4],
            self.buf[self.pos + 5],
            self.buf[self.pos + 6],
            self.buf[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(val)
    }

    pub fn read_bool(&mut self) -> Result<bool, ProtocolError> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_string(&mut self) -> Result<&'a str, ProtocolError> {
        let len = self.read_varint()? as usize;
        if len > 32767 {
            return Err(ProtocolError::StringTooLong {
                len,
                max: 32767,
            });
        }
        if self.pos + len > self.buf.len() {
            return Err(ProtocolError::Decode("string exceeds buffer"));
        }
        let s = std::str::from_utf8(&self.buf[self.pos..self.pos + len])
            .map_err(|_| ProtocolError::Decode("invalid UTF-8"))?;
        self.pos += len;
        Ok(s)
    }

    pub fn read_remaining(&mut self) -> &'a [u8] {
        let remaining = &self.buf[self.pos..];
        self.pos = self.buf.len();
        remaining
    }
}

pub struct PacketWriter {
    buf: Vec<u8>,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn write_varint(&mut self, value: i32) {
        encode_varint(&mut self.buf, value);
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf.put_u8(value);
    }

    pub fn write_i16_be(&mut self, value: i16) {
        self.buf.put_i16(value);
    }

    pub fn write_u16_be(&mut self, value: u16) {
        self.buf.put_u16(value);
    }

    pub fn write_i32_be(&mut self, value: i32) {
        self.buf.put_i32(value);
    }

    pub fn write_i64_be(&mut self, value: i64) {
        self.buf.put_i64(value);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.buf.put_u8(if value { 1 } else { 0 });
    }

    pub fn write_string(&mut self, value: &str) {
        self.write_varint(value.len() as i32);
        self.buf.extend_from_slice(value.as_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }
}

impl Default for PacketWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip() {
        let cases = vec![0, 1, 127, 128, 255, 256, 32767, 65535, 2097151, 2147483647];
        for &val in &cases {
            let mut buf = Vec::new();
            encode_varint(&mut buf, val);
            let mut slice: &[u8] = &buf;
            let decoded = decode_varint(&mut slice).unwrap();
            assert_eq!(val, decoded, "roundtrip failed for {val}");
            assert!(slice.is_empty(), "not all bytes consumed for {val}");
        }
    }

    #[test]
    fn test_varint_sizes() {
        assert_eq!(varint_size(0), 1);
        assert_eq!(varint_size(127), 1);
        assert_eq!(varint_size(128), 2);
        assert_eq!(varint_size(16383), 2);
        assert_eq!(varint_size(16384), 3);
    }

    #[test]
    fn test_packet_reader_writer() {
        let mut w = PacketWriter::new();
        w.write_varint(42);
        w.write_string("hello");
        w.write_u16_be(25565);
        w.write_i64_be(12345);

        let bytes = w.finish();
        let mut r = PacketReader::new(&bytes);
        assert_eq!(r.read_varint().unwrap(), 42);
        assert_eq!(r.read_string().unwrap(), "hello");
        assert_eq!(r.read_u16_be().unwrap(), 25565);
        assert_eq!(r.read_i64_be().unwrap(), 12345);
        assert_eq!(r.remaining(), 0);
    }

    #[tokio::test]
    async fn test_varint_async_roundtrip() {
        use tokio::io::AsyncWriteExt;

        let cases = vec![0, 1, 127, 128, 255, 256, 65535, 2097151, 2147483647];
        for &val in &cases {
            let (mut a, mut b) = tokio::io::duplex(1024);

            let mut encoded = Vec::new();
            encode_varint(&mut encoded, val);
            a.write_all(&encoded).await.unwrap();
            drop(a);

            let decoded = read_varint_async(&mut b).await.unwrap();
            assert_eq!(val, decoded, "async roundtrip failed for {val}");
        }
    }

    #[tokio::test]
    async fn test_frame_roundtrip() {
        use crate::protocol::frame::{FramedReader, FramedWriter};

        let (mut a, b) = tokio::io::duplex(4096);
        let (mut c, d) = tokio::io::duplex(4096);

        let handle = tokio::spawn(async move {
            tokio::io::copy_bidirectional(&mut a, &mut c).await.unwrap();
        });

        let mut writer = FramedWriter::new(b);
        let mut reader = FramedReader::new(d);

        let payload = b"\x00\xFF\x05localhost\x63\xDD\x01";
        writer.write_frame(payload).await.unwrap();

        let read_back = reader.read_frame().await.unwrap();
        assert_eq!(&read_back[..], &payload[..]);
        drop(handle);
    }
}
