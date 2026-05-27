use crate::{
    handshake::VersionMessage,
    network::{IpAddress, MsgHeader, MAINNET, VERACK, VERSION},
    P2PError,
};
use std::{
    io::{Read, Write},
    marker::PhantomData,
    net::{IpAddr, SocketAddr, TcpStream},
    time::SystemTime,
};

#[derive(Debug)]
pub struct Connected {}

#[derive(Debug)]
pub struct Disconnected {}

#[derive(Debug)]
pub struct Peer<State = Disconnected> {
    pub stream: TcpStream,
    pub peer: IpAddress,
    state: PhantomData<State>,
}

impl Peer {}

impl Peer<Disconnected> {
    pub fn connect_str(socket: &str) -> Result<Peer<Connected>, P2PError> {
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

        let mut stream = TcpStream::connect(socket)?;

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
        let version_serialize = version.serialize();
        let payload_size = version_serialize.len() as u32;
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

        stream
            .write_all(&message)
            .expect("Error sending the message");
        println!("Submmited message.");

        let mut response_header_bytes = [0u8; 24];
        stream.read_exact(&mut response_header_bytes)?;

        let response_header =
            MsgHeader::deserialize(&response_header_bytes).expect("Error reading the response");

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

        Ok(Peer {
            stream,
            peer,
            state: PhantomData::<Connected>,
        })
    }
}
