//! The handshake is a step series to get a successful connection to a peer. !//

use crate::{decode_compact_size, encode_compact_size, P2PError};

// The version message provides information about the transmitting node to the receiving node at the beginning of a connection.
// until both peers have exchanged "version" messages. No other messages will be accepted.

// If a "version" message is accepted, the receiving node should send a "verack" (version acknowledgment) message.
#[derive(Debug)]
pub struct VersionMessage {
    pub version: i32,
    pub services: u64,
    pub timestamp: i64,
    pub addr_recv_service: u64,
    pub addr_recv_ip: [u8; 16],
    pub addr_recv_port: u16,
    pub addr_trans_service: u64,
    pub addr_trans_ip: [u8; 16],
    pub addr_trans_port: u16,
    pub nonce: u64,
    pub user_agent: String,
    pub start_height: i32,
    pub relay: bool,
}

impl VersionMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(85);
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.services.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.addr_recv_service.to_le_bytes());
        bytes.extend_from_slice(&self.addr_recv_ip);
        bytes.extend_from_slice(&self.addr_recv_port.to_be_bytes());
        bytes.extend_from_slice(&self.addr_trans_service.to_le_bytes());
        bytes.extend_from_slice(&self.addr_trans_ip);
        bytes.extend_from_slice(&self.addr_trans_port.to_be_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());

        let user_agent_bytes = self.user_agent.as_bytes();
        bytes.extend_from_slice(&encode_compact_size(user_agent_bytes.len()));
        bytes.extend_from_slice(user_agent_bytes);
        bytes.extend_from_slice(&self.start_height.to_le_bytes());
        bytes.push(if self.relay { 1 } else { 0 });

        bytes
    }

    pub fn deserialize(mut bytes: &[u8]) -> Result<Self, P2PError> {
        if bytes.len() < 4 + 8 + 8 {
            return Err(P2PError::Custom(
                "Invalid Version Message length".to_string(),
            ));
        }
        let version =
            i32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into i32"))
            })?);
        let services =
            u64::from_le_bytes(bytes[4..12].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into u64"))
            })?);
        let timestamp =
            i64::from_le_bytes(bytes[12..20].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into i64"))
            })?);
        bytes = &bytes[20..];

        if bytes.len() < 8 + 16 + 2 {
            return Err(P2PError::Custom(
                "Invalid Version Message length".to_string(),
            ));
        }
        let addr_recv_service =
            u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into u64"))
            })?);
        let addr_recv_ip: [u8; 16] = bytes[8..24]
            .try_into()
            .map_err(|err| P2PError::Parse(format!("{}", err)))?;
        let addr_recv_port =
            u16::from_be_bytes(bytes[24..26].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into u16"))
            })?);
        bytes = &bytes[26..];

        if bytes.len() < 8 + 16 + 2 {
            return Err(P2PError::Custom(
                "Invalid Version Message length".to_string(),
            ));
        }
        let addr_trans_service =
            u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into u64"))
            })?);
        let addr_trans_ip: [u8; 16] = bytes[8..24]
            .try_into()
            .map_err(|err| P2PError::Parse(format!("{}", err)))?;
        let addr_trans_port =
            u16::from_be_bytes(bytes[24..26].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into u16"))
            })?);
        bytes = &bytes[26..];

        if bytes.len() < 8 {
            return Err(P2PError::Custom(
                "Invalid Version Message length".to_string(),
            ));
        }
        let nonce =
            u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into u64"))
            })?);
        bytes = &bytes[8..];

        let user_agent_len = decode_compact_size(&mut bytes)? as usize;
        if bytes.len() < user_agent_len {
            return Err(P2PError::Custom(
                "Invalid Version Message length".to_string(),
            ));
        }
        let (ua_slice, rest) = bytes.split_at(user_agent_len);
        let user_agent = String::from_utf8(ua_slice.to_vec()).map_err(|_| {
            P2PError::Parse(format!("Error while try to convert bytes into String"))
        })?;
        bytes = rest;

        if bytes.len() < 4 + 1 {
            return Err(P2PError::Custom(
                "Invalid Version Message length".to_string(),
            ));
        }
        let start_height =
            i32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                P2PError::Parse(format!("Error while try to convert bytes into i32"))
            })?);
        let relay = bytes[4] != 0;

        Ok(Self {
            version,
            services,
            timestamp,
            addr_recv_service,
            addr_recv_ip,
            addr_recv_port,
            addr_trans_service,
            addr_trans_ip,
            addr_trans_port,
            nonce,
            user_agent,
            start_height,
            relay,
        })
    }
}
