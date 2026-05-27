use bitcoin_protocol::{
    handshake::VersionMessage,
    inventory::{InvMessage, InvType, InvVector},
    network::{Addr, MsgHeader, ADDR, GETADDR, INV, MAINNET, PING, PONG, VERACK, VERSION},
    peers::PeerStore,
};
use std::{
    io::{self, Error, ErrorKind, Read, Write},
    net::TcpStream,
};

fn main() -> Result<(), P2PError> {
    let version = VersionMessage {
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
    let _inv_serialized = my_inventory.serialize();

    let version_serialize = version.serialize();
    let payload_size: u32 = version_serialize.len().try_into().unwrap();
    let checksum = MsgHeader::calculate_checksum(&version_serialize);

    let header_msg = MsgHeader {
        magic: MAINNET,
        command: VERSION,
        payload_size,
        checksum,
    }
    .serialize();

    let mut message: Vec<u8> = Vec::new();
    message.extend_from_slice(&header_msg);
    message.extend_from_slice(&version_serialize);

    let mut peer_store = PeerStore::new();
    if let Ok(mut stream) = TcpStream::connect("74.48.195.218:8333") {
        println!("Connect Successfully: {:?}", stream);
        stream
            .write_all(&message)
            .expect("Error sending the message");
        println!("Submmited message.");

        let mut response_header_bytes = [0u8; 24];
        stream.read_exact(&mut response_header_bytes)?;
        let response_header =
            MsgHeader::deserialize(&response_header_bytes).expect("Error reading the response");
        println!("{:?}", response_header);

        let mut response_payload_bytes = vec![0u8; response_header.payload_size as usize];
        stream.read_exact(&mut response_payload_bytes)?;
        let response_payload =
            VersionMessage::deserialize(&response_payload_bytes).expect("Error reading payload");
        println!("Hi: {}", response_payload.user_agent);

        let verack = MsgHeader {
            magic: MAINNET,
            command: VERACK,
            payload_size: 0,
            checksum: [0x5d, 0xf6, 0xe0, 0xe2],
        };

        stream.write_all(&verack.serialize())?;
        println!("Verack Submmited");

        let mut response_verack_bytes = [0u8; 24];
        stream.read_exact(&mut response_verack_bytes)?;
        let _response_verack =
            MsgHeader::deserialize(&response_verack_bytes).expect("Error reading the response");
        println!("got Verack.");

        let getaddr = &MsgHeader {
            magic: MAINNET,
            command: GETADDR,
            payload_size: 0,
            checksum: [0x5d, 0xf6, 0xe0, 0xe2],
        }
        .serialize();

        stream.write_all(getaddr).unwrap();
        loop {
            let mut network_mainnet: [u8; 4] = [0u8; 4];
            if let Err(e) = stream.read_exact(&mut network_mainnet) {
                eprintln!("Error reading stream bytes: {}", e);
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Err(P2PError::CustomError(
                        "The connection has been close by the remote node".to_string()
                    ));
                }
                continue;
            }
            if network_mainnet == MAINNET {
                let mut header: [u8; 20] = [0u8; 20];
                stream.read_exact(&mut header).unwrap();

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
                        stream.read_exact(&mut payload).unwrap();
                        let addresses = Addr::deserialize(&payload).unwrap();

                        for addr in addresses.ip_addresses {
                            peer_store.add_peer(addr);
                        }

                        peer_store.save();
                    }
                    PING => {
                        let command = PONG;
                        let mut payload: [u8; 8] = [0u8; 8];
                        stream
                            .read_exact(&mut payload)
                            .expect("Error reading the payload");
                        let checksum = MsgHeader::calculate_checksum(&payload);

                        let pong = MsgHeader {
                            magic: receive_header.magic,
                            command,
                            payload_size: 8,
                            checksum,
                        }
                        .serialize();

                        let mut message = Vec::new();
                        message.extend_from_slice(&pong);
                        message.extend_from_slice(&payload);
                        stream.write_all(&message).unwrap()
                    }

                    INV => {
                        let mut payload = vec![0; receive_header.payload_size as usize];

                        stream
                            .read_exact(&mut payload)
                            .expect("Error reading the payload");

                        let inv = InvMessage::deserialize(&payload);

                        println!("Inventory {:?}", inv)
                    }
                    _ => {
                        let mut payload = vec![0; receive_header.payload_size as usize];
                        stream
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
    } else {
        Err(Error::from(ErrorKind::ConnectionRefused))?
    }
}
