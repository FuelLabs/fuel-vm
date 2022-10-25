use fuel_crypto::PublicKey;
use fuel_types::bytes::SizedBytes;
use fuel_types::{Address, AssetId, Bytes32, Salt, Word};

use alloc::vec::{IntoIter, Vec};
use itertools::Itertools;

mod checkable;
mod fee;
mod metadata;
mod repr;
mod types;

#[cfg(feature = "std")]
mod id;

#[cfg(feature = "std")]
mod txio;

pub mod consensus_parameters;

pub use checkable::{CheckError, Checkable};
pub use consensus_parameters::ConsensusParameters;
pub use fee::{Chargeable, TransactionFee};
pub use metadata::Cacheable;
pub use repr::TransactionRepr;
pub use types::{
    Create, Input, InputRepr, Output, OutputRepr, Script, StorageSlot, TxPointer, UtxoId, Witness,
};

#[cfg(feature = "std")]
pub use id::{Signable, UniqueIdentifier};
#[cfg(feature = "std")]
pub use types::{CreateCheckedMetadata, ScriptCheckedMetadata};

/// Identification of transaction (also called transaction hash)
pub type TxId = Bytes32;

/// The fuel transaction entity https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_format.md#transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Transaction {
    Script(Script),
    Create(Create),
}

impl Default for Transaction {
    fn default() -> Self {
        Script::default().into()
    }
}

