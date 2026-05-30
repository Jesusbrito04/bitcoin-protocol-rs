use std::sync::{mpsc::channel, Arc};

use bitcoin_protocol::{
    inventory::{InvMessage, InvType, InvVector},
    peers::{manager::PeersManager, PeerStore},
    P2PError, Serialize,
};

fn main() -> Result<(), P2PError> {
    let msg_tx = InvVector {
        inv_type: InvType::MsgTx,
        inv_hash: [
            0x4b, 0x8e, 0x1f, 0x0c, 0x9c, 0x7a, 0x6a, 0x2d, 0x3f, 0x4e, 0x5a, 0x11, 0x87, 0xc9,
            0xb1, 0xd0, 0xa2, 0xf3, 0xc4, 0xe5, 0xb6, 0xa7, 0xd8, 0xc9, 0xe0, 0xf1, 0xa2, 0xb3,
            0xc4, 0xd5, 0xe6, 0xf7,
        ],
    };

    let msg_block = InvVector {
        inv_type: InvType::MsgBlock,
        inv_hash: [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8b, 0x15, 0xe2, 0xdb,
            0xff, 0xc3, 0xce, 0x07, 0x76, 0x75, 0x13, 0x54, 0xdd, 0x22, 0x08, 0x12, 0x67, 0x2d,
            0xf4, 0x87, 0x2c, 0xf4,
        ],
    };

    let my_inventory = InvMessage {
        inventory: vec![msg_tx, msg_block],
    };
    let _inv_serialized = my_inventory.serialize();
    let (sender, receiver) = channel();
    let store = Arc::new(PeerStore::new()?);
    let mut manager = PeersManager::new(store);
    manager.manager(sender)?;

    for rx in receiver.iter() {
        println!("{:?}", rx)
    }
    println!("Shutting down gracefully...");
    for handler in manager.handlers {
        let _ = handler.join().unwrap().unwrap();
    }
    Ok(())
}
