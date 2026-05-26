use std::collections::HashSet;
use crate::network::IpAddress;

#[derive(Debug)]
pub struct PeerStore {
    pub peers: HashSet<IpAddress>
}

impl PeerStore {
    pub fn new() -> Self {
        PeerStore {
            peers: HashSet::new()
        }
    }
    pub fn add_peer(&mut self, value: IpAddress) -> bool {
        self.peers.insert(value)
    }
}