impl Transaction {
    pub const fn script(
        gas_price: Word,
        gas_limit: Word,
        maturity: Word,
        script: Vec<u8>,
        script_data: Vec<u8>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Script {
        let receipts_root = Bytes32::zeroed();

        Script {
            gas_price,
            gas_limit,
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
        maturity: Word,
        bytecode_witness_index: u8,
        salt: Salt,
        storage_slots: Vec<StorageSlot>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Create {
        // TODO consider split this function in two; one that will trust a provided bytecod len,
        // and other that will return a resulting, failing if the witness index isn't present
        let bytecode_length = witnesses
            .get(bytecode_witness_index as usize)
            .map(|witness| witness.as_ref().len() as Word / 4)
            .unwrap_or(0);

        Create {
            gas_price,
            gas_limit,
            maturity,
            bytecode_length,
            bytecode_witness_index,
            salt,
            storage_slots,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    /// Convert the type into a JSON string
    ///
    /// This is implemented as infallible because serde_json will fail only if the type can't
    /// serialize one of its attributes. We don't have such case with the transaction because all
    /// of its attributes are trivially serialized.
    ///
    /// If an error happens, a JSON string with the error description will be returned
    #[cfg(all(feature = "serde", feature = "alloc"))]
    pub fn to_json(&self) -> alloc::string::String {
        serde_json::to_string(self).unwrap_or_else(|e| alloc::format!(r#"{{"error": "{}"}}"#, e))
    }

    /// Attempt to deserialize a transaction from a JSON string, returning `None` if it fails
    #[cfg(all(feature = "serde", feature = "alloc"))]
    pub fn from_json<J>(json: J) -> Option<Self>
    where
        J: AsRef<str>,
    {
        // we opt to return `Option` to not leak serde concrete error implementations in the crate.
        // considering we don't expect to handle failures downstream (e.g. if a string is not a
        // valid json, then we simply don't have a transaction out of that), then its not required
        // to leak the type
        serde_json::from_str(json.as_ref()).ok()
    }

    pub const fn is_script(&self) -> bool {
        matches!(self, Self::Script { .. })
    }

    pub const fn is_create(&self) -> bool {
        matches!(self, Self::Create { .. })
    }

    pub const fn as_create(&self) -> Option<&Create> {
        match self {
            Self::Create(create) => Some(create),
            _ => None,
        }
    }

    pub fn as_create_mut(&mut self) -> Option<&mut Create> {
        match self {
            Self::Create(create) => Some(create),
            _ => None,
        }
    }

    pub const fn as_script(&self) -> Option<&Script> {
        match self {
            Self::Script(script) => Some(script),
            _ => None,
        }
    }

    pub fn as_script_mut(&mut self) -> Option<&mut Script> {
        match self {
            Self::Script(script) => Some(script),
            _ => None,
        }
    }
}

pub trait Executable: field::Inputs + field::Outputs + field::Witnesses {
    /// Returns the assets' ids used in the inputs in the order of inputs.
    fn input_asset_ids(&self) -> IntoIter<&AssetId> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::CoinPredicate { asset_id, .. } | Input::CoinSigned { asset_id, .. } => {
                    Some(asset_id)
                }
                Input::MessagePredicate { .. } | Input::MessageSigned { .. } => {
                    Some(&AssetId::BASE)
                }
                _ => None,
            })
            .collect_vec()
            .into_iter()
    }

    /// Returns unique assets' ids used in the inputs.
    fn input_asset_ids_unique(&self) -> IntoIter<&AssetId> {
        let asset_ids = self.input_asset_ids();

        #[cfg(feature = "std")]
        let asset_ids = asset_ids.unique();

        #[cfg(not(feature = "std"))]
        let asset_ids = asset_ids.sorted().dedup();

        asset_ids.collect_vec().into_iter()
    }

    /// Returns ids of all `Input::Contract` that are present in the inputs.
    // TODO: Return `Vec<input::Contract>` instead
    #[cfg(feature = "std")]
    fn input_contracts(&self) -> IntoIter<&fuel_types::ContractId> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::Contract { contract_id, .. } => Some(contract_id),
                _ => None,
            })
            .unique()
            .collect_vec()
            .into_iter()
    }

    /// Checks that all owners of inputs in the predicates are valid.
    #[cfg(feature = "std")]
    fn check_predicate_owners(&self) -> bool {
        self.inputs()
            .iter()
            .filter_map(|i| match i {
                Input::CoinPredicate {
                    owner, predicate, ..
                } => Some((owner, predicate)),
                Input::MessagePredicate {
                    recipient,
                    predicate,
                    ..
                } => Some((recipient, predicate)),
                _ => None,
            })
            .fold(true, |result, (owner, predicate)| {
                result && Input::is_predicate_owner_valid(owner, predicate)
            })
    }

    /// Append a new unsigned coin input to the transaction.
    ///
    /// When the transaction is constructed, [`Signable::sign_inputs`] should
    /// be called for every secret key used with this method.
    ///
    /// The production of the signatures can be done only after the full
    /// transaction skeleton is built because the input of the hash message
    /// is the ID of the final transaction.
    fn add_unsigned_coin_input(
        &mut self,
        utxo_id: UtxoId,
        owner: &PublicKey,
        amount: Word,
        asset_id: AssetId,
        tx_pointer: TxPointer,
        maturity: Word,
    ) {
        let owner = Input::owner(owner);

        let witness_index = self.witnesses().len() as u8;
        let input = Input::coin_signed(
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
            maturity,
        );

        self.witnesses_mut().push(Witness::default());
        self.inputs_mut().push(input);
    }

    /// Append a new unsigned message input to the transaction.
    ///
    /// When the transaction is constructed, [`Signable::sign_inputs`] should
    /// be called for every secret key used with this method.
    ///
    /// The production of the signatures can be done only after the full
    /// transaction skeleton is built because the input of the hash message
    /// is the ID of the final transaction.
    fn add_unsigned_message_input(
        &mut self,
        sender: Address,
        recipient: Address,
        nonce: Word,
        amount: Word,
        data: Vec<u8>,
    ) {
        let message_id = Input::compute_message_id(&sender, &recipient, nonce, amount, &data);

        let witness_index = self.witnesses().len() as u8;
        let input = Input::message_signed(
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            witness_index,
            data,
        );

        self.witnesses_mut().push(Witness::default());
        self.inputs_mut().push(input);
    }

    /// Prepare the transaction for VM initialization for script execution
    ///
    /// note: Fields dependent on storage/state such as balance and state roots, or tx pointers,
    /// should already set by the client beforehand.
    #[cfg(feature = "std")]
    fn prepare_init_script(&mut self) -> &mut Self {
        self.outputs_mut()
            .iter_mut()
            .for_each(|o| o.prepare_init_script());

        self
    }

    /// Prepare the transaction for VM initialization for predicate verification
    fn prepare_init_predicate(&mut self) -> &mut Self {
        self.inputs_mut()
            .iter_mut()
            .for_each(|i| i.prepare_init_predicate());

        self.outputs_mut()
            .iter_mut()
            .for_each(|o| o.prepare_init_predicate());

        self
    }
}

impl<T: field::Inputs + field::Outputs + field::Witnesses> Executable for T {}

impl SizedBytes for Transaction {
    fn serialized_size(&self) -> usize {
        match self {
            Self::Script(script) => script.serialized_size(),
            Self::Create(create) => create.serialized_size(),
        }
    }
}

impl From<Script> for Transaction {
    fn from(script: Script) -> Self {
        Transaction::Script(script)
    }
}

impl From<Create> for Transaction {
    fn from(create: Create) -> Self {
        Transaction::Create(create)
    }
}

/// The module contains traits for each possible field in the `Transaction`. Those traits can be
/// used to write generic code based on the different combinations of the fields.
pub mod field {
    use crate::{Input, Output, StorageSlot, Witness};
    use fuel_types::{Bytes32, Word};

    use alloc::vec::Vec;

    pub trait GasPrice {
        fn gas_price(&self) -> &Word;
        fn gas_price_mut(&mut self) -> &mut Word;
        fn gas_price_offset(&self) -> usize {
            Self::gas_price_offset_static()
        }

        fn gas_price_offset_static() -> usize;
    }

    pub trait GasLimit {
        fn gas_limit(&self) -> &Word;
        fn gas_limit_mut(&mut self) -> &mut Word;
        fn gas_limit_offset(&self) -> usize {
            Self::gas_limit_offset_static()
        }

