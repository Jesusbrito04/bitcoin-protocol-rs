use crate::{network::IpAddress, P2PError, Serialize};
use sled::{open, Db, IVec};

pub mod peer;
pub use peer::Peer;
pub mod manager;

const CURRENT_DB_VERSION: u32 = 1;
const BASE_PATH: &str = "./bitcoin-protocol/src/peers/db";

#[derive(Debug, Clone)]
pub struct PeerStore {
    pub version: u32,
    pub db: Db,
}

impl PeerStore {
    pub fn new() -> Result<Self, P2PError> {
        let db =
            open(BASE_PATH).map_err(|_| P2PError::Custom("Error opening database".to_string()))?;
        if let Some(version) = db
            .get(b"__version__")
            .map_err(|_| P2PError::Custom("Not version db found".to_string()))?
        {
            if version != CURRENT_DB_VERSION.to_le_bytes() {
                return Err(P2PError::Custom("Incompatible db version".to_string()));
            };
        }
        let _ = db
            .insert(b"__version__", &CURRENT_DB_VERSION.to_le_bytes())
            .map_err(|_| "Error inserting db version");
        Ok(PeerStore {
            version: CURRENT_DB_VERSION,
            db: db,
        })
    }
    pub fn add_peer(&self, value: IpAddress) -> Result<Option<IVec>, P2PError> {
        let mut key: [u8; 18] = [0u8; 18];
        key[..16].copy_from_slice(&value.ip);
        key[16..].copy_from_slice(&value.port.to_be_bytes());

        let value = value.serialize();
        self.db
            .insert(key, value)
            .map_err(|_| P2PError::Custom("Error to insert peer".to_string()))
    }
    pub fn get_peers(&self) -> Result<Vec<IpAddress>, P2PError> {
        let mut peers: Vec<IpAddress> = Vec::new();
        for item in self.db.iter() {
            let (key, value) =
                item.map_err(|_| P2PError::Custom("Error to get peer".to_string()))?;
            if key == b"__version__" {
                continue;
            }
            let mut value = &value[..];
            let peer = IpAddress::deserialize(&mut value)?;
            peers.push(peer);
        }
        Ok(peers)
    }
}
