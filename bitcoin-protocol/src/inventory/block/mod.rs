use crate::{decode_compact_size, encode_compact_size, Serialize};

use super::transaction::Transaction;
#[derive(Debug)]
pub struct BlockHeader {
    version: u32,
    p_block: [u8; 32],
    merkle_root: [u8; 32],
    timestamp: u32,
    target: u32,
    nonce: u32,
}

#[derive(Debug)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}

impl Serialize for BlockHeader {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(80);
        buffer.extend_from_slice(&self.version.to_le_bytes());
        buffer.extend_from_slice(&self.p_block);
        buffer.extend_from_slice(&self.merkle_root);
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());
        buffer.extend_from_slice(&self.target.to_le_bytes());
        buffer.extend_from_slice(&self.nonce.to_le_bytes());
        buffer
    }

    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let (version, rest) = bytes.split_at(4);
        let version = u32::from_le_bytes(version.try_into()?);
        *bytes = rest;

        let (p_block, rest) = bytes.split_at(32);
        let p_block = p_block.try_into()?;
        *bytes = rest;

        let (merkle_root, rest) = bytes.split_at(32);
        let merkle_root = merkle_root.try_into()?;
        *bytes = rest;

        let (timestamp, rest) = bytes.split_at(4);
        let timestamp = u32::from_le_bytes(timestamp.try_into()?);
        *bytes = rest;

        let (target, rest) = bytes.split_at(4);
        let target = u32::from_le_bytes(target.try_into()?);
        *bytes = rest;

        let (nonce, rest) = bytes.split_at(4);
        let nonce = u32::from_le_bytes(nonce.try_into()?);

        *bytes = rest;

        Ok(Self {
            version,
            p_block,
            merkle_root,
            timestamp,
            target,
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
