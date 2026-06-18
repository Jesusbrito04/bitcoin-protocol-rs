use std::fmt::Display;

use crypto_bigint::U256;
use hex::FromHexError;
use sled::{open, Db, Error as dbError, IVec};

use crate::{inventory::block::BlockHeader, P2PError, Serialize};

#[derive(Debug)]
pub enum Error {
    Database(dbError),
    HashNotFound,
    InvalidBlock(String),
    FromHex(FromHexError),
    Parse(String),
    Custom(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Custom(err) => writeln!(f, "{}", err),
            Error::Database(err) => writeln!(f, "Database error: {}", err),
            Error::InvalidBlock(err) => writeln!(f, "Invalid block: {}", err),
            Error::HashNotFound => writeln!(f, "Hash not found"),
            Error::Parse(err) => writeln!(f, "Error while try to parse: {}", err),
            Error::FromHex(err) => writeln!(f, "Try parsing to hex: {}", err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Custom(value.to_string())
    }
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
const CHAIN_TIP: &str = "__tip__";

#[derive(Debug)]
pub struct HeaderStore {
    pub db: Db,
}

impl HeaderStore {
    pub fn new() -> Result<Self, Error> {
        let store = open(BASE_PATH).map_err(|e| Error::Database(e))?;
        Ok(Self { db: store })
    }

    pub fn insert_db<K: AsRef<[u8]>, V: Into<IVec>>(
        &self,
        key: K,
        value: V,
    ) -> Result<Option<IVec>, Error> {
        Ok(self.db.insert(key, value).map_err(|e| Error::Database(e))?)
    }

    pub fn contains_key<K: AsRef<[u8]>>(&self, key: K) -> Result<bool, Error> {
        Ok(self.db.contains_key(key).map_err(|e| Error::Database(e))?)
    }

    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<IVec>, Error> {
        Ok(self.db.get(key).map_err(|e| Error::Database(e))?)
    }
}
