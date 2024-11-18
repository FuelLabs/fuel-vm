use crate::{
    input::{
        coin::{
            CoinPredicate,
            CoinSigned,
        },
        message::{
            MessageCoinPredicate,
            MessageDataPredicate,
        },
    },
    policies::Policies,
    TxPointer,
};
use fuel_crypto::{
    Hasher,
    PublicKey,
};
use fuel_types::{
    canonical::{
        Deserialize,
        Error,
        Serialize,
    },
    Address,
    AssetId,
    BlobId,
    Bytes32,
    Nonce,
    Salt,
    Word,
};

use input::*;
use output::*;

#[cfg(feature = "typescript")]
use self::{
    input::typescript as input_ts,
    output::typescript as output_ts,
};

use alloc::vec::{
    IntoIter,
    Vec,
};
use itertools::Itertools;

mod fee;
mod metadata;
mod repr;
mod types;
mod validity;

mod id;

pub mod consensus_parameters;
pub mod policies;

pub use consensus_parameters::{
    ConsensusParameters,
    ContractParameters,
    DependentCost,
    FeeParameters,
    GasCosts,
    GasCostsValues,
    PredicateParameters,
    ScriptParameters,
    TxParameters,
};
pub use fee::{
    Chargeable,
    TransactionFee,
};
pub use metadata::Cacheable;
pub use repr::TransactionRepr;
pub use types::*;
pub use validity::{
    FormatValidityChecks,
    ValidityError,
};

#[cfg(feature = "alloc")]
pub use id::Signable;

pub use id::{
    PrepareSign,
    UniqueIdentifier,
};

/// Identification of transaction (also called transaction hash)
pub type TxId = Bytes32;

/// The fuel transaction entity <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/transaction.md>.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    strum_macros::EnumCount,
    serde::Serialize,
    serde::Deserialize,
)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[allow(clippy::large_enum_variant)]
pub enum Transaction {
    Script(Script),
    Create(Create),
    Mint(Mint),
    Upgrade(Upgrade),
    Upload(Upload),
    Blob(Blob),
}

#[cfg(feature = "test-helpers")]
impl Default for Transaction {
    fn default() -> Self {
        Script::default().into()
    }
}

impl Transaction {
    /// Return default valid transaction useful for tests.
    #[cfg(all(feature = "random", feature = "std", feature = "test-helpers"))]
    pub fn default_test_tx() -> Self {
        use crate::Finalizable;

        crate::TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(0)
            .add_fee_input()
            .finalize()
            .into()
    }

