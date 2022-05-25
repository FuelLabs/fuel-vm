use crate::consts::*;

use fuel_asm::Opcode;
use fuel_types::{AssetId, Bytes32, ContractId, Salt, Word};

#[cfg(feature = "std")]
use fuel_types::bytes::SizedBytes;

#[cfg(feature = "std")]
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};

use alloc::vec::Vec;
use core::mem;

mod internals;
mod metadata;
mod offset;
mod repr;
mod types;
mod validation;

#[cfg(feature = "std")]
mod id;

#[cfg(feature = "std")]
mod txio;

pub mod consensus_parameters;

pub use metadata::Metadata;
pub use repr::TransactionRepr;
pub use types::{Input, Output, StorageSlot, UtxoId, Witness};
pub use validation::ValidationError;

/// Identification of transaction (also called transaction hash)
pub type TxId = Bytes32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        bytecode_length: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        static_contracts: Vec<ContractId>,
        storage_slots: Vec<StorageSlot>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
        metadata: Option<Metadata>,
    },
}

impl Default for Transaction {
    fn default() -> Self {
        use alloc::vec;

        // Create a valid transaction with a single return instruction
        //
        // The Return op is mandatory for the execution of any context
        let script = Opcode::RET(0x10).to_bytes().to_vec();

        Transaction::script(0, 1, 0, 0, script, vec![], vec![], vec![], vec![])
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

    pub fn create(
        gas_price: Word,
        gas_limit: Word,
        byte_price: Word,
        maturity: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        static_contracts: Vec<ContractId>,
        storage_slots: Vec<StorageSlot>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Self {
        // TODO consider split this function in two; one that will trust a provided bytecod len,
        // and other that will return a resulting, failing if the witness index isn't present
        let bytecode_length = witnesses
            .get(bytecode_witness_index as usize)
            .map(|witness| witness.as_ref().len() as Word / 4)
            .unwrap_or(0);

        Self::Create {
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            bytecode_length,
            bytecode_witness_index,
            salt,
            static_contracts,
            storage_slots,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn input_asset_ids(&self) -> impl Iterator<Item = &AssetId> {
        self.inputs().iter().filter_map(|input| match input {
            Input::CoinPredicate { asset_id, .. } | Input::CoinSigned { asset_id, .. } => {
                Some(asset_id)
            }
            _ => None,
        })
    }

    pub fn input_asset_ids_unique(&self) -> impl Iterator<Item = &AssetId> {
        use itertools::Itertools;

        let asset_ids = self.input_asset_ids();

        #[cfg(feature = "std")]
        let asset_ids = asset_ids.unique();

        #[cfg(not(feature = "std"))]
        let asset_ids = asset_ids.sorted().dedup();

        asset_ids
    }

    #[cfg(feature = "std")]
    pub fn input_contracts(&self) -> impl Iterator<Item = &ContractId> {
        use itertools::Itertools;

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

    pub const fn receipts_root(&self) -> Option<&Bytes32> {
        match self {
            Self::Script { receipts_root, .. } => Some(receipts_root),
            _ => None,
        }
    }

    pub fn set_receipts_root(&mut self, root: Bytes32) -> Option<Bytes32> {
        match self {
            Self::Script { receipts_root, .. } => Some(mem::replace(receipts_root, root)),

            _ => None,
        }
    }

    /// Append a new unsigned input to the transaction.
    ///
    /// When the transaction is constructed, [`Transaction::sign_inputs`] should
    /// be called for every secret key used with this method.
    ///
    /// The production of the signatures can be done only after the full
    /// transaction skeleton is built because the input of the hash message
    /// is the ID of the final transaction.
    #[cfg(feature = "std")]
    pub fn add_unsigned_coin_input(
        &mut self,
        utxo_id: UtxoId,
        owner: &PublicKey,
        amount: Word,
        asset_id: AssetId,
        maturity: Word,
    ) {
        let owner = Input::coin_owner(owner);

        let witness_index = self.witnesses().len() as u8;
        let input = Input::coin_signed(utxo_id, owner, amount, asset_id, witness_index, maturity);

        self._add_witness(Witness::default());
        self._add_input(input);
    }

    /// For all inputs of type `coin`, check if its `owner` equals the public
    /// counterpart of the provided key. Sign all matches.
    #[cfg(feature = "std")]
    pub fn sign_inputs(&mut self, secret: &SecretKey) {
        use itertools::Itertools;

        let pk = PublicKey::from(secret);
        let pk = Input::coin_owner(&pk);
        let id = self.id();

        // Safety: checked length
        let message = unsafe { Message::as_ref_unchecked(id.as_ref()) };

        let signature = Signature::sign(secret, message);

        let (inputs, witnesses) = match self {
            Self::Script {
                inputs, witnesses, ..
            } => (inputs, witnesses),
            Self::Create {
                inputs, witnesses, ..
            } => (inputs, witnesses),
        };

        inputs
            .iter()
            .filter_map(|input| match input {
                Input::CoinSigned {
                    owner,
                    witness_index,
                    ..
                } if owner == &pk => Some(*witness_index as usize),
                _ => None,
            })
            .dedup()
            .for_each(|w| {
                if let Some(w) = witnesses.get_mut(w) {
                    *w = signature.as_ref().into();
                }
            });
    }

    /// Used for accounting purposes when charging byte based fees
    #[cfg(feature = "std")]
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
            bytecode_length: 0,
            bytecode_witness_index: 0,
            salt: Default::default(),
            static_contracts: vec![],
            storage_slots: vec![],
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
            bytecode_length: 0,
            bytecode_witness_index: 0,
            salt: Default::default(),
            static_contracts: vec![],
            storage_slots: vec![],
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
