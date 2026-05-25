use crate::{decode_compact_size, encode_compact_size, P2PError};
use std::convert::TryFrom;

#[derive(Debug, Clone, Copy)]
pub enum InvType {
    MsgError = 0,
    MsgTx = 1,
    MsgBlock = 2,
    MsgFilteredBlock = 3,
    MsgCmpctBlock = 4,
}
#[derive(Debug, Clone)]
pub struct InvVector {
    pub inv_type: InvType,
    pub inv_hash: [u8; 32],
}

// The "inv" messages (inventory message) transmits one or more inventories of objects known to the transmitting peer.
#[derive(Debug, Clone)]
pub struct InvMessage {
    pub inventory: Vec<InvVector>,
}

impl TryFrom<u32> for InvType {
    type Error = P2PError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InvType::MsgError),
            1 => Ok(InvType::MsgTx),
            2 => Ok(InvType::MsgBlock),
            3 => Ok(InvType::MsgFilteredBlock),
            4 => Ok(InvType::MsgCmpctBlock),
            _ => Err(P2PError::ParseError("Unknown Type".to_string())),
        }
    }
}

impl InvMessage {
    pub fn deserialize(mut bytes: &[u8]) -> Result<Self, P2PError> {
        let count = decode_compact_size(&mut bytes)?;
        if count > 50_000 {
            return Err(P2PError::OutOfRange);
        }

        if bytes.len() < 36 * count as usize {
            return Err(P2PError::NotEnoughBytesToSplit);
        }

        let mut entries: Vec<InvVector> = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let (item, rest) = bytes.split_at(36);
            bytes = rest;
            let inv_type = u32::from_le_bytes(item[0..4].try_into().map_err(|_| {
                P2PError::ConvertionError(format!("Error while try to convert bytes into u32"))
            })?);
            let inv_hash = &item[4..];
            let inv_vec = InvVector {
                inv_type: InvType::try_from(inv_type)?,
                inv_hash: inv_hash
                    .try_into()
                    .map_err(|e| P2PError::ParseError(format!("{}", e)))?,
            };
            entries.push(inv_vec);
        }
        Ok(Self { inventory: entries })
    }

    pub fn serialize(&self) -> Vec<u8> {
        let count = self.inventory.iter().count();
        let count_bytes = encode_compact_size(count);
        let mut buffer: Vec<u8> = Vec::with_capacity(count_bytes.len() + (count * 36));
        buffer.extend_from_slice(&count_bytes);
        for inv in &self.inventory {
            let inv_type = u32::to_le_bytes(inv.inv_type as u32);
            let inv_hash = inv.inv_hash;
            buffer.extend_from_slice(&inv_type);
            buffer.extend_from_slice(&inv_hash);
        }
        buffer
    }
}
