use std::{fmt::Display, io};

pub mod handshake;
pub mod inventory;
pub mod network;
pub mod peers;

pub fn encode_compact_size(var: usize) -> Vec<u8> {
    if var < 253 {
        vec![var as u8]
    } else if var <= 0xFFFF {
        let mut bytes = vec![0xFD];
        bytes.extend(&(var as u16).to_le_bytes());
        bytes
    } else if var <= 0xFFFFFFFF {
        let mut bytes = vec![0xFE];
        bytes.extend(&(var as u32).to_le_bytes());
        bytes
    } else {
        let mut bytes = vec![0xFF];
        bytes.extend(var.to_le_bytes());
        bytes
    }
}

pub fn decode_compact_size(bytes: &mut &[u8]) -> Result<u64, P2PError> {
    let (prefix, rest) = bytes.split_first().ok_or(P2PError::NotEnoughBytesToSplit)?;
    *bytes = rest;

    match *prefix {
        0..=252 => Ok(*prefix as u64),
        253 => {
            let (val, rest) = bytes.split_at(2);
            *bytes = rest;
            Ok(u16::from_le_bytes(val.try_into().map_err(|_| {
                P2PError::ParseError("Error while try to convert bytes into u16".to_string())
            })?) as u64)
        }
        254 => {
            let (val, rest) = bytes.split_at(4);
            *bytes = rest;
            Ok(u32::from_le_bytes(val.try_into().map_err(|_| {
                P2PError::ParseError("Error while try to convert bytes into u32".to_string())
            })?) as u64)
        }
        _ => {
            let (val, rest) = bytes.split_at(8);
            *bytes = rest;
            Ok(u64::from_le_bytes(val.try_into().map_err(|_| {
                P2PError::ParseError("Error while try to convert bytes into u64".to_string())
            })?) as u64)
        }
    }
}

#[derive(Debug)]
pub enum P2PError {
    CustomError(String),
    NotEnoughBytesToSplit,
    ParseError(String),
    OutOfRange,
    IoError(io::Error),
}

impl Display for P2PError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            P2PError::ParseError(s) => write!(f, "Error parsing: {}", s),
            P2PError::OutOfRange => write!(f, "Out of range"),
            P2PError::NotEnoughBytesToSplit => write!(f, "Not enough bytes to split"),
            P2PError::CustomError(s) => write!(f, "{}", s),
            P2PError::IoError(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for P2PError {}

impl From<io::Error> for P2PError {
    fn from(value: io::Error) -> Self {
        P2PError::IoError(value)
    }
}
