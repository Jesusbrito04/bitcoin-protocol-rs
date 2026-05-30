use crate::{
    handshake::VersionMessage,
    inventory::{block::Block, InvMessage},
    network::{
        Addr, IpAddress, MsgHeader, ADDR, BLOCK, GETADDR, GETDATA, INV, MAINNET, PING, PONG,
        VERACK, VERSION,
    },
    peers::PeerStore,
    P2PError, Serialize,
};
use std::{
    io::{self, Read, Write},
    marker::PhantomData,
    net::{IpAddr, SocketAddr, TcpStream},
    sync::Arc,
    time::SystemTime,
};

#[derive(Debug)]
pub struct Connected {}

#[derive(Debug)]
pub struct Disconnected {}

#[derive(Debug)]
pub struct Handshake {}

#[derive(Debug)]
pub struct Peer<State = Disconnected> {
    pub stream: TcpStream,
    pub peer: IpAddress,
    state: PhantomData<State>,
}

impl Peer<Disconnected> {
    pub fn connect_str(socket: &str) -> Result<Peer<Handshake>, P2PError> {
        let socket_address = if socket.contains(":") {
            let socket: SocketAddr = socket
                .parse()
                .map_err(|e| P2PError::Parse(format!("{}", e)))?;
            socket
        } else {
            let socket: SocketAddr = SocketAddr::new(
                socket
                    .parse()
                    .map_err(|e| P2PError::Parse(format!("{}", e)))?,
                8333,
            );
            socket
        };

        let socket = SocketAddr::new(socket_address.ip(), socket_address.port());

        let ip = match socket_address.ip() {
            IpAddr::V4(ip) => ip.to_ipv6_compatible().octets(),
            IpAddr::V6(ip) => ip.octets(),
        };

        let stream = TcpStream::connect(socket)?;

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| P2PError::Custom("Can't get the unix timestamp".to_string()))?
            .as_secs()
            .try_into()
            .map_err(|_| P2PError::Parse("Can't parse Duration to secs".to_string()))?;

        let peer = IpAddress {
            ip,
            service: 1,
            port: 8333,
            time: timestamp,
        };

        Ok(Peer {
            stream,
            peer,
            state: PhantomData::<Handshake>,
        })
    }
}

impl Peer<Handshake> {
    pub fn do_handshake(mut self) -> Result<Peer<Connected>, P2PError> {
        // Build the Version-Message payload (identify my node).
        let version_serialize = VersionMessage {
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
        }
        .serialize();

        // Build the Message-Header.
        let payload_size = version_serialize.len() as u32;
        let checksum = MsgHeader::calculate_checksum(&version_serialize);
        let header_msg = MsgHeader {
            magic: MAINNET,
            command: VERSION,
            payload_size,
            checksum,
        }
        .serialize();

        // Build and send the message stream-bytes.
        let mut message: Vec<u8> = Vec::new();
        message.extend_from_slice(&header_msg);
        message.extend_from_slice(&version_serialize);
        self.stream.write_all(&message)?;
        println!("Submmited message.");

        // Spect his Version-Message as a response.
        let mut buffer = [0u8; 24];
        self.stream.read_exact(&mut buffer)?;
        let response_header = MsgHeader::deserialize(&mut buffer.as_ref())?;

        if response_header.command != VERSION {
            return Err(P2PError::Custom(
                "Can't establish the handshake correctly".to_string(),
            ));
        }

        // Read his Version-Message payload.
        let mut buffer = vec![0u8; response_header.payload_size as usize];
        self.stream.read_exact(&mut buffer)?;
        let _response_payload = VersionMessage::deserialize(&mut buffer.as_ref())?; // TODO: Verify version

        // Build and send version-acknowledge
        let verack = MsgHeader {
            magic: MAINNET,
            command: VERACK,
            payload_size: 0,
            checksum: [0x5d, 0xf6, 0xe0, 0xe2],
        };
        self.stream.write_all(&verack.serialize())?;
        println!("Verack Submmited");

        // Spect his version-acknowledge as a response.
        let mut buffer = [0u8; 24];
        self.stream.read_exact(&mut buffer)?;
        let _response_verack =
            MsgHeader::deserialize(&mut buffer.as_ref()).expect("Error reading the response");
        println!("got Verack.");

        Ok(Peer {
            stream: self.stream,
            peer: self.peer,
            state: PhantomData::<Connected>,
        })
    }
}

impl Peer<Connected> {
    pub fn get_addr(&mut self) -> Result<(), P2PError> {
        let getaddr = &MsgHeader {
            magic: MAINNET,
            command: GETADDR,
            payload_size: 0,
            checksum: [0x5d, 0xf6, 0xe0, 0xe2],
        }
        .serialize();

        self.stream.write_all(getaddr)?;
        Ok(())
    }

    pub fn run(&mut self, store: Arc<PeerStore>) -> Result<(), P2PError> {
        loop {
            let mut network_mainnet: [u8; 4] = [0u8; 4];
            if let Err(e) = self.stream.read_exact(&mut network_mainnet) {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Err(P2PError::Custom(
                        "The connection has been close by the remote node".to_string(),
                    ));
                }
                continue;
            }
            if network_mainnet == MAINNET {
                let mut header: [u8; 20] = [0u8; 20];
                self.stream.read_exact(&mut header)?;

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
                        self.stream.read_exact(&mut payload)?;
                        let addresses = Addr::deserialize(&mut payload.as_ref())?;
                        for addr in addresses.ip_addresses {
                            store.add_peer(addr)?;
                        }
                    }
                    PING => {
                        let command = PONG;
                        let mut payload: [u8; 8] = [0u8; 8];
                        self.stream.read_exact(&mut payload)?;
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
                        self.stream.write_all(&message)?
                    }
                    INV => {
                        let mut buffer_payload = vec![0; receive_header.payload_size as usize];
                        self.stream.read_exact(&mut buffer_payload)?;
                        let inv = InvMessage::deserialize(&mut buffer_payload.as_ref());
                        let get_data_payload = inv?.serialize();
                        let get_data_header = MsgHeader {
                            magic: MAINNET,
                            command: GETDATA,
                            payload_size: get_data_payload.len() as u32,
                            checksum: MsgHeader::calculate_checksum(&get_data_payload),
                        };
                        let mut message: Vec<u8> =
                            Vec::with_capacity(24 + get_data_header.payload_size as usize);
                        message.extend_from_slice(&get_data_header.serialize());
                        message.extend_from_slice(&get_data_payload);
                        self.stream.write_all(&message)?;
                    }
                    BLOCK => {
                        let mut buffer_payload = vec![0; receive_header.payload_size as usize];
                        self.stream.read_exact(&mut buffer_payload)?;
                        let block = Block::deserialize(&mut buffer_payload.as_ref())?;
                    }
                    _ => {
                        let mut payload = vec![0; receive_header.payload_size as usize];
                        self.stream
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
}