    pub const fn script(
        gas_limit: Word,
        script: Vec<u8>,
        script_data: Vec<u8>,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Script {
        let receipts_root = Bytes32::zeroed();

        Script {
            body: ScriptBody {
                script_gas_limit: gas_limit,
                receipts_root,
                script: ScriptCode { bytes: script },
                script_data,
            },
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn create(
        bytecode_witness_index: u16,
        policies: Policies,
        salt: Salt,
        mut storage_slots: Vec<StorageSlot>,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Create {
        // sort incoming storage slots
        storage_slots.sort();

        Create {
            body: CreateBody {
                bytecode_witness_index,
                salt,
                storage_slots,
            },
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn mint(
        tx_pointer: TxPointer,
        input_contract: input::contract::Contract,
        output_contract: output::contract::Contract,
        mint_amount: Word,
        mint_asset_id: AssetId,
        gas_price: Word,
    ) -> Mint {
        Mint {
            tx_pointer,
            input_contract,
            output_contract,
            mint_amount,
            mint_asset_id,
            gas_price,
            metadata: None,
        }
    }

    pub fn upgrade(
        upgrade_purpose: UpgradePurpose,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Upgrade {
        Upgrade {
            body: UpgradeBody {
                purpose: upgrade_purpose,
            },
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    /// Creates an `Upgrade` transaction with the purpose of upgrading the consensus
    /// parameters.
    pub fn upgrade_consensus_parameters(
        consensus_parameters: &ConsensusParameters,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        mut witnesses: Vec<Witness>,
    ) -> Result<Upgrade, ValidityError> {
        let serialized_consensus_parameters = postcard::to_allocvec(consensus_parameters)
            .map_err(|_| {
                ValidityError::TransactionUpgradeConsensusParametersSerialization
            })?;
        let checksum = Hasher::hash(&serialized_consensus_parameters);
        let witness_index = u16::try_from(witnesses.len())
            .map_err(|_| ValidityError::TransactionWitnessesMax)?;
        witnesses.push(serialized_consensus_parameters.into());

        Ok(Upgrade {
            body: UpgradeBody {
                purpose: UpgradePurpose::ConsensusParameters {
                    witness_index,
                    checksum,
                },
            },
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        })
    }

    pub fn upload(
        upload_body: UploadBody,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Upload {
        Upload {
            body: upload_body,
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn upload_from_subsection(
        subsection: UploadSubsection,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        mut witnesses: Vec<Witness>,
    ) -> Upload {
        let body = UploadBody {
            root: subsection.root,
            witness_index: u16::try_from(witnesses.len()).unwrap_or(u16::MAX),
            subsection_index: subsection.subsection_index,
            subsections_number: subsection.subsections_number,
            proof_set: subsection.proof_set,
        };
        witnesses.push(subsection.subsection.into());
        Upload {
            body,
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn blob(
        body: BlobBody,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        witnesses: Vec<Witness>,
    ) -> Blob {
        Blob {
            body,
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    pub fn blob_from_bytes(
        bytes: Vec<u8>,
        policies: Policies,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        mut witnesses: Vec<Witness>,
    ) -> Blob {
        let body = BlobBody {
            id: BlobId::compute(&bytes),
            witness_index: u16::try_from(witnesses.len()).unwrap_or(u16::MAX),
        };
        witnesses.push(bytes.into());
        Blob {
            body,
            policies,
            inputs,
            outputs,
            witnesses,
            metadata: None,
        }
    }

    /// Convert the type into a JSON string
    ///
    /// This is implemented as infallible because serde_json will fail only if the type
    /// can't serialize one of its attributes. We don't have such case with the
    /// transaction because all of its attributes are trivially serialized.
    ///
    /// If an error happens, a JSON string with the error description will be returned
    #[cfg(test)]
    pub fn to_json(&self) -> alloc::string::String {
        serde_json::to_string(self)
            .unwrap_or_else(|e| alloc::format!(r#"{{"error": "{e}"}}"#))
    }

    /// Attempt to deserialize a transaction from a JSON string, returning `None` if it
    /// fails
    #[cfg(test)]
    pub fn from_json<J>(json: J) -> Option<Self>
    where
        J: AsRef<str>,
    {
        // we opt to return `Option` to not leak serde concrete error implementations in
        // the crate. considering we don't expect to handle failures downstream
        // (e.g. if a string is not a valid json, then we simply don't have a
        // transaction out of that), then its not required to leak the type
        serde_json::from_str(json.as_ref()).ok()
    }

    pub const fn is_script(&self) -> bool {
        matches!(self, Self::Script { .. })
    }

    pub const fn is_create(&self) -> bool {
        matches!(self, Self::Create { .. })
    }

    pub const fn is_mint(&self) -> bool {
        matches!(self, Self::Mint { .. })
    }

    pub const fn is_upgrade(&self) -> bool {
        matches!(self, Self::Upgrade { .. })
    }

    pub const fn is_upload(&self) -> bool {
        matches!(self, Self::Upload { .. })
    }

    pub const fn is_blob(&self) -> bool {
        matches!(self, Self::Blob { .. })
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

    pub const fn as_mint(&self) -> Option<&Mint> {
        match self {
            Self::Mint(mint) => Some(mint),
            _ => None,
        }
    }

    pub fn as_mint_mut(&mut self) -> Option<&mut Mint> {
        match self {
            Self::Mint(mint) => Some(mint),
            _ => None,
        }
    }

    pub const fn as_upgrade(&self) -> Option<&Upgrade> {
        match self {
            Self::Upgrade(tx) => Some(tx),
            _ => None,
        }
    }

    pub fn as_upgrade_mut(&mut self) -> Option<&mut Upgrade> {
        match self {
            Self::Upgrade(tx) => Some(tx),
            _ => None,
        }
    }

    pub const fn as_upload(&self) -> Option<&Upload> {
        match self {
            Self::Upload(tx) => Some(tx),
            _ => None,
        }
    }

    pub fn as_upload_mut(&mut self) -> Option<&mut Upload> {
        match self {
            Self::Upload(tx) => Some(tx),
            _ => None,
        }
    }

    pub const fn as_blob(&self) -> Option<&Blob> {
        match self {
            Self::Blob(tx) => Some(tx),
            _ => None,
        }
    }

    pub fn as_blob_mut(&mut self) -> Option<&mut Blob> {
        match self {
            Self::Blob(tx) => Some(tx),
            _ => None,
        }
    }
}

pub trait Executable: field::Inputs + field::Outputs + field::Witnesses {
    /// Returns the assets' ids used in the inputs in the order of inputs.
    fn input_asset_ids<'a>(
        &'a self,
        base_asset_id: &'a AssetId,
    ) -> IntoIter<&'a AssetId> {
        self.inputs()
            .iter()
            .filter_map(|input| match input {
                Input::CoinPredicate(CoinPredicate { asset_id, .. })
                | Input::CoinSigned(CoinSigned { asset_id, .. }) => Some(asset_id),
                Input::MessageCoinSigned(_)
                | Input::MessageCoinPredicate(_)
                | Input::MessageDataPredicate(_)
                | Input::MessageDataSigned(_) => Some(base_asset_id),
                _ => None,
            })
            .collect_vec()
            .into_iter()
    }

    /// Returns unique assets' ids used in the inputs.
    fn input_asset_ids_unique<'a>(
        &'a self,
        base_asset_id: &'a AssetId,
    ) -> IntoIter<&'a AssetId> {
        let asset_ids = self.input_asset_ids(base_asset_id);

        #[cfg(feature = "std")]
        let asset_ids = asset_ids.unique();

        #[cfg(not(feature = "std"))]
        let asset_ids = asset_ids.sorted().dedup();

        asset_ids.collect_vec().into_iter()
    }

    /// Checks that all owners of inputs in the predicates are valid.
    fn check_predicate_owners(&self) -> bool {
        self.inputs()
            .iter()
            .filter_map(|i| match i {
                Input::CoinPredicate(CoinPredicate {
                    owner, predicate, ..
                }) => Some((owner, predicate)),
                Input::MessageDataPredicate(MessageDataPredicate {
                    recipient,
                    predicate,
                    ..
                }) => Some((recipient, predicate)),
                Input::MessageCoinPredicate(MessageCoinPredicate {
                    recipient,
                    predicate,
                    ..
                }) => Some((recipient, predicate)),
                _ => None,
            })
            .fold(true, |result, (owner, predicate)| {
                result && Input::is_predicate_owner_valid(owner, &**predicate)
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
        witness_index: u16,
    ) {
        let owner = Input::owner(owner);

        let input = Input::coin_signed(
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
        );
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
        nonce: Nonce,
        amount: Word,
        data: Vec<u8>,
        witness_index: u16,
    ) {
        let input = if data.is_empty() {
            Input::message_coin_signed(sender, recipient, amount, nonce, witness_index)
        } else {
            Input::message_data_signed(
                sender,
                recipient,
                amount,
                nonce,
                witness_index,
                data,
            )
        };

        self.inputs_mut().push(input);
    }
}

impl<T: field::Inputs + field::Outputs + field::Witnesses> Executable for T {}

impl From<Script> for Transaction {
    fn from(tx: Script) -> Self {
        Self::Script(tx)
    }
}

impl From<Create> for Transaction {
    fn from(tx: Create) -> Self {
        Self::Create(tx)
    }
}

impl From<Mint> for Transaction {
    fn from(tx: Mint) -> Self {
        Self::Mint(tx)
    }
}

impl From<Upgrade> for Transaction {
    fn from(tx: Upgrade) -> Self {
        Self::Upgrade(tx)
    }
}

impl From<Upload> for Transaction {
    fn from(tx: Upload) -> Self {
        Self::Upload(tx)
    }
}

impl From<Blob> for Transaction {
    fn from(tx: Blob) -> Self {
        Self::Blob(tx)
    }
}

impl Serialize for Transaction {
    fn size_static(&self) -> usize {
        match self {
            Self::Script(tx) => tx.size_static(),
            Self::Create(tx) => tx.size_static(),
            Self::Mint(tx) => tx.size_static(),
            Self::Upgrade(tx) => tx.size_static(),
            Self::Upload(tx) => tx.size_static(),
            Self::Blob(tx) => tx.size_static(),
        }
    }

    fn size_dynamic(&self) -> usize {
        match self {
            Self::Script(tx) => tx.size_dynamic(),
            Self::Create(tx) => tx.size_dynamic(),
            Self::Mint(tx) => tx.size_dynamic(),
            Self::Upgrade(tx) => tx.size_dynamic(),
            Self::Upload(tx) => tx.size_dynamic(),
            Self::Blob(tx) => tx.size_dynamic(),
        }
    }

    fn encode_static<O: fuel_types::canonical::Output + ?Sized>(
        &self,
        buffer: &mut O,
    ) -> Result<(), Error> {
        match self {
            Self::Script(tx) => tx.encode_static(buffer),
            Self::Create(tx) => tx.encode_static(buffer),
            Self::Mint(tx) => tx.encode_static(buffer),
            Self::Upgrade(tx) => tx.encode_static(buffer),
            Self::Upload(tx) => tx.encode_static(buffer),
            Self::Blob(tx) => tx.encode_static(buffer),
        }
    }

    fn encode_dynamic<O: fuel_types::canonical::Output + ?Sized>(
        &self,
        buffer: &mut O,
    ) -> Result<(), Error> {
        match self {
            Self::Script(tx) => tx.encode_dynamic(buffer),
            Self::Create(tx) => tx.encode_dynamic(buffer),
            Self::Mint(tx) => tx.encode_dynamic(buffer),
            Self::Upgrade(tx) => tx.encode_dynamic(buffer),
            Self::Upload(tx) => tx.encode_dynamic(buffer),
            Self::Blob(tx) => tx.encode_dynamic(buffer),
        }
    }
}

impl Deserialize for Transaction {
    fn decode_static<I: fuel_types::canonical::Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<Self, Error> {
        let mut discriminant_buffer = [0u8; 8];
        buffer.peek(&mut discriminant_buffer)?;

        let discriminant =
            <TransactionRepr as Deserialize>::decode(&mut &discriminant_buffer[..])?;

        match discriminant {
            TransactionRepr::Script => {
                Ok(<Script as Deserialize>::decode_static(buffer)?.into())
            }
            TransactionRepr::Create => {
                Ok(<Create as Deserialize>::decode_static(buffer)?.into())
            }
            TransactionRepr::Mint => {
                Ok(<Mint as Deserialize>::decode_static(buffer)?.into())
            }
            TransactionRepr::Upgrade => {
                Ok(<Upgrade as Deserialize>::decode_static(buffer)?.into())
            }
            TransactionRepr::Upload => {
                Ok(<Upload as Deserialize>::decode_static(buffer)?.into())
            }
            TransactionRepr::Blob => {
                Ok(<Blob as Deserialize>::decode_static(buffer)?.into())
            }
        }
    }

    fn decode_dynamic<I: fuel_types::canonical::Input + ?Sized>(
        &mut self,
        buffer: &mut I,
    ) -> Result<(), Error> {
        match self {
            Self::Script(tx) => tx.decode_dynamic(buffer),
            Self::Create(tx) => tx.decode_dynamic(buffer),
            Self::Mint(tx) => tx.decode_dynamic(buffer),
            Self::Upgrade(tx) => tx.decode_dynamic(buffer),
            Self::Upload(tx) => tx.decode_dynamic(buffer),
            Self::Blob(tx) => tx.decode_dynamic(buffer),
        }
    }
}

/// The module contains traits for each possible field in the `Transaction`. Those traits
/// can be used to write generic code based on the different combinations of the fields.
pub mod field {
    use crate::{
        input,
        output,
        policies,
        Input,
        Output,
        StorageSlot,
        UpgradePurpose as UpgradePurposeType,
        Witness,
    };
    use fuel_types::{
        AssetId,
        BlockHeight,
        Bytes32,
        Word,
    };

    use crate::policies::PolicyType;
    use alloc::vec::Vec;
    use core::ops::{
        Deref,
        DerefMut,
    };

    pub trait Tip {
        fn tip(&self) -> Word;
        fn set_tip(&mut self, value: Word);
    }

    impl<T: Policies + ?Sized> Tip for T {
        #[inline(always)]
        fn tip(&self) -> Word {
            self.policies().get(PolicyType::Tip).unwrap_or_default()
        }

        #[inline(always)]
        fn set_tip(&mut self, price: Word) {
            self.policies_mut().set(PolicyType::Tip, Some(price))
        }
    }

    pub trait WitnessLimit {
        fn witness_limit(&self) -> Word;
        fn set_witness_limit(&mut self, value: Word);
    }

    impl<T: Policies + ?Sized> WitnessLimit for T {
        #[inline(always)]
        fn witness_limit(&self) -> Word {
            self.policies().get(PolicyType::WitnessLimit).unwrap_or(0)
        }

        #[inline(always)]
        fn set_witness_limit(&mut self, value: Word) {
            self.policies_mut()
                .set(PolicyType::WitnessLimit, Some(value))
        }
    }

    pub trait ScriptGasLimit {
        fn script_gas_limit(&self) -> &Word;
        fn script_gas_limit_mut(&mut self) -> &mut Word;
        fn script_gas_limit_offset(&self) -> usize {
            Self::script_gas_limit_offset_static()
        }

        fn script_gas_limit_offset_static() -> usize;
    }

    pub trait Maturity {
        fn maturity(&self) -> BlockHeight;
        fn set_maturity(&mut self, value: BlockHeight);
    }

    impl<T: Policies + ?Sized> Maturity for T {
        #[inline(always)]
        fn maturity(&self) -> BlockHeight {
            self.policies()
                .get(PolicyType::Maturity)
                .map(|value| u32::try_from(value).unwrap_or(u32::MAX).into())
                .unwrap_or_default()
        }

        #[inline(always)]
        fn set_maturity(&mut self, block_height: BlockHeight) {
            self.policies_mut()
                .set(PolicyType::Maturity, Some(*block_height.deref() as u64))
        }
    }

    pub trait Expiration {
        fn expiration(&self) -> BlockHeight;
        fn set_expiration(&mut self, value: BlockHeight);
    }

    impl<T: Policies + ?Sized> Expiration for T {
        #[inline(always)]
        fn expiration(&self) -> BlockHeight {
            self.policies()
                .get(PolicyType::Expiration)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(u32::MAX)
                .into()
        }

        #[inline(always)]
        fn set_expiration(&mut self, block_height: BlockHeight) {
            self.policies_mut()
                .set(PolicyType::Expiration, Some(*block_height.deref() as u64))
        }
    }

    pub trait MaxFeeLimit {
        fn max_fee_limit(&self) -> Word;
        fn set_max_fee_limit(&mut self, value: Word);
    }

    impl<T: Policies + ?Sized> MaxFeeLimit for T {
        #[inline(always)]
        fn max_fee_limit(&self) -> Word {
            self.policies().get(PolicyType::MaxFee).unwrap_or(0)
        }

        #[inline(always)]
        fn set_max_fee_limit(&mut self, value: Word) {
            self.policies_mut().set(PolicyType::MaxFee, Some(value))
        }
    }

    pub trait Owner {
        fn owner(&self) -> Word;
        fn set_owner(&mut self, value: Word);
    }

    impl<T: Policies + ?Sized> Owner for T {
        #[inline(always)]
        fn owner(&self) -> Word {
            self.policies().get(PolicyType::Owner).unwrap_or(0)
        }

        #[inline(always)]
        fn set_owner(&mut self, value: Word) {
            self.policies_mut().set(PolicyType::Owner, Some(value))
        }
    }

    pub trait TxPointer {
        fn tx_pointer(&self) -> &crate::TxPointer;
        fn tx_pointer_mut(&mut self) -> &mut crate::TxPointer;
        fn tx_pointer_offset(&self) -> usize {
            Self::tx_pointer_static()
        }

        fn tx_pointer_static() -> usize;
    }

    pub trait InputContract {
        fn input_contract(&self) -> &input::contract::Contract;
        fn input_contract_mut(&mut self) -> &mut input::contract::Contract;
        fn input_contract_offset(&self) -> usize;
    }

    pub trait OutputContract {
        fn output_contract(&self) -> &output::contract::Contract;
        fn output_contract_mut(&mut self) -> &mut output::contract::Contract;
        fn output_contract_offset(&self) -> usize;
    }

    pub trait MintAmount {
        fn mint_amount(&self) -> &Word;
        fn mint_amount_mut(&mut self) -> &mut Word;
        fn mint_amount_offset(&self) -> usize;
    }

    pub trait MintAssetId {
        fn mint_asset_id(&self) -> &AssetId;
        fn mint_asset_id_mut(&mut self) -> &mut AssetId;
        fn mint_asset_id_offset(&self) -> usize;
    }

    pub trait MintGasPrice {
        fn gas_price(&self) -> &Word;
        fn gas_price_mut(&mut self) -> &mut Word;
        fn gas_price_offset(&self) -> usize;
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

    pub trait ChargeableBody<Body> {
        fn body(&self) -> &Body;
        fn body_mut(&mut self) -> &mut Body;
        fn body_offset_end(&self) -> usize;
    }

    pub trait Policies {
        fn policies(&self) -> &policies::Policies;
        fn policies_mut(&mut self) -> &mut policies::Policies;
        fn policies_offset(&self) -> usize;
    }

    pub trait BytecodeWitnessIndex {
        fn bytecode_witness_index(&self) -> &u16;
        fn bytecode_witness_index_mut(&mut self) -> &mut u16;
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
        fn storage_slots_mut(&mut self) -> StorageSlotRef;
        fn storage_slots_offset_static() -> usize;

        /// Returns the offset to the `StorageSlot` at `idx` index, if any.
        fn storage_slots_offset_at(&self, idx: usize) -> Option<usize>;
    }

    /// Reference object for mutating storage slots which will automatically
    /// sort the slots when dropped.
    pub struct StorageSlotRef<'a> {
        pub(crate) storage_slots: &'a mut Vec<StorageSlot>,
    }

    impl<'a> AsMut<Vec<StorageSlot>> for StorageSlotRef<'a> {
        fn as_mut(&mut self) -> &mut Vec<StorageSlot> {
            self.storage_slots
        }
    }

    impl<'a> Deref for StorageSlotRef<'a> {
        type Target = [StorageSlot];

        fn deref(&self) -> &Self::Target {
            self.storage_slots.deref()
        }
    }

    impl<'a> DerefMut for StorageSlotRef<'a> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.storage_slots.deref_mut()
        }
    }

    /// Ensure the storage slots are sorted after being set
    impl<'a> Drop for StorageSlotRef<'a> {
        fn drop(&mut self) {
            self.storage_slots.sort()
        }
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

    pub trait UpgradePurpose {
        fn upgrade_purpose(&self) -> &UpgradePurposeType;
        fn upgrade_purpose_mut(&mut self) -> &mut UpgradePurposeType;
        fn upgrade_purpose_offset(&self) -> usize {
            Self::upgrade_purpose_offset_static()
        }

        fn upgrade_purpose_offset_static() -> usize;
    }

    pub trait BytecodeRoot {
        fn bytecode_root(&self) -> &Bytes32;
        fn bytecode_root_mut(&mut self) -> &mut Bytes32;
        fn bytecode_root_offset(&self) -> usize {
            Self::bytecode_root_offset_static()
        }

        fn bytecode_root_offset_static() -> usize;
    }

    pub trait BlobId {
        fn blob_id(&self) -> &fuel_types::BlobId;
        fn blob_id_mut(&mut self) -> &mut fuel_types::BlobId;
        fn blob_id_offset(&self) -> usize {
            Self::blob_id_offset_static()
        }

        fn blob_id_offset_static() -> usize;
    }

    pub trait SubsectionIndex {
        fn subsection_index(&self) -> &u16;
        fn subsection_index_mut(&mut self) -> &mut u16;
        fn subsection_index_offset(&self) -> usize {
            Self::subsection_index_offset_static()
        }

        fn subsection_index_offset_static() -> usize;
    }

    pub trait SubsectionsNumber {
        fn subsections_number(&self) -> &u16;
        fn subsections_number_mut(&mut self) -> &mut u16;
        fn subsections_number_offset(&self) -> usize {
            Self::subsections_number_offset_static()
        }

        fn subsections_number_offset_static() -> usize;
    }

    pub trait ProofSet {
        fn proof_set(&self) -> &Vec<Bytes32>;
        fn proof_set_mut(&mut self) -> &mut Vec<Bytes32>;
        fn proof_set_offset(&self) -> usize {
            Self::proof_set_offset_static()
        }

        fn proof_set_offset_static() -> usize;
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use crate::{
        transaction::{
            input_ts::Input,
            output_ts::Output,
            Policies,
        },
        AssetId,
        Witness,
        Word,
    };
    use alloc::{
        boxed::Box,
        format,
        string::String,
        vec::Vec,
    };
    use fuel_types::Bytes32;

    #[derive(Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
    #[wasm_bindgen]
    pub struct Transaction(#[wasm_bindgen(skip)] pub Box<crate::Transaction>);

    #[derive(
        Default, Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize,
    )]
    #[wasm_bindgen]
    pub struct Create(#[wasm_bindgen(skip)] pub Box<crate::Create>);

    #[derive(
        Default, Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize,
    )]
    #[wasm_bindgen]
    pub struct Script(#[wasm_bindgen(skip)] pub Box<crate::Script>);

    #[derive(
        Default, Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize,
    )]
    #[wasm_bindgen]
    pub struct Mint(#[wasm_bindgen(skip)] pub Box<crate::Mint>);

    #[derive(
        Default, Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize,
    )]
    #[wasm_bindgen]
    pub struct Upgrade(#[wasm_bindgen(skip)] pub Box<crate::Upgrade>);

    #[derive(Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
    #[wasm_bindgen]
    pub struct UpgradePurpose(#[wasm_bindgen(skip)] pub Box<crate::UpgradePurpose>);

    #[derive(
        Default, Debug, Clone, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize,
    )]
    #[wasm_bindgen]
    pub struct Upload(#[wasm_bindgen(skip)] pub Box<crate::Upload>);

    #[wasm_bindgen]
    impl Transaction {
        #[wasm_bindgen(js_name = toJSON)]
        pub fn to_json(&self) -> String {
            serde_json::to_string(&self.0).expect("unable to json format")
        }

        #[wasm_bindgen(js_name = toString)]
        pub fn typescript_to_string(&self) -> String {
            format!("{:?}", self.0)
        }

        #[wasm_bindgen(js_name = to_bytes)]
        pub fn typescript_to_bytes(&self) -> Vec<u8> {
            use fuel_types::canonical::Serialize;
            self.0.to_bytes()
        }

        #[wasm_bindgen(js_name = from_bytes)]
        pub fn typescript_from_bytes(value: &[u8]) -> Result<Transaction, js_sys::Error> {
            use fuel_types::canonical::Deserialize;
            crate::Transaction::from_bytes(value)
                .map(|v| Transaction(Box::new(v)))
                .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))
        }

        #[wasm_bindgen]
        pub fn script(
            gas_limit: Word,
            script: Vec<u8>,
            script_data: Vec<u8>,
            policies: Policies,
            inputs: Vec<Input>,
            outputs: Vec<Output>,
            witnesses: Vec<Witness>,
        ) -> Script {
            Script(
                crate::Transaction::script(
                    gas_limit,
                    script,
                    script_data,
                    policies,
                    inputs.into_iter().map(|v| *v.0).collect(),
                    outputs.into_iter().map(|v| *v.0).collect(),
                    witnesses,
                )
                .into(),
            )
        }

        #[wasm_bindgen]
        pub fn create(
            bytecode_witness_index: u16,
            policies: Policies,
            salt: crate::Salt,
            storage_slots: Vec<crate::StorageSlot>,
            inputs: Vec<Input>,
            outputs: Vec<Output>,
            witnesses: Vec<Witness>,
        ) -> Create {
            Create(
                crate::Transaction::create(
                    bytecode_witness_index,
                    policies,
                    salt,
                    storage_slots,
                    inputs.into_iter().map(|v| *v.0).collect(),
                    outputs.into_iter().map(|v| *v.0).collect(),
                    witnesses,
                )
                .into(),
            )
        }

        #[wasm_bindgen]
        pub fn mint(
            tx_pointer: crate::TxPointer,
            input_contract: crate::input::contract::Contract,
            output_contract: crate::output::contract::Contract,
            mint_amount: Word,
            mint_asset_id: AssetId,
            gas_price: Word,
        ) -> Mint {
            Mint(
                crate::Mint {
                    tx_pointer,
                    input_contract,
                    output_contract,
                    mint_amount,
                    mint_asset_id,
                    gas_price,
                    metadata: None,
                }
                .into(),
            )
        }

        #[wasm_bindgen]
        pub fn upgrade(
            purpose: UpgradePurpose,
            policies: Policies,
            inputs: Vec<Input>,
            outputs: Vec<Output>,
            witnesses: Vec<Witness>,
        ) -> Upgrade {
            Upgrade(
                crate::Transaction::upgrade(
                    *purpose.0.as_ref(),
                    policies,
                    inputs.into_iter().map(|v| *v.0).collect(),
                    outputs.into_iter().map(|v| *v.0).collect(),
                    witnesses,
                )
                .into(),
            )
        }

        #[wasm_bindgen]
        pub fn upload(
            root: Bytes32,
            witness_index: u16,
            subsection_index: u16,
            subsections_number: u16,
            proof_set: Vec<Bytes32>,
            policies: Policies,
            inputs: Vec<Input>,
            outputs: Vec<Output>,
            witnesses: Vec<Witness>,
        ) -> Upload {
            Upload(
                crate::Transaction::upload(
                    crate::UploadBody {
                        root,
                        witness_index,
                        subsection_index,
                        subsections_number,
                        proof_set,
                    },
                    policies,
                    inputs.into_iter().map(|v| *v.0).collect(),
                    outputs.into_iter().map(|v| *v.0).collect(),
                    witnesses,
                )
                .into(),
            )
        }
    }

    macro_rules! ts_methods {
        ($t:ty, $tx:expr) => {
            #[wasm_bindgen]
            impl $t {
                #[wasm_bindgen(js_name = as_tx)]
                pub fn typescript_wrap_tx(self) -> Transaction {
                    Transaction(Box::new($tx(self.0.as_ref().clone())))
                }

                #[wasm_bindgen(constructor)]
                pub fn typescript_new() -> $t {
                    <$t>::default()
                }

                #[wasm_bindgen(js_name = toJSON)]
                pub fn to_json(&self) -> String {
                    serde_json::to_string(&self).expect("unable to json format")
                }

                #[wasm_bindgen(js_name = toString)]
                pub fn typescript_to_string(&self) -> String {
                    format!("{:?}", self)
                }

                #[wasm_bindgen(js_name = to_bytes)]
                pub fn typescript_to_bytes(&self) -> Vec<u8> {
                    use fuel_types::canonical::Serialize;
                    <_ as Serialize>::to_bytes(self.0.as_ref())
                }

                #[wasm_bindgen(js_name = from_bytes)]
                pub fn typescript_from_bytes(value: &[u8]) -> Result<$t, js_sys::Error> {
                    use fuel_types::canonical::Deserialize;
                    let res = <_ as Deserialize>::from_bytes(value)
                        .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))?;
                    Ok(Self(Box::new(res)))
                }
            }
        };
    }

    ts_methods!(Script, crate::Transaction::Script);
    ts_methods!(Create, crate::Transaction::Create);
    ts_methods!(Mint, crate::Transaction::Mint);
    ts_methods!(Upgrade, crate::Transaction::Upgrade);
    ts_methods!(Upload, crate::Transaction::Upload);
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script__metered_bytes_size___includes_witnesses() {
        let witness = [0u8; 64].to_vec();
        let script_with_no_witnesses = Transaction::script(
            Default::default(),
            vec![],
            vec![],
            Default::default(),
            vec![],
            vec![],
            vec![],
        );
        let script_with_witnesses = Transaction::script(
            Default::default(),
            vec![],
            vec![],
            Default::default(),
            vec![],
            vec![],
            vec![witness.clone().into()],
        );

        assert_eq!(
            script_with_witnesses.metered_bytes_size(),
            script_with_no_witnesses.metered_bytes_size() + witness.size()
        );
    }

    #[test]
    fn create__metered_bytes_size___includes_witnesses() {
        let witness = [0u8; 64].to_vec();
        let create_with_no_witnesses = Transaction::create(
            0,
            Default::default(),
            Default::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let create_with_witnesses = Transaction::create(
            0,
            Default::default(),
            Default::default(),
            vec![],
            vec![],
            vec![],
            vec![witness.clone().into()],
        );
        assert_eq!(
            create_with_witnesses.metered_bytes_size(),
            create_with_no_witnesses.metered_bytes_size() + witness.size()
        );
    }

    #[test]
    fn upgrade__metered_bytes_size___includes_witnesses() {
        let witness = [0u8; 64].to_vec();
        let tx_with_no_witnesses = Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: Default::default(),
            },
            Default::default(),
            vec![],
            vec![],
            vec![],
        );
        let tx_with_witnesses = Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: Default::default(),
            },
            Default::default(),
            vec![],
            vec![],
            vec![witness.clone().into()],
        );
        assert_eq!(
            tx_with_witnesses.metered_bytes_size(),
            tx_with_no_witnesses.metered_bytes_size() + witness.size()
        );
    }

    #[test]
    fn upload__metered_bytes_size__includes_witness() {
        let witness = [0u8; 64].to_vec();
        let tx_with_no_witnesses = Transaction::upload(
            Default::default(),
            Default::default(),
            vec![],
            vec![],
            vec![],
        );
        let tx_with_witnesses = Transaction::upload(
            Default::default(),
            Default::default(),
            vec![],
            vec![],
            vec![witness.clone().into()],
        );
        assert_eq!(
            tx_with_witnesses.metered_bytes_size(),
            tx_with_no_witnesses.metered_bytes_size() + witness.size()
        );
    }
}
