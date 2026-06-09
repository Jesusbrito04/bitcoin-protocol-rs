use std::sync::{Arc, Mutex};

use crypto_bigint::U256;

use crate::{
    decode_compact_size, encode_compact_size,
    index::store::{HeaderStore, StoredData},
    P2PError, Serialize,
};

use super::transaction::Transaction;
#[derive(Debug)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_block: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u32,
    pub nbits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    pub fn get_target(&self) -> U256 {
        let exponent = self.nbits >> 24;
        let coefficient = U256::from_u32(self.nbits & 0x00ffffff);
        if exponent >= 3 {
            let shift = 8 * (exponent - 3);
            return coefficient << shift;
        } else {
            let shift = 8 * (3 - exponent);
            return coefficient >> shift;
        }
    }

    pub fn get_chainwork(&self) -> U256 {
        let target = self.get_target().wrapping_add(&U256::ONE);
        let max = U256::MAX;
        let (quotient, remainder) = max.div_rem(&target.to_nz().expect("Can't divide by Zero"));
        let remainder = remainder.wrapping_add(&U256::ONE);

        if remainder == target {
            return quotient.wrapping_add(&U256::ONE);
        } else {
            return quotient;
        }
    }
}

#[derive(Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Serialize for BlockHeader {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(80);
        buffer.extend_from_slice(&self.version.to_le_bytes());
        buffer.extend_from_slice(&self.prev_block);
        buffer.extend_from_slice(&self.merkle_root);
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());
        buffer.extend_from_slice(&self.nbits.to_le_bytes());
        buffer.extend_from_slice(&self.nonce.to_le_bytes());
        buffer
    }

    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let (version, rest) = bytes.split_at(4);
        let version = u32::from_le_bytes(version.try_into()?);
        *bytes = rest;

        let (prev_block, rest) = bytes.split_at(32);
        let prev_block = prev_block.try_into()?;
        *bytes = rest;

        let (merkle_root, rest) = bytes.split_at(32);
        let merkle_root = merkle_root.try_into()?;
        *bytes = rest;

        let (timestamp, rest) = bytes.split_at(4);
        let timestamp = u32::from_le_bytes(timestamp.try_into()?);
        *bytes = rest;

        let (nbits, rest) = bytes.split_at(4);
        let nbits = u32::from_le_bytes(nbits.try_into()?);
        *bytes = rest;

        let (nonce, rest) = bytes.split_at(4);
        let nonce = u32::from_le_bytes(nonce.try_into()?);

        *bytes = rest;

        Ok(Self {
            version,
            prev_block,
            merkle_root,
            timestamp,
            nbits,
            nonce,
        })
    }
}

impl Serialize for Block {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let header = self.header.serialize();
        let transactions_len = encode_compact_size(self.transactions.len());
        buffer.extend_from_slice(&header);
        buffer.extend_from_slice(&transactions_len);
        for tx in &self.transactions {
            buffer.extend_from_slice(&tx.serialize());
        }
        buffer
    }

    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let block_header = BlockHeader::deserialize(bytes)?;
        let transactions_len = decode_compact_size(bytes)?;
        let mut transactions: Vec<Transaction> = Vec::new();
        for _ in 0..transactions_len {
            let tx = Transaction::deserialize(bytes)?;
            transactions.push(tx);
        }
        Ok(Self {
            header: block_header,
            transactions,
        })
    }
}

pub struct GetHeadersMessage {
    pub version: u32,
    pub locator: BlockLocator,
    pub hash_stop: [u8; 32],
}

impl Serialize for GetHeadersMessage {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::with_capacity(4 + self.locator.serialize().len() + 32);
        buffer.extend_from_slice(&self.version.to_le_bytes());
        buffer.extend_from_slice(&self.locator.serialize());
        buffer.extend_from_slice(&self.hash_stop);
        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, P2PError> {
        let (version, rest) = bytes.split_at(4);
        let version = u32::from_le_bytes(version.try_into()?);
        *bytes = rest;
        let locator = BlockLocator::deserialize(bytes)?;
        let (hash_stop, rest) = bytes.split_at(32);
        *bytes = rest;
        Ok(Self {
            version,
            locator,
            hash_stop: hash_stop.try_into()?,
        })
    }
}

#[derive(Debug)]
pub struct Headers {
    pub headers: Vec<BlockHeader>,
}
impl Serialize for Headers {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let headers_cmpct = encode_compact_size(self.headers.len());
        buffer.extend_from_slice(&headers_cmpct);
        for header in &self.headers {
            buffer.extend_from_slice(&header.serialize());
            buffer.push(0x00);
        }
        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, P2PError> {
        let mut headers = Vec::new();
        let headers_cmpct = decode_compact_size(bytes)?;
        for _ in 0..headers_cmpct {
            if bytes.len() < 81 {
                return Err(P2PError::NotEnoughBytesToSplit);
            }
            let header = BlockHeader::deserialize(bytes)?;
            headers.push(header);
            *bytes = &bytes[1..];
        }
        Ok(Self { headers })
    }
}

pub struct BlockLocator {
    pub hashes: Vec<[u8; 32]>,
}

impl BlockLocator {
    pub fn new(
        chain_tip: StoredData,
        chain_store: &Arc<Mutex<HeaderStore>>,
    ) -> Result<Self, P2PError> {
        let store = chain_store.lock().unwrap();
        let mut hashes = Vec::new();
        let mut step = 1;
        let mut current_height = chain_tip.height;

        while current_height > 0 {
            let data = store
                .get_header_by_height(current_height)
                .map_err(|_| P2PError::Custom("Error getting desired hash".to_string()))?;
            hashes.push(data.hash);

            if hashes.len() >= 10 {
                step *= 2;
                if step <= current_height {
                    current_height -= step;
                } else {
                    break;
                }
            } else {
                current_height -= step;
            }
        }

        let genesis_hash = store
            .get_header_by_height(0)
            .map_err(|_| P2PError::Custom("Error getting desired hash".to_string()))?;
        hashes.push(genesis_hash.hash);

        Ok(Self { hashes })
    }
}

impl Serialize for BlockLocator {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let hashes_count = self.hashes.len();
        let hashes_cmpct = encode_compact_size(hashes_count as usize);
        let mut buffer = Vec::with_capacity(hashes_count * 32);
        buffer.extend_from_slice(&hashes_cmpct);
        for hash in &self.hashes {
            buffer.extend_from_slice(hash);
        }
        buffer
    }

    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let hashes_count = decode_compact_size(bytes)?;
        let mut hashes: Vec<[u8; 32]> = Vec::with_capacity(hashes_count as usize);
        for _ in 0..hashes_count {
            if bytes.len() < 32 {
                return Err(P2PError::NotEnoughBytesToSplit);
            }

            let (hash, rest) = bytes.split_at(32);
            hashes.push(hash.try_into()?);
            *bytes = rest;
        }
        Ok(Self { hashes })
    }
}
