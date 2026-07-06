use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::protocol::error::ProtocolError;
use crate::protocol::varint::{encode_varint, read_varint_async};

const MAX_PACKET_SIZE: usize = 2_097_152;

pub struct FramedReader<R: AsyncRead + Unpin> {
    reader: R,
}

impl<R: AsyncRead + Unpin> FramedReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    pub async fn read_frame(&mut self) -> Result<BytesMut, ProtocolError> {
        let length = read_varint_async(&mut self.reader).await? as usize;

        if length > MAX_PACKET_SIZE {
            return Err(ProtocolError::PacketTooLarge {
                size: length,
                max: MAX_PACKET_SIZE,
            });
        }

        if length == 0 {
            return Err(ProtocolError::Decode("zero-length packet"));
        }

        let mut buf = BytesMut::with_capacity(length);
        buf.resize(length, 0);
        self.reader.read_exact(&mut buf).await?;

        Ok(buf)
    }
}

pub struct FramedWriter<W: AsyncWrite + Unpin> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> FramedWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub async fn write_frame(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        let mut buf = Vec::new();
        let length = data.len() as i32;
        encode_varint(&mut buf, length);
        buf.extend_from_slice(data);
        self.writer.write_all(&buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), ProtocolError> {
        self.writer.flush().await?;
        Ok(())
    }
}
