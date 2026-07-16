use crate::decoder::Decoder;
use crate::error::DecodeError;
use crate::{set_packet_id, version::PacketId};
use minecraft_protocol_derive::{Decoder, Encoder};
use std::io::Read;

set_packet_id!(Handshake, 0x00);

pub enum HandshakeServerBoundPacket {
    Handshake(Handshake),
}

impl HandshakeServerBoundPacket {
    pub fn get_type_id(&self) -> u8 {
        match self {
            HandshakeServerBoundPacket::Handshake(_) => 0x00,
        }
    }

    pub fn decode<R: Read>(type_id: u8, reader: &mut R) -> Result<Self, DecodeError> {
        match type_id {
            0x00 => {
                let handshake = Handshake::decode(reader)?;
                Ok(HandshakeServerBoundPacket::Handshake(handshake))
            }
            _ => Err(DecodeError::UnknownPacketType { type_id }),
        }
    }
}

#[derive(Clone, Encoder, Decoder, Debug)]
pub struct Handshake {
    #[data_type(with = "var_int")]
    pub protocol_version: i32,
    #[data_type(max_length = 32767)]
    pub server_addr: String,
    pub server_port: u16,
    #[data_type(with = "var_int")]
    pub next_state: i32,
}

impl Handshake {
    pub fn new(
        protocol_version: i32,
        server_addr: String,
        server_port: u16,
        next_state: i32,
    ) -> HandshakeServerBoundPacket {
        let handshake = Handshake {
            protocol_version,
            server_addr,
            server_port,
            next_state,
        };

        HandshakeServerBoundPacket::Handshake(handshake)
    }
}

#[cfg(test)]
mod server_addr_length_tests {
    use super::*;
    use crate::encoder::Encoder;
    use std::io::Cursor;

    #[test]
    fn decodes_handshake_with_forwarded_server_addr_over_255_chars() {
        // Velocity/BungeeCord modern forwarding appends a null-separated
        // UUID and signed property payload to server_addr, which routinely
        // exceeds the legacy 255-character Minecraft handshake limit.
        let long_addr = format!("play.example.com\0{}", "x".repeat(400));
        assert!(long_addr.len() > 255);

        let handshake = Handshake {
            protocol_version: 765,
            server_addr: long_addr.clone(),
            server_port: 25565,
            next_state: 2,
        };

        let mut buf = Vec::new();
        handshake.encode(&mut buf).unwrap();

        let decoded = Handshake::decode(&mut Cursor::new(buf)).unwrap();
        assert_eq!(decoded.server_addr, long_addr);
    }
}
