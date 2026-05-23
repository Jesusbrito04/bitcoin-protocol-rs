pub mod handshake;
pub mod inventory;

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
                P2PError::ConvertionError("Error while try to convert bytes into u16".to_string())
            })?) as u64)
        }
        254 => {
            let (val, rest) = bytes.split_at(4);
            *bytes = rest;
            Ok(u32::from_le_bytes(val.try_into().map_err(|_| {
                P2PError::ConvertionError("Error while try to convert bytes into u32".to_string())
            })?) as u64)
        }
        _ => {
            let (val, rest) = bytes.split_at(8);
            *bytes = rest;
            Ok(u64::from_le_bytes(val.try_into().map_err(|_| {
                P2PError::ConvertionError("Error while try to convert bytes into u64".to_string())
            })?) as u64)
        }
    }
}

#[derive(Debug, Clone)]
pub enum P2PError {
    InvalidVersionMessageLen,
    ConvertionError(String),
    NotEnoughBytesToSplit,
    ParseError(String),
    OutOfRange,
}
