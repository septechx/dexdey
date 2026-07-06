// Very WIP, allow stuff that will be used in the future
#![allow(dead_code)]

pub mod decode;
pub mod error;
pub mod forwarding;
pub mod frame;
pub mod packet;
pub mod packets;
pub mod varint;
pub mod version;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Config,
    Play,
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Handshake => write!(f, "HANDSHAKE"),
            Self::Status => write!(f, "STATUS"),
            Self::Login => write!(f, "LOGIN"),
            Self::Config => write!(f, "CONFIG"),
            Self::Play => write!(f, "PLAY"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Serverbound,
    Clientbound,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serverbound => write!(f, "SERVERBOUND"),
            Self::Clientbound => write!(f, "CLIENTBOUND"),
        }
    }
}
