use sha2::{Digest, Sha256};
use crate::P2PError;

pub const MAINNET: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
pub const TESTNET: [u8; 4] = [0x0b, 0x11, 0x09, 0x07];
pub const REGTEST: [u8; 4] = [0xfa, 0xbf, 0xb5, 0xda];
pub const VERSION: [u8; 12] = [0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const VERACK: [u8; 12] = [0x76, 0x65, 0x72, 0x61, 0x63, 0x6B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

#[derive(Debug)]
pub struct MsgHeader {
    pub magic: [u8; 4],
    pub command: [u8; 12],
    pub payload_size: u32,
    pub checksum: [u8; 4]
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

    pub fn deserialize(bytes: &[u8]) -> Result<Self, P2PError>{
        if bytes.len() < 24 {
            return Err(P2PError::NotEnoughBytesToSplit);
        }
        let mut magic: [u8; 4] = [0u8; 4];
        let mut command: [u8; 12] = [0u8; 12];
        let payload_size: u32;
        let mut checksum: [u8; 4] = [0u8; 4];
        
        magic.copy_from_slice(&bytes[0..4]);
        command.copy_from_slice(&bytes[4..16]);
        payload_size = u32::from_le_bytes(bytes[16..20].try_into().map_err(|e| P2PError::ConvertionError(format!("Error decoding payload size: {}", e)))?);
        checksum.copy_from_slice(&bytes[20..24]);
        
        Ok(Self {
            magic,
            command,
            payload_size,
            checksum
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
