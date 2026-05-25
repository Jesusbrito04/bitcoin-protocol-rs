use crate::{decode_compact_size, encode_compact_size, P2PError};
use sha2::{Digest, Sha256};

// Magic bytes indicating the originating network. They are by default little-endian bytes.
pub const MAINNET: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
pub const TESTNET: [u8; 4] = [0x0b, 0x11, 0x09, 0x07];
pub const REGTEST: [u8; 4] = [0xfa, 0xbf, 0xb5, 0xda];

// Command message, ASCII string which identifies what message type is contained in the payload.
pub const VERSION: [u8; 12] = [
    0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E, 0x00, 0x00, 0x00, 0x00, 0x00,
];
pub const VERACK: [u8; 12] = [
    0x76, 0x65, 0x72, 0x61, 0x63, 0x6B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
pub const GETADDR: [u8; 12] = [
    0x67, 0x65, 0x74, 0x61, 0x64, 0x64, 0x72, 0x00, 0x00, 0x00, 0x00, 0x00,
];
pub const ADDR: [u8; 12] = [
    0x61, 0x64, 0x64, 0x72, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
pub const PING: [u8; 12] = [
    0x70, 0x69, 0x6E, 0x67, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
pub const PONG: [u8; 12] = [
    0x70, 0x6F, 0x6E, 0x67, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
pub const INV: [u8; 12] = [
    0x69, 0x6E, 0x76, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Debug)]
pub struct MsgHeader {
    pub magic: [u8; 4],
    pub command: [u8; 12],
    pub payload_size: u32,
    pub checksum: [u8; 4],
}

impl MsgHeader {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::with_capacity(24);
        buffer.extend_from_slice(&self.magic);
        buffer.extend_from_slice(&self.command);
        buffer.extend_from_slice(&self.payload_size.to_le_bytes());
        buffer.extend_from_slice(&self.checksum);
        buffer
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, P2PError> {
        // The message header has a length of 24 bytes
        if bytes.len() < 24 {
            return Err(P2PError::NotEnoughBytesToSplit);
        }
        let mut magic: [u8; 4] = [0u8; 4];
        let mut command: [u8; 12] = [0u8; 12];
        let payload_size: u32;
        let mut checksum: [u8; 4] = [0u8; 4];

        magic.copy_from_slice(&bytes[0..4]);
        command.copy_from_slice(&bytes[4..16]);
        payload_size = u32::from_le_bytes(bytes[16..20].try_into().map_err(|e| {
            P2PError::ConvertionError(format!("Error decoding payload size: {}", e))
        })?);
        checksum.copy_from_slice(&bytes[20..24]);

        Ok(Self {
            magic,
            command,
            payload_size,
            checksum,
        })
    }

    pub fn calculate_checksum(payload: &[u8]) -> [u8; 4] {
        let hash1 = Sha256::digest(payload);
        let hash2 = Sha256::digest(hash1);
        let mut checksum: [u8; 4] = [0u8; 4];
        checksum.copy_from_slice(&hash2[..4]);
        checksum
    }
}

pub struct Ping {
    pub nonce: u64,
}

pub struct Pong {
    pub nonce: u64,
}

impl Ping {
    pub fn serialize(&self) -> Vec<u8> {
        let mut nonce = Vec::with_capacity(8);
        nonce.extend_from_slice(&self.nonce.to_le_bytes());
        nonce
    }
    pub fn deserialize(bytes: &[u8]) -> Result<Self, P2PError> {
        let nonce = u64::from_le_bytes(bytes[..8].try_into().map_err(|_| {
            P2PError::ConvertionError(format!("Error while try to convert bytes into u64"))
        })?);
        Ok(Self { nonce: nonce })
    }
}

impl Pong {
    pub fn serialize(&self) -> Vec<u8> {
        let mut nonce = Vec::with_capacity(8);
        nonce.extend_from_slice(&self.nonce.to_le_bytes());
        nonce
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, P2PError> {
        let nonce = u64::from_le_bytes(bytes[..8].try_into().map_err(|_| {
            P2PError::ConvertionError(format!("Error while try to convert bytes into u64"))
        })?);
        Ok(Self { nonce: nonce })
    }
}

#[derive(Debug)]
pub struct Addr {
    pub ip_addresses: Vec<IpAddress>,
}

impl Addr {
    pub fn serialize(&self) -> Vec<u8> {
        let ip_count = encode_compact_size(self.ip_addresses.len());
        let mut buffer: Vec<u8> =
            Vec::with_capacity(ip_count.len() + (self.ip_addresses.len() * 30));
        buffer.extend_from_slice(&ip_count);
        for address in &self.ip_addresses {
            buffer.extend_from_slice(&address.serialize());
        }
        buffer
    }

    pub fn deserialize(mut bytes: &[u8]) -> Result<Self, P2PError> {
        let ip_count = decode_compact_size(&mut bytes)?;
        let mut ip_addresses: Vec<IpAddress> = Vec::with_capacity(ip_count as usize);
        for _ in 0..ip_count {
            let address = IpAddress::deserialize(&mut bytes)?;
            ip_addresses.push(address);
        }
        Ok(Self { ip_addresses })
    }
}

#[derive(Debug)]
pub struct IpAddress {
    pub time: u32,
    pub service: u64,
    pub ip: [u8; 16],
    pub port: u16,
}

impl IpAddress {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::with_capacity(30);
        buffer.extend_from_slice(&self.time.to_le_bytes());
        buffer.extend_from_slice(&self.service.to_le_bytes());
        buffer.extend_from_slice(&self.ip);
        buffer.extend_from_slice(&self.port.to_be_bytes());
        buffer
    }

    pub fn deserialize(bytes: &mut &[u8]) -> Result<Self, P2PError> {
        let (time, rest) = bytes.split_at(4);
        let time = u32::from_le_bytes(time.try_into().map_err(|_| {
            P2PError::ConvertionError(format!("Error while try to convert bytes into u32"))
        })?);
        let (service, rest) = rest.split_at(8);
        let service = u64::from_le_bytes(service.try_into().map_err(|_| {
            P2PError::ConvertionError(format!("Error while try to convert bytes into u64"))
        })?);
        let (ip, rest) = rest.split_at(16);
        let ip = ip
            .try_into()
            .map_err(|e| P2PError::ConvertionError(format!("Error decoding ip address: {}", e)))?;
        let (port, rest) = rest.split_at(2);
        let port = u16::from_be_bytes(port.try_into().map_err(|_| {
            P2PError::ConvertionError(format!("Error while try to convert bytes into u16"))
        })?);

        *bytes = rest;

        Ok(Self {
            time,
            service,
            ip,
            port,
        })
    }
}
