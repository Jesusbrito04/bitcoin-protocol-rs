use crate::{
    decode_compact_size, encode_compact_size,
    index::chain::BlockChain,
    inventory::InvType::{MsgBlock, MsgTx, MsgWitnessBlock, MsgWitnessTx},
    P2PError, Serialize,
};
use std::{
    convert::TryFrom,
    sync::{Arc, Mutex},
};
pub mod block;
pub mod transaction;

#[derive(Debug, Clone, Copy)]
pub enum InvType {
    MsgError = 0,
    MsgTx = 1,
    MsgBlock = 2,
    MsgFilteredBlock = 3,
    MsgCmpctBlock = 4,
    MsgWitnessTx = 0x40000001,
    MsgWitnessBlock = 0x40000002,
    MsgFilteredWitnessBlock = 0x40000003,
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

impl InvMessage {
    pub fn is_new_header_available(
        &self,
        chain_store: &Arc<Mutex<BlockChain>>,
    ) -> Result<bool, P2PError> {
        let chain_store_locked = chain_store
            .lock()
            .map_err(|e| P2PError::Custom(format!("Cant get the locked value: {e}")))?;

        for inv in &self.inventory {
            match inv.inv_type {
                InvType::MsgBlock => {
                    if !chain_store_locked.contains_header(&inv.inv_hash)? {
                        return Ok(true);
                    }
                    continue;
                }
                InvType::MsgWitnessBlock => {
                    if !chain_store_locked.contains_header(&inv.inv_hash)? {
                        return Ok(true);
                    }
                    continue;
                }
                _ => continue,
            }
        }

        return Ok(false);
    }
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
            0x40000001 => Ok(InvType::MsgWitnessTx),
            0x40000002 => Ok(InvType::MsgWitnessBlock),
            0x40000003 => Ok(InvType::MsgFilteredWitnessBlock),
            _ => Err(P2PError::Parse("Unknown Type".to_string())),
        }
    }
}

impl Serialize for InvMessage {
    type Value = Self;
    fn deserialize(bytes: &mut &[u8]) -> Result<Self, P2PError> {
        let count = decode_compact_size(bytes)?;
        if count > 50_000 {
            return Err(P2PError::OutOfRange);
        }

        if bytes.len() < 36 * count as usize {
            return Err(P2PError::NotEnoughBytesToSplit);
        }

        let mut entries: Vec<InvVector> = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let (item, rest) = bytes.split_at(36);
            *bytes = rest;

            let type_num = u32::from_le_bytes(item[0..4].try_into()?);
            let inv_type = match type_num.try_into()? {
                MsgTx => MsgWitnessTx,
                MsgBlock => MsgWitnessBlock,
                inv_type => inv_type,
            };

            let inv_hash = &item[4..];
            let inv_vec = InvVector {
                inv_type,
                inv_hash: inv_hash.try_into()?,
            };
            entries.push(inv_vec);
        }
        Ok(Self { inventory: entries })
    }

    fn serialize(&self) -> Vec<u8> {
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
