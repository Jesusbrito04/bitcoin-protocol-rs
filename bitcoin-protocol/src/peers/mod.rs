use crate::{decode_compact_size, encode_compact_size, network::IpAddress, P2PError};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Write},
};

const PEERSTOREVERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct PeerStore {
    pub version: u32,
    pub peers: HashSet<IpAddress>,
}

impl PeerStore {
    pub fn new() -> Self {
        PeerStore {
            version: PEERSTOREVERSION,
            peers: HashSet::new(),
        }
    }
    pub fn add_peer(&mut self, value: IpAddress) -> bool {
        self.peers.insert(value)
    }
    pub fn save(&self) {
        let peers_len = self.peers.len();
        let bytes_len = encode_compact_size(peers_len);
        let mut buffer: Vec<u8> = Vec::with_capacity(4 + bytes_len.len() + peers_len * 30);
        if let Ok(mut file) = File::create("./peers.dat") {
            buffer.extend_from_slice(&u32::to_le_bytes(self.version));
            buffer.extend_from_slice(&bytes_len);
            for addr in &self.peers {
                buffer.extend_from_slice(&addr.serialize());
            }

            if let Err(e) = file.write_all(&buffer) {
                eprintln!("Cant write in the file: {}", e)
            }

            println!("Save correctly")
        }
    }

    pub fn load() -> Result<Self, P2PError> {
        let mut file = match File::open("./peers.dat") {
            Ok(f) => f,
            Err(e) => return Err(e)?,
        };
        let mut buffer: Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut cursor = &buffer[..];

        let mut peers = HashSet::new();

        let (version, rest) = cursor.split_at(4);
        cursor = rest;
        let peers_len = decode_compact_size(&mut cursor).map_err(|e| e)?;
        for _ in 0..peers_len {
            let (peer, rest) = cursor.split_at(30);
            let addr = IpAddress::deserialize(&mut peer.as_ref()).unwrap();
            peers.insert(addr);
            cursor = rest
        }
        Ok(PeerStore {
            version: u32::from_le_bytes(version.try_into().unwrap()),
            peers,
        })
    }
}
