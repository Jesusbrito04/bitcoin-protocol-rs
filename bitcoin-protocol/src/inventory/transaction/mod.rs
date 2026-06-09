use std::fmt::Display;

use crate::{decode_compact_size, encode_compact_size, P2PError, Serialize};

#[derive(Debug)]
pub struct Transaction {
    version: u32,
    marker_flag: Option<(u8, u8)>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    witnesses: Option<Vec<Witness>>,
    locktime: u32,
}

impl Transaction {
    pub fn from_hex(raw_tx: String) -> Result<Transaction, P2PError> {
        let raw_bytes = hex::decode(raw_tx).map_err(|e| P2PError::Parse(e.to_string()))?;
        Transaction::deserialize(&mut raw_bytes.as_slice())
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version = self.version;
        let marker_flag = self.marker_flag.is_some();
        let inputs = &self.inputs;
        let outputs = &self.outputs;

        let locktime = self.locktime;
        write!(f, "\n")?;
        writeln!(f, "{{")?;
        write!(f, "   \"version\": \"{}\",\n", version)?;
        if marker_flag {
            let (marker, flag) = self.marker_flag.unwrap();
            write!(f, "   \"marker\": \"{}\",\n", marker)?;
            write!(f, "   \"flag\": \"{}\",\n", flag)?;
        };

        write!(f, "   \"inputCount\": \"{}\",\n", inputs.len())?;
        write!(f, "   \"inputs\": [")?;
        for (i, input) in inputs.iter().enumerate() {
            write!(f, "{{ {} }}", input)?;
            if i < inputs.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]\n")?;
        write!(f, "   \"outputsCount\": \"{}\",\n", outputs.len())?;
        write!(f, "   \"outputs\": [")?;
        for (i, output) in outputs.iter().enumerate() {
            write!(f, "{{ {} }}", output)?;
            if i < outputs.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]\n")?;
        if let Some(witnesses) = self.witnesses.as_ref() {
            write!(f, "   \"witnessesCount\": \"{}\",\n", witnesses.len())?;
            for (i, witness) in witnesses.iter().enumerate() {
                write!(f, "{}", witness)?;
                if i < witnesses.len() - 1 {
                    write!(f, ", ")?;
                }
            }
            write!(f, " ]\n")?;
        }
        write!(f, "   \"locktime\": \"{}\"\n", locktime)?;
        writeln!(f, "}}")
    }
}

impl Serialize for Transaction {
    type Value = Self;
    fn serialize(&self) -> Vec<u8> {
        let segwit = self.marker_flag.is_some();
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.version.to_le_bytes());

        if segwit {
            let (marker, flag) = self
                .marker_flag
                .ok_or("Error getting market flag")
                .expect("Error with market flag");
            buffer.push(marker);
            buffer.push(flag);
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

        if segwit {
            if let Some(witnesses) = &self.witnesses {
                for witness_items in witnesses {
                    let cmpct_size_witnesses =
                        encode_compact_size(witness_items.witness.len() as usize);
                    buffer.extend_from_slice(&cmpct_size_witnesses);
                    for witness in &witness_items.witness {
                        buffer.extend_from_slice(&witness.serialize());
                    }
                }
            }
        }

        buffer.extend_from_slice(&self.locktime.to_le_bytes());

        buffer
    }
    fn deserialize(bytes: &mut &[u8]) -> Result<Self::Value, P2PError> {
        let (version, rest) = bytes.split_at(4);
        let version = u32::from_le_bytes(version.try_into()?);
        *bytes = rest;
        let mut marker_flag = None;
        if bytes[0] == 0x00 {
            let (marker, rest) = bytes.split_at(2);
            let (m, f) = marker
                .split_first()
                .ok_or(P2PError::NotEnoughBytesToSplit)?;
            marker_flag = Some((*m, f[0]));
            *bytes = rest;
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

        let mut witnesses: Option<Vec<Witness>> = None;

        if marker_flag.is_some() {
            let mut all_witnesses = Vec::new();
            for _ in 0..inputs.len() {
                let mut witness = Witness {
                    witness: Vec::new(),
                };
                let witness_count = decode_compact_size(bytes)?;
                for _ in 0..witness_count {
                    let witnessitem = WitnessStack::deserialize(bytes)?;
                    witness.witness.push(witnessitem);
                }
                all_witnesses.push(witness)
            }
            witnesses = Some(all_witnesses);
        }

        let (locktime, rest) = bytes.split_at(4);
        let locktime = u32::from_le_bytes(locktime.try_into()?);

        *bytes = rest;

        Ok(Self {
            version,
            marker_flag,
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

impl Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let txid = hex::encode(self.txid);
        let script_sig = hex::encode(&self.script_sig);
        write!(f, "txid: {}, ", txid)?;
        write!(f, "vout: {}, ", self.vout)?;
        write!(f, "scriptsigsize: {:x}, ", script_sig.len())?;
        if script_sig.len() == 0 {
            write!(f, "scriptsig: [], ")?;
        } else {
            write!(f, "scriptsig: {}, ", script_sig)?;
        }
        write!(f, "sequence: {} ", self.sequence)
    }
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

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let script_pubkey = hex::encode(&self.script_pubkey);
        let script_pubkey_size = self.script_pubkey.len();
        write!(f, "amount: {} ", self.amount)?;
        write!(f, "scriptPubKeySize: {:x} ", script_pubkey_size)?;
        write!(f, "scriptPubKey: {} ", script_pubkey)
    }
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
pub struct Witness {
    pub witness: Vec<WitnessStack>,
}
#[derive(Debug, Clone)]
pub struct WitnessStack {
    pub item: Vec<u8>,
}

impl Display for Witness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let witness = &self.witness;
        write!(f, "   \"witness\": [ ")?;
        for (i, witness_stack) in witness.iter().enumerate() {
            write!(f, "{}", witness_stack)?;
            if i < witness.len() - 1 {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

impl Display for WitnessStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let witness = hex::encode(&self.item);
        let item_len = self.item.len();
        write!(f, "\"size\": {:x}, ", item_len)?;
        write!(f, "\"item\": {}", witness)
    }
}

impl Serialize for WitnessStack {
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
