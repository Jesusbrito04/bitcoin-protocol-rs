use crate::{
    index::store::HeaderStore,
    peers::{Peer, PeerStore},
    P2PError,
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{mpsc::Sender, Arc, Mutex},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct PeersManager {
    pub handlers: Vec<JoinHandle<Result<(), P2PError>>>,
    pub store: Arc<PeerStore>,
}

impl PeersManager {
    pub fn new(store: Arc<PeerStore>) -> Self {
        Self {
            handlers: vec![],
            store,
        }
    }
    pub fn manager(&mut self, tx: Sender<String>) -> Result<(), P2PError> {
        let header_store = HeaderStore::new()
            .map_err(|_| P2PError::Custom(format!("Error on initialize header store")))?;
        if self.handlers.len() <= 5 {
            let tx = Arc::new(tx);
            let header_store = Arc::new(Mutex::new(header_store));

            for peer in self.store.get_peers()? {
                let store = self.store.clone();
                let sender = tx.clone();
                let header_store: Arc<Mutex<HeaderStore>> = header_store.clone();

                let handler = thread::spawn(move || -> Result<(), P2PError> {
                    let socketaddr = SocketAddr::new(IpAddr::from(peer.ip), peer.port);
                    let mut peer: Peer<crate::peers::peer::Connected> =
                        Peer::connect_str(&socketaddr.to_string())?.do_handshake()?;

                    sender
                        .send(format!("Connected correclty with peer: {:#?}", peer.peer))
                        .unwrap();

                    peer.run(store, header_store)?;
                    Ok(())
                });
                self.handlers.push(handler);
            }
        }
        return Ok(());
    }
}