        fn gas_limit_offset_static() -> usize;
    }

    pub trait Maturity {
        fn maturity(&self) -> &Word;
        fn maturity_mut(&mut self) -> &mut Word;
        fn maturity_offset(&self) -> usize {
            Self::maturity_offset_static()
        }

        fn maturity_offset_static() -> usize;
    }

    pub trait ReceiptsRoot {
        fn receipts_root(&self) -> &Bytes32;
        fn receipts_root_mut(&mut self) -> &mut Bytes32;
        fn receipts_root_offset(&self) -> usize {
            Self::receipts_root_offset_static()
        }

        fn receipts_root_offset_static() -> usize;
    }

    pub trait Script {
        fn script(&self) -> &Vec<u8>;
        fn script_mut(&mut self) -> &mut Vec<u8>;
        fn script_offset(&self) -> usize {
            Self::script_offset_static()
        }

        fn script_offset_static() -> usize;
    }

    pub trait ScriptData {
        fn script_data(&self) -> &Vec<u8>;
        fn script_data_mut(&mut self) -> &mut Vec<u8>;
        fn script_data_offset(&self) -> usize;
    }

    pub trait BytecodeLength {
        fn bytecode_length(&self) -> &Word;
        fn bytecode_length_mut(&mut self) -> &mut Word;
        fn bytecode_length_offset(&self) -> usize {
            Self::bytecode_length_offset_static()
        }

        fn bytecode_length_offset_static() -> usize;
    }

    pub trait BytecodeWitnessIndex {
        fn bytecode_witness_index(&self) -> &u8;
        fn bytecode_witness_index_mut(&mut self) -> &mut u8;
        fn bytecode_witness_index_offset(&self) -> usize {
            Self::bytecode_witness_index_offset_static()
        }

        fn bytecode_witness_index_offset_static() -> usize;
    }

    pub trait Salt {
        fn salt(&self) -> &fuel_types::Salt;
        fn salt_mut(&mut self) -> &mut fuel_types::Salt;
        fn salt_offset(&self) -> usize {
            Self::salt_offset_static()
        }

        fn salt_offset_static() -> usize;
    }

    pub trait StorageSlots {
        fn storage_slots(&self) -> &Vec<StorageSlot>;
        fn storage_slots_mut(&mut self) -> &mut Vec<StorageSlot>;
        fn storage_slots_offset(&self) -> usize {
            Self::storage_slots_offset_static()
        }

        fn storage_slots_offset_static() -> usize;

        /// Returns the offset to the `StorageSlot` at `idx` index, if any.
        fn storage_slots_offset_at(&self, idx: usize) -> Option<usize>;
    }

    pub trait Inputs {
        fn inputs(&self) -> &Vec<Input>;
        fn inputs_mut(&mut self) -> &mut Vec<Input>;
        fn inputs_offset(&self) -> usize;

        /// Returns the offset to the `Input` at `idx` index, if any.
        fn inputs_offset_at(&self, idx: usize) -> Option<usize>;

        /// Returns predicate's offset and length of the `Input` at `idx`, if any.
        fn inputs_predicate_offset_at(&self, idx: usize) -> Option<(usize, usize)>;
    }

    pub trait Outputs {
        fn outputs(&self) -> &Vec<Output>;
        fn outputs_mut(&mut self) -> &mut Vec<Output>;
        fn outputs_offset(&self) -> usize;

        /// Returns the offset to the `Output` at `idx` index, if any.
        fn outputs_offset_at(&self, idx: usize) -> Option<usize>;
    }

    pub trait Witnesses {
        fn witnesses(&self) -> &Vec<Witness>;
        fn witnesses_mut(&mut self) -> &mut Vec<Witness>;
        fn witnesses_offset(&self) -> usize;

        /// Returns the offset to the `Witness` at `idx` index, if any.
        fn witnesses_offset_at(&self, idx: usize) -> Option<usize>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metered_data_excludes_witnesses() {
        // test script
        let script_with_no_witnesses =
            Transaction::script(0, 0, 0, vec![], vec![], vec![], vec![], vec![]);
        let script_with_witnesses = Transaction::script(
            0,
            0,
            0,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![[0u8; 64].to_vec().into()],
        );

        assert_eq!(
            script_with_witnesses.metered_bytes_size(),
            script_with_no_witnesses.metered_bytes_size()
        );
        // test create
        let create_with_no_witnesses = Transaction::create(
            0,
            0,
            0,
            0,
            Default::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let create_with_witnesses = Transaction::create(
            0,
            0,
            0,
            0,
            Default::default(),
            vec![],
            vec![],
            vec![],
            vec![[0u8; 64].to_vec().into()],
        );
        assert_eq!(
            create_with_witnesses.metered_bytes_size(),
            create_with_no_witnesses.metered_bytes_size()
        );
    }
}
