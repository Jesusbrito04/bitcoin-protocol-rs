use bitcoin_protocol::{
    inventory::{InvMessage, InvType, InvVector},
    network::{Addr, MsgHeader, ADDR, INV, MAINNET, PING, PONG},
    peers::{peer::Peer, PeerStore},
    P2PError, Serialize,
};
use std::io::{self, Read, Write};

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

    let peer_store = PeerStore::new()?;

    let mut peer = Peer::connect_str("74.48.195.218")?.do_handshake()?;
    peer.get_addr()?;

    loop {
        let mut network_mainnet: [u8; 4] = [0u8; 4];
        if let Err(e) = peer.stream.read_exact(&mut network_mainnet) {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                return Err(P2PError::Custom(
                    "The connection has been close by the remote node".to_string(),
                ));
            }
            continue;
        }
        if network_mainnet == MAINNET {
            let mut header: [u8; 20] = [0u8; 20];
            peer.stream.read_exact(&mut header)?;

            let mut network_command: [u8; 12] = [0u8; 12];
            network_command.copy_from_slice(&header[..12]);
            let mut payload_size: [u8; 4] = [0u8; 4];
            payload_size.copy_from_slice(&header[12..16]);
            let mut checksum: [u8; 4] = [0u8; 4];
            checksum.copy_from_slice(&header[16..20]);

            let receive_header = MsgHeader {
                magic: network_mainnet,
                command: network_command,
                payload_size: u32::from_le_bytes(payload_size),
                checksum,
            };

            match receive_header.command {
                ADDR => {
                    let mut payload = vec![0; receive_header.payload_size as usize];
                    peer.stream.read_exact(&mut payload)?;
                    let addresses = Addr::deserialize(&mut payload.as_ref())?;

                    for addr in addresses.ip_addresses {
                        peer_store.add_peer(addr)?;
                    }
                }
                PING => {
                    let command = PONG;
                    let mut payload: [u8; 8] = [0u8; 8];
                    peer.stream.read_exact(&mut payload)?;
                    let checksum = MsgHeader::calculate_checksum(&payload);

                    let pong = MsgHeader {
                        magic: receive_header.magic,
                        command,
                        payload_size: 8,
                        checksum,
                    }
                    .serialize();
                    println!("ping");
                    let mut message = Vec::new();
                    message.extend_from_slice(&pong);
                    message.extend_from_slice(&payload);
                    peer.stream.write_all(&message)?
                }

                INV => {
                    let mut payload = vec![0; receive_header.payload_size as usize];

                    peer.stream
                        .read_exact(&mut payload)
                        .expect("Error reading the payload");

                    let inv = InvMessage::deserialize(&payload);

                    println!("Inventory {:?}", inv)
                }
                _ => {
                    let mut payload = vec![0; receive_header.payload_size as usize];
                    peer.stream
                        .read_exact(&mut payload)
                        .expect("Error reading the payload");

                    println!(
                        "payload: {:?}",
                        String::from_utf8_lossy(&receive_header.command)
                    )
                }
            }
        }
    }
}
