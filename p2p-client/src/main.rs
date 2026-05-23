use bitcoin_protocol::{
    handshake::VersionMessage,
    inventory::{InvMessage, InvType, InvVector},
};

fn main() {
    let message = VersionMessage {
        version: 60002,
        services: 1,
        timestamp: 1355854353,
        addr_recv_service: 1,
        addr_recv_ip: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        addr_recv_port: 8333,
        addr_trans_service: 2,
        addr_trans_ip: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        addr_trans_port: 8333,
        nonce: 232832832,
        user_agent: "/Jesus:0.1.0/".to_string(),
        start_height: 212672,
        relay: false,
    };

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
    println!("InvMessage: {:#?}", my_inventory);

    let inv_serialized = my_inventory.serialize();
    println!("InvMessage serialized: {:#?}", inv_serialized);
    
    println!(
        "InvMessage deserialized: {:#?}",
        InvMessage::deserialize(&inv_serialized)
    )
}
