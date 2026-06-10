use crypto_bigint::U256;
use hex::FromHexError;
use sled::{open, Db, Error as dbError};

use crate::{inventory::block::BlockHeader, P2PError, Serialize};

#[derive(Debug)]
pub enum Error {
    Database(dbError),
    HashNotFound,
    InvalidBlock(P2PError),
    FromHex(FromHexError),
    Parse(String),
}
#[derive(Debug)]
pub struct HeaderStore {
    pub db: Db,
}

#[derive(Debug)]
pub struct StoredData {
    pub hash: [u8; 32],
    pub header: BlockHeader,
    pub height: u32,
    pub total_chainwork: U256,
}

impl Serialize for StoredData {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.hash);
        buffer.extend_from_slice(&self.header.serialize());
        buffer.extend_from_slice(&self.height.to_le_bytes());
        buffer.extend_from_slice(&self.total_chainwork.to_le_bytes());
        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, P2PError> {
        let (hash, rest) = bytes.split_at(32);
        let hash = hash.try_into()?;
        *bytes = rest;
        let header = BlockHeader::deserialize(bytes)?;
        let (height, rest) = bytes.split_at(4);
        let height = u32::from_le_bytes(height.try_into()?);
        *bytes = rest;

        let (total_chainwork, rest) = bytes.split_at(32);
        let total_chainwork = U256::from_le_slice(total_chainwork);

        *bytes = rest;
        Ok(Self {
            hash,
            header,
            height,
            total_chainwork,
        })
    }
}

const BASE_PATH: &str = "./bitcoin-protocol/src/index/db";
const HASH_GENESIS_BLOCK: &str = "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";
const CHAIN_TIP: &str = "__tip__";

impl HeaderStore {
    pub fn new() -> Result<Self, Error> {
        let store = open(BASE_PATH).map_err(|e| Error::Database(e))?;
        let mut genesis_hash: [u8; 32] = hex::decode(HASH_GENESIS_BLOCK)
            .map_err(|e| Error::FromHex(e))?
            .try_into()
            .map_err(|_| {
                Error::InvalidBlock(P2PError::Custom("Invalid genesis block".to_string()))
            })?;

        genesis_hash.reverse(); // Little endian bytes

        if !store
            .contains_key(genesis_hash)
            .map_err(|e| Error::Database(e))?
        {
            let height = 0;
            let bytes = hex::decode("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c").map_err(|e| Error::Parse(e.to_string()))?;
            let blockheader = BlockHeader::deserialize(&mut bytes.as_slice())
                .map_err(|e| Error::InvalidBlock(e))?;
            let chainwork = blockheader.get_chainwork();

            let data = StoredData {
                hash: genesis_hash,
                header: blockheader,
                height,
                total_chainwork: chainwork,
            }
            .serialize();

            let height_hash = "height_0";

            store
                .insert(height_hash, &genesis_hash)
                .map_err(|e| Error::Database(e))?;
            store
                .insert(CHAIN_TIP, &genesis_hash)
                .map_err(|e| Error::Database(e))?;

            store
                .insert(genesis_hash, data)
                .map_err(|e| Error::Database(e))?;
        }

        Ok(Self { db: store })
    }

    pub fn add_header(&mut self, hash: [u8; 32], header: BlockHeader) -> Result<(), Error> {
        let prev_block = self.get_header(&header.prev_block)?;
        let height = prev_block.height + 1;
        let total_chainwork = header.get_chainwork() + prev_block.total_chainwork;

        let data = StoredData {
            hash,
            header,
            height,
            total_chainwork,
        };

        let tip = self.chain_tip()?;

        self.db
            .insert(hash, data.serialize())
            .map_err(|e| Error::Database(e))?;

        if data.total_chainwork > tip.total_chainwork {
            self.db
                .insert(CHAIN_TIP, &hash)
                .map_err(|e| Error::Database(e))?;
        }
        let height_hash = format!("height_{}", height);

        self.db
            .insert(height_hash, &hash)
            .map_err(|e| Error::Database(e))?;

        Ok(())
    }

    pub fn get_header(&self, hash: &[u8; 32]) -> Result<StoredData, Error> {
        let data = self.db.get(hash).map_err(|e| Error::Database(e))?;
        if data.is_none() {
            return Err(Error::HashNotFound);
        }
        let data = data
            .ok_or(|e| Error::Database(e))
            .map_err(|_| Error::HashNotFound)?;

        StoredData::deserialize(&mut &data[..]).map_err(|e| Error::InvalidBlock(e))
    }

    pub fn chain_tip(&self) -> Result<StoredData, Error> {
        let tip_hash = self
            .db
            .get(CHAIN_TIP)
            .map_err(|e| Error::Database(e))?
            .ok_or(Error::HashNotFound)?;

        let hash_array: [u8; 32] = tip_hash[..]
            .try_into()
            .map_err(|_| Error::Parse("Tip hash has invalid length".to_string()))?;

        self.get_header(&hash_array)
    }

    pub fn get_header_by_height(&self, height: u32) -> Result<StoredData, Error> {
        let height_hash = format!("height_{}", height);

        if !self
            .db
            .contains_key(&height_hash)
            .map_err(|e| Error::Database(e))?
        {
            return Err(Error::HashNotFound);
        } else {
            let hash = self
                .db
                .get(height_hash)
                .map_err(|_| Error::HashNotFound)?
                .ok_or(Error::HashNotFound)?;

            return self.get_header(
                &hash[..]
                    .try_into()
                    .map_err(|_| Error::Parse("Cant find the header".to_string()))?,
            );
        };
    }
}
