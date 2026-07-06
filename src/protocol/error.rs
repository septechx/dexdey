use thiserror::Error;

use crate::protocol::ConnectionState;
use crate::protocol::Direction;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] tokio::io::Error),
    #[error("VarInt is too large")]
    VarIntTooLarge,
    #[error("Invalid packet ID 0x{id:02X} in state {state:?} direction {direction:?}")]
    UnknownPacketId {
        id: i32,
        state: ConnectionState,
        direction: Direction,
    },
    #[error("Packet too large: {size} bytes (max {max})")]
    PacketTooLarge { size: usize, max: usize },
    #[error("Unconsumed bytes in packet: {remaining} bytes remaining")]
    UnconsumedBytes { remaining: usize },
    #[error("String too long: {len} bytes (max {max})")]
    StringTooLong { len: usize, max: usize },
    #[error("Decode error: {0}")]
    Decode(&'static str),
}
