use fuel_asm::Opcode;
use fuel_types::{Bytes32, Color, ContractId, Salt, Word};
use itertools::Itertools;

use fuel_types::bytes::SizedBytes;
use std::convert::TryFrom;
use std::io::Write;
use std::{io, mem};

mod id;
mod metadata;
mod offset;
mod txio;
mod types;
mod validation;

pub use metadata::Metadata;
pub use types::{Input, Output, UtxoId, Witness};
pub use validation::ValidationError;

const WORD_SIZE: usize = mem::size_of::<Word>();

const TRANSACTION_SCRIPT_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Byte price
    + WORD_SIZE // Maturity
    + WORD_SIZE // Script size
    + WORD_SIZE // Script data size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE // Witnesses size
    + Bytes32::LEN; // Receipts root

const TRANSACTION_CREATE_FIXED_SIZE: usize = WORD_SIZE // Identifier
    + WORD_SIZE // Gas price
    + WORD_SIZE // Gas limit
    + WORD_SIZE // Byte price
    + WORD_SIZE // Maturity
    + WORD_SIZE // Bytecode size
    + WORD_SIZE // Bytecode witness index
    + WORD_SIZE // Static contracts size
    + WORD_SIZE // Inputs size
    + WORD_SIZE // Outputs size
    + WORD_SIZE // Witnesses size
    + Salt::LEN; // Salt

/// Identification of transaction (also called transaction hash)
pub type TxId = Bytes32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TransactionRepr {
    Script = 0x00,
    Create = 0x01,
}

impl TryFrom<Word> for TransactionRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Script),
            0x01 => Ok(Self::Create),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Transaction {
    Script {
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        maturity: Word,
        receipts_root: Bytes32,
        script: Vec<u8>,
        script_data: Vec<u8>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
        metadata: Option<Metadata>,
    },

    Create {
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        maturity: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        static_contracts: Vec<ContractId>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
        metadata: Option<Metadata>,
    },
}

impl Default for Transaction {
    fn default() -> Self {
        // Create a valid transaction with a single return instruction
        //
        // The Return op is mandatory for the execution of any context
        let script = Opcode::RET(0x10).to_bytes().to_vec();

        Transaction::script(0, 1000000, 0, 0, script, vec![], vec![], vec![], vec![])
    }
}

impl Transaction {
    pub const fn script(
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        maturity: Word,
        script: Vec<u8>,
        script_data: Vec<u8>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Self {
        let receipts_root = Bytes32::zeroed();

        Self::Script {
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            receipts_root,
            script,
            script_data,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub const fn create(
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        maturity: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        static_contracts: Vec<ContractId>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Self {
        Self::Create {
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            bytecode_witness_index,
            salt,
            static_contracts,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn input_colors(&self) -> impl Iterator<Item = &Color> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::Coin { color, .. } => Some(color),
                _ => None,
            })
            .unique()
    }

    pub fn input_contracts(&self) -> impl Iterator<Item = &ContractId> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::Contract { contract_id, .. } => Some(contract_id),
                _ => None,
            })
            .unique()
    }

    pub const fn gas_price(&self) -> Word {
        match self {
            Self::Script { gas_price, .. } => *gas_price,
            Self::Create { gas_price, .. } => *gas_price,
        }
    }

    pub fn set_gas_price(&mut self, price: Word) {
        match self {
            Self::Script { gas_price, .. } => *gas_price = price,
            Self::Create { gas_price, .. } => *gas_price = price,
        }
    }

    pub const fn gas_limit(&self) -> Word {
        match self {
            Self::Script { gas_limit, .. } => *gas_limit,
            Self::Create { gas_limit, .. } => *gas_limit,
        }
    }

    pub fn set_gas_limit(&mut self, limit: Word) {
        match self {
            Self::Script { gas_limit, .. } => *gas_limit = limit,
            Self::Create { gas_limit, .. } => *gas_limit = limit,
        }
    }

    pub const fn byte_price(&self) -> Word {
        match self {
            Self::Script { byte_price, .. } => *byte_price,
            Self::Create { byte_price, .. } => *byte_price,
        }
    }

    pub fn set_byte_price(&mut self, price: Word) {
        match self {
            Self::Script { byte_price, .. } => *byte_price = price,
            Self::Create { byte_price, .. } => *byte_price = price,
        }
    }

