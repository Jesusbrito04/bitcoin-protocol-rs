use crate::{decode_compact_size, encode_compact_size, Serialize};

#[derive(Debug)]
pub struct Transaction {
    is_segwit: bool,
    version: u32,
    market_flag: Option<u16>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    witnesses: Option<Vec<Vec<WitnessItem>>>,
    locktime: u32,
}

impl Serialize for Transaction {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.version.to_le_bytes());

        if self.is_segwit && self.market_flag.is_some() {
            buffer.extend_from_slice(&self.market_flag.unwrap().to_le_bytes());
        }

        let cmpct_size_txin = encode_compact_size(self.inputs.len() as usize);
        buffer.extend_from_slice(&cmpct_size_txin);
        for input in &self.inputs {
            buffer.extend_from_slice(&input.serialize());
        }

        let cmpct_size_txout = encode_compact_size(self.outputs.len() as usize);
        buffer.extend_from_slice(&cmpct_size_txout);
        for output in &self.outputs {
            buffer.extend_from_slice(&output.serialize());
        }

        if self.is_segwit {
            if let Some(witnesses) = &self.witnesses {
                for witness_items in witnesses {
                    let cmpct_size_witnesses = encode_compact_size(witness_items.len() as usize);
                    buffer.extend_from_slice(&cmpct_size_witnesses);
                    for witness in witness_items {
                        buffer.extend_from_slice(&witness.serialize());
                    }
                }
            }
        }

        buffer.extend_from_slice(&self.locktime.to_le_bytes());

        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let mut is_segwit: bool = false;
        let (version, rest) = bytes.split_at(4);
        let version = u32::from_le_bytes(version.try_into()?);
        *bytes = rest;
        let mut market_flag = None;

        if bytes[0] == 0x00 {
            let (market, rest) = bytes.split_at(2);
            market_flag = Some(u16::from_le_bytes(market.try_into()?));
            *bytes = rest;
            is_segwit = true
        }

        let mut inputs: Vec<Input> = Vec::new();
        let input_count = decode_compact_size(bytes)?;
        for _ in 0..input_count {
            let input = Input::deserialize(bytes)?;
            inputs.push(input);
        }

        let mut outputs: Vec<Output> = Vec::new();
        let output_count = decode_compact_size(bytes)?;
        for _ in 0..output_count {
            let output = Output::deserialize(bytes)?;
            outputs.push(output);
        }

        let mut witnesses: Option<Vec<Vec<WitnessItem>>> = None;

        if is_segwit {
            let mut all_witnesses = Vec::new();
            for _ in 0..inputs.len() {
                let mut witness_items: Vec<WitnessItem> = Vec::new();
                let witness_count = decode_compact_size(bytes)?;
                for _ in 0..witness_count {
                    let witness = WitnessItem::deserialize(bytes)?;
                    witness_items.push(witness);
                }
                all_witnesses.push(witness_items)
            }
            witnesses = Some(all_witnesses);
        }

        let (locktime, rest) = bytes.split_at(4);
        let locktime = u32::from_le_bytes(locktime.try_into()?);

        *bytes = rest;

        Ok(Self {
            is_segwit,
            version,
            market_flag,
            inputs,
            outputs,
            witnesses,
            locktime,
        })
    }
}

#[derive(Debug)]
pub struct Input {
    txid: [u8; 32],
    vout: u32,
    script_sig: Vec<u8>,
    sequence: u32,
}

impl Serialize for Input {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(36 + 5);
        buffer.extend_from_slice(&self.txid);
        buffer.extend_from_slice(&self.vout.to_le_bytes());
        buffer.extend_from_slice(&encode_compact_size(self.script_sig.len() as usize));
        buffer.extend_from_slice(&self.script_sig);
        buffer.extend_from_slice(&self.sequence.to_le_bytes());
        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let (txid, rest) = bytes.split_at(32);
        let txid = txid.try_into()?;
        *bytes = rest;

        let (vout, rest) = bytes.split_at(4);
        let vout = u32::from_le_bytes(vout.try_into()?);
        *bytes = rest;

        let script_sig_len = decode_compact_size(bytes)?;
        let (script_sig, rest) = bytes.split_at(script_sig_len as usize);
        let script_sig = script_sig.to_vec();
        *bytes = rest;

        let (sequence, rest) = bytes.split_at(4);
        let sequence = u32::from_le_bytes(sequence.try_into()?);

        *bytes = rest;

        Ok(Self {
            txid,
            vout,
            script_sig,
            sequence,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Output {
    amount: u64,
    script_pubkey: Vec<u8>,
}

impl Serialize for Output {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let cmpct_script_pubkey = encode_compact_size(self.script_pubkey.len());
        let mut buffer = Vec::with_capacity(8 + cmpct_script_pubkey.len() + 22);
        buffer.extend_from_slice(&self.amount.to_le_bytes());
        buffer.extend_from_slice(&cmpct_script_pubkey);
        buffer.extend_from_slice(&self.script_pubkey);
        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let (amount, rest) = bytes.split_at(8);
        let amount = u64::from_le_bytes(amount.try_into()?);
        *bytes = rest;

        let cmpct_script_pubkey = decode_compact_size(bytes)?;
        let (script_pubkey, rest) = bytes.split_at(cmpct_script_pubkey as usize);
        let script_pubkey = script_pubkey.to_vec();

        *bytes = rest;

        Ok(Self {
            amount,
            script_pubkey,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WitnessItem {
    item: Vec<u8>,
}

impl Serialize for WitnessItem {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let witness_len = encode_compact_size(self.item.len());
        buffer.extend_from_slice(&witness_len);
        buffer.extend_from_slice(&self.item);
        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, crate::P2PError> {
        let script_sig_len = decode_compact_size(bytes)?;
        let (item, rest) = bytes.split_at(script_sig_len as usize);
        let item = item.to_vec();
        *bytes = rest;

        Ok(Self { item })
    }
}
