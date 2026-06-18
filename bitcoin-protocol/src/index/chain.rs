use std::ops::Div;

use crypto_bigint::U256;
use hex::encode;
use sha2::{Digest, Sha256};

use crate::{
    index::store::{Error, HeaderStore, StoredData},
    inventory::block::{BlockHeader, Headers},
    P2PError, Serialize,
};

const HASH_GENESIS_BLOCK: &str = "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";
const HEADER_GENESIS_BLOCK: &str = "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c";
const CHAIN_TIP: &str = "__tip__";

pub struct BlockChain {
    mtp: Vec<BlockHeader>,
    store: HeaderStore,
}

impl BlockChain {
    pub fn new() -> Result<Self, P2PError> {
        let store: HeaderStore = HeaderStore::new().map_err(|e| P2PError::DbError(e))?;

        Ok(Self {
            mtp: Vec::new(),
            store,
        })
    }

    pub fn init_sync(&self, headers: Headers) -> Result<(), P2PError> {
        let mut genesis_hash: [u8; 32] = hex::decode(HASH_GENESIS_BLOCK)
            .map_err(|_| P2PError::Custom("Error parsing hash hex".to_string()))?
            .try_into()
            .map_err(|_| P2PError::Custom("Error trying to parse from Vec".to_string()))?;

        genesis_hash.reverse(); // Little endian bytes

        if !self.contains_header(&genesis_hash)? {
            let height = 0;
            let header_bytes = hex::decode(HEADER_GENESIS_BLOCK)
                .map_err(|_| P2PError::Custom("Error parsing header hex".to_string()))?;
            let blockheader = BlockHeader::deserialize(&mut header_bytes.as_slice())?;
            let chainwork = blockheader.get_chainwork();

            let data = StoredData {
                hash: genesis_hash,
                header: blockheader,
                height,
                total_chainwork: chainwork,
            }
            .serialize();

            let height_hash = b"height_0";

            self.store
                .insert_db(height_hash, &genesis_hash)
                .map_err(|e| P2PError::DbError(e))?;
            self.store
                .insert_db(CHAIN_TIP, &genesis_hash)
                .map_err(|e| P2PError::DbError(e))?;

            self.store
                .insert_db(genesis_hash, data)
                .map_err(|e| P2PError::DbError(e))?;
        }

        let mut current_tip = self
            .chain_tip()
            .map_err(|e| P2PError::Custom(format!("{:?}", e)))?;

        for block_h in headers.headers {
            if current_tip.hash == block_h.prev_block {
                let hash = Sha256::digest(block_h.serialize());
                let mut hash2 = Sha256::digest(hash);

                let expected_target = self.compute_next_target()?;
                let expected_nbits = BlockHeader::target_to_nbits(expected_target);

                if expected_nbits != block_h.nbits {
                    return Err(P2PError::Custom("Error target incorrect".to_string()));
                }

                if !block_h.validate_pow() {
                    return Err(P2PError::Custom("Invalid Proof of work".to_string()));
                }

                self.add_header(hash2[..].try_into()?, block_h)?;

                hash2.0.reverse();

                current_tip = StoredData {
                    height: current_tip.height + 1,
                    hash: hash2[..].try_into()?,
                    header: block_h,
                    total_chainwork: current_tip.total_chainwork + block_h.get_chainwork(),
                };

                println!(
                    "Added block hash: {:?} height: {}",
                    encode(hash2.0),
                    current_tip.height + 1
                );
            }
        }
        Ok(())
    }

    pub fn add_header(&self, hash: [u8; 32], header: BlockHeader) -> Result<(), P2PError> {
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

        self.store
            .insert_db(hash, data.serialize())
            .map_err(|e| P2PError::DbError(e))?;

        if data.total_chainwork > tip.total_chainwork {
            self.store
                .insert_db(CHAIN_TIP, &hash)
                .map_err(|e| P2PError::DbError(e))?;
        }
        let height_hash = format!("height_{}", height);

        self.store
            .insert_db(height_hash, &hash)
            .map_err(|e| P2PError::DbError(e))?;

        Ok(())
    }

    pub fn get_header(&self, hash: &[u8; 32]) -> Result<StoredData, P2PError> {
        let data = self.store.get(hash).map_err(|e| P2PError::DbError(e))?;
        if data.is_none() {
            return Err(P2PError::DbError(Error::HashNotFound));
        }
        let data = data
            .ok_or(|e| Error::Database(e))
            .map_err(|_| P2PError::DbError(Error::HashNotFound))?;

        Ok(StoredData::deserialize(&mut &data[..])?)
    }

    pub fn chain_tip(&self) -> Result<StoredData, P2PError> {
        let tip_hash = self
            .store
            .get(CHAIN_TIP)
            .map_err(|e| P2PError::DbError(e))?
            .ok_or(P2PError::DbError(Error::HashNotFound))?;

        let hash_array: [u8; 32] = tip_hash[..].try_into()?;

        self.get_header(&hash_array)
    }

    pub fn compute_next_target(&self) -> Result<U256, P2PError> {
        let block_tip = self.chain_tip()?;
        let block_height = block_tip.height;

        if (block_height + 1) % 2016 != 0 {
            let target = block_tip.header.get_target();
            return Ok(target);
        }

        let first_timestamp = self
            .get_header_by_height(block_height - 2015)?
            .header
            .timestamp;
        let last_timestamp = block_tip.header.timestamp;

        let actual = last_timestamp - first_timestamp;
        let expected = 2016 * 10 * 60;

        let actual_modified = if actual < 302400 {
            302400
        } else if actual > 4838400 {
            4838400
        } else {
            actual
        };

        let current_target = block_tip.header.get_target();
        let new_target = current_target
            .checked_mul(&U256::from_u32(actual_modified))
            .ok_or(P2PError::Custom("Cant multiply operation".to_string()))?
            .div(U256::from_u32(expected));

        let max_target =
            U256::from_be_hex("00000000ffff0000000000000000000000000000000000000000000000000000");
        if new_target.gt(&max_target) {
            return Ok(max_target);
        }

        Ok(new_target)
    }

    pub fn get_header_by_height(&self, height: u32) -> Result<StoredData, P2PError> {
        let height_hash = format!("height_{}", height);

        if !self
            .store
            .contains_key(&height_hash)
            .map_err(|e| P2PError::DbError(e))?
        {
            return Err(P2PError::DbError(Error::HashNotFound));
        } else {
            let hash = self
                .store
                .get(height_hash)
                .map_err(|_| P2PError::DbError(Error::HashNotFound))?
                .ok_or(P2PError::Parse("Error converting to result".to_string()))?;

            return self.get_header(&hash[..].try_into()?);
        };
    }

    pub fn contains_header(&self, hash: &[u8; 32]) -> Result<bool, P2PError> {
        self.store
            .contains_key(hash)
            .map_err(|e| P2PError::DbError(e))
    }
}