    pub const fn maturity(&self) -> Word {
        match self {
            Self::Script { maturity, .. } => *maturity,
            Self::Create { maturity, .. } => *maturity,
        }
    }

    pub fn set_maturity(&mut self, mat: Word) {
        match self {
            Self::Script { maturity, .. } => *maturity = mat,
            Self::Create { maturity, .. } => *maturity = mat,
        }
    }

    pub const fn is_script(&self) -> bool {
        matches!(self, Self::Script { .. })
    }

    pub const fn metadata(&self) -> Option<&Metadata> {
        match self {
            Self::Script { metadata, .. } => metadata.as_ref(),
            Self::Create { metadata, .. } => metadata.as_ref(),
        }
    }

    pub fn inputs(&self) -> &[Input] {
        match self {
            Self::Script { inputs, .. } => inputs.as_slice(),
            Self::Create { inputs, .. } => inputs.as_slice(),
        }
    }

    pub fn outputs(&self) -> &[Output] {
        match self {
            Self::Script { outputs, .. } => outputs.as_slice(),
            Self::Create { outputs, .. } => outputs.as_slice(),
        }
    }

    pub fn witnesses(&self) -> &[Witness] {
        match self {
            Self::Script { witnesses, .. } => witnesses.as_slice(),
            Self::Create { witnesses, .. } => witnesses.as_slice(),
        }
    }

    pub fn set_witnesses(&mut self, new_witnesses: Vec<Witness>) {
        match self {
            Self::Script { witnesses, .. } => *witnesses = new_witnesses,
            Self::Create { witnesses, .. } => *witnesses = new_witnesses,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> io::Result<(usize, Self)> {
        let mut tx = Self::default();

        let n = tx.write(bytes)?;

        Ok((n, tx))
    }

    pub const fn receipts_root(&self) -> Option<&Bytes32> {
        match self {
            Self::Script { receipts_root, .. } => Some(receipts_root),
            _ => None,
        }
    }

    pub fn set_receipts_root(&mut self, root: Bytes32) -> Option<Bytes32> {
        match self {
            Self::Script { receipts_root, .. } => Some(std::mem::replace(receipts_root, root)),

            _ => None,
        }
    }

    /// Used for accounting purposes when charging byte based fees
    pub fn metered_bytes_size(&self) -> usize {
        // Just use the default serialized size for now until
        // the compressed representation for accounting purposes
        // is defined. Witness data should still be excluded.
        let witness_data = self
            .witnesses()
            .iter()
            .map(|w| w.serialized_size())
            .sum::<usize>();

        self.serialized_size() - witness_data // Witness data size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metered_data_excludes_witnesses() {
        // test script
        let script_with_no_witnesses = Transaction::Script {
            gas_price: 0,
            gas_limit: 0,
            byte_price: 0,
            maturity: 0,
            receipts_root: Default::default(),
            script: vec![],
            script_data: vec![],
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![],
            metadata: None,
        };
        let script_with_witnesses = Transaction::Script {
            gas_price: 0,
            gas_limit: 0,
            byte_price: 0,
            maturity: 0,
            receipts_root: Default::default(),
            script: vec![],
            script_data: vec![],
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![[0u8; 64].to_vec().into()],
            metadata: None,
        };

        assert_eq!(
            script_with_witnesses.metered_bytes_size(),
            script_with_no_witnesses.metered_bytes_size()
        );
        // test create
        let create_with_no_witnesses = Transaction::Create {
            gas_price: 0,
            gas_limit: 0,
            byte_price: 0,
            maturity: 0,
            bytecode_witness_index: 0,
            salt: Default::default(),
            static_contracts: vec![],
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![],
            metadata: None,
        };
        let create_with_witnesses = Transaction::Create {
            gas_price: 0,
            gas_limit: 0,
            byte_price: 0,
            maturity: 0,
            bytecode_witness_index: 0,
            salt: Default::default(),
            static_contracts: vec![],
            inputs: vec![],
            outputs: vec![],
            witnesses: vec![[0u8; 64].to_vec().into()],
            metadata: None,
        };
        assert_eq!(
            create_with_witnesses.metered_bytes_size(),
            create_with_no_witnesses.metered_bytes_size()
        );
    }
}
