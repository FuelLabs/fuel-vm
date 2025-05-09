//! A checked transaction is type-wrapper for transactions which have been checked.
//! It is impossible to construct a checked transaction without performing necessary
//! checks.
//!
//! This allows the VM to accept transactions with metadata that have been already
//! verified upstream.

#![allow(non_upper_case_globals)]

use fuel_tx::{
    Create,
    Mint,
    Script,
    Transaction,
    ValidityError,
    field::Expiration,
};
use fuel_types::{
    BlockHeight,
    ChainId,
};

use alloc::{
    boxed::Box,
    vec::Vec,
};
use core::{
    borrow::Borrow,
    fmt::Debug,
    future::Future,
};
use fuel_tx::{
    ConsensusParameters,
    field::{
        Inputs,
        MaxFeeLimit,
    },
};

mod balances;
#[cfg(feature = "test-helpers")]
pub mod builder;
pub mod types;

pub use types::*;

use crate::{
    error::PredicateVerificationFailed,
    interpreter::{
        Memory,
        MemoryInstance,
    },
    pool::VmMemoryPool,
    prelude::*,
    storage::predicate::{
        EmptyStorage,
        PredicateStorageProvider,
        PredicateStorageRequirements,
    },
};

bitflags::bitflags! {
    /// Possible types of transaction checks.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Checks: u32 {
        /// Basic checks defined in the specification for each transaction:
        /// https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/transaction.md#transaction
        /// Also ensures that malleable fields are zeroed.
        const Basic         = 0b00000001;
        /// Check that signature in the transactions are valid.
        const Signatures    = 0b00000010;
        /// Check that predicate in the transactions are valid.
        const Predicates    = 0b00000100;
    }
}

impl core::fmt::Display for Checks {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:032b}", self.bits())
    }
}

/// The type describes that the inner transaction was already checked.
///
/// All fields are private, and there is no constructor, so it is impossible to create the
/// instance of `Checked` outside the `fuel-tx` crate.
///
/// The inner data is immutable to prevent modification to invalidate the checking.
///
/// If you need to modify an inner state, you need to get inner values
/// (via the `Into<(Tx, Tx ::Metadata)>` trait), modify them and check again.
///
/// # Dev note: Avoid serde serialization of this type.
///
/// Since checked tx would need to be re-validated on deserialization anyways,
/// it's cleaner to redo the tx check.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Checked<Tx: IntoChecked> {
    transaction: Tx,
    metadata: Tx::Metadata,
    checks_bitmask: Checks,
}

impl<Tx: IntoChecked> Checked<Tx> {
    fn new(transaction: Tx, metadata: Tx::Metadata, checks_bitmask: Checks) -> Self {
        Checked {
            transaction,
            metadata,
            checks_bitmask,
        }
    }

    pub(crate) fn basic(transaction: Tx, metadata: Tx::Metadata) -> Self {
        Checked::new(transaction, metadata, Checks::Basic)
    }

    /// Returns reference on inner transaction.
    pub fn transaction(&self) -> &Tx {
        &self.transaction
    }

    /// Returns the metadata generated during the check for transaction.
    pub fn metadata(&self) -> &Tx::Metadata {
        &self.metadata
    }

    /// Returns the bitmask of all passed checks.
    pub fn checks(&self) -> &Checks {
        &self.checks_bitmask
    }

    /// Performs check of signatures, if not yet done.
    pub fn check_signatures(mut self, chain_id: &ChainId) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Signatures) {
            self.transaction.check_signatures(chain_id)?;
            self.checks_bitmask.insert(Checks::Signatures);
        }
        Ok(self)
    }
}

/// Transaction that has checks for all dynamic values, e.g. `gas_price`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Ready<Tx: IntoChecked> {
    gas_price: Word,
    transaction: Tx,
    metadata: Tx::Metadata,
    checks_bitmask: Checks,
}

impl<Tx: IntoChecked> Ready<Tx> {
    /// Consume and decompose components of the `Immutable` transaction.
    pub fn decompose(self) -> (Word, Checked<Tx>) {
        let Ready {
            gas_price,
            transaction,
            metadata,
            checks_bitmask,
        } = self;
        let checked = Checked::new(transaction, metadata, checks_bitmask);
        (gas_price, checked)
    }

    /// Getter for `gas_price` field
    pub fn gas_price(&self) -> Word {
        self.gas_price
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoChecked> Checked<Tx> {
    /// Convert `Checked` into `Ready` without performing final checks.
    pub fn test_into_ready(self) -> Ready<Tx> {
        let Checked {
            transaction,
            metadata,
            checks_bitmask,
        } = self;
        Ready {
            gas_price: 0,
            transaction,
            metadata,
            checks_bitmask,
        }
    }
}

impl<Tx: IntoChecked + Chargeable> Checked<Tx> {
    /// Run final checks on `Checked` using dynamic values, e.g. `gas_price`
    pub fn into_ready(
        self,
        gas_price: Word,
        gas_costs: &GasCosts,
        fee_parameters: &FeeParameters,
        block_height: Option<BlockHeight>,
    ) -> Result<Ready<Tx>, CheckError> {
        let Checked {
            transaction,
            metadata,
            checks_bitmask,
        } = self;
        let fee = TransactionFee::checked_from_tx(
            gas_costs,
            fee_parameters,
            &transaction,
            gas_price,
        )
        .ok_or(CheckError::Validity(ValidityError::BalanceOverflow))?;

        let max_fee_from_policies = transaction.max_fee_limit();
        let max_fee_from_gas_price = fee.max_fee();

        if let Some(block_height) = block_height {
            if block_height > transaction.expiration() {
                return Err(CheckError::Validity(ValidityError::TransactionExpiration));
            }
        }

        if max_fee_from_gas_price > max_fee_from_policies {
            Err(CheckError::InsufficientMaxFee {
                max_fee_from_policies,
                max_fee_from_gas_price,
            })
        } else {
            Ok(Ready {
                gas_price,
                transaction,
                metadata,
                checks_bitmask,
            })
        }
    }
}

impl<Tx: IntoChecked + UniqueIdentifier> Checked<Tx> {
    /// Returns the transaction ID from the computed metadata
    pub fn id(&self) -> TxId {
        self.transaction
            .cached_id()
            .expect("Transaction metadata should be computed for checked transactions")
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoChecked + Default> Default for Checked<Tx>
where
    Checked<Tx>: CheckPredicates,
{
    fn default() -> Self {
        Tx::default()
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect("default tx should produce a valid fully checked transaction")
    }
}

impl<Tx: IntoChecked> From<Checked<Tx>> for (Tx, Tx::Metadata) {
    fn from(checked: Checked<Tx>) -> Self {
        let Checked {
            transaction,
            metadata,
            ..
        } = checked;

        (transaction, metadata)
    }
}

impl<Tx: IntoChecked> AsRef<Tx> for Checked<Tx> {
    fn as_ref(&self) -> &Tx {
        &self.transaction
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoChecked> AsMut<Tx> for Checked<Tx> {
    fn as_mut(&mut self) -> &mut Tx {
        &mut self.transaction
    }
}

impl<Tx: IntoChecked> Borrow<Tx> for Checked<Tx> {
    fn borrow(&self) -> &Tx {
        self.transaction()
    }
}

/// The error can occur when transforming transactions into the `Checked` type.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CheckError {
    /// The transaction doesn't pass validity rules.
    Validity(ValidityError),
    /// The predicate verification failed.
    PredicateVerificationFailed(PredicateVerificationFailed),
    /// The max fee used during checking was lower than calculated during `Immutable`
    /// conversion
    InsufficientMaxFee {
        /// The max fee from the policies defined by the user.
        max_fee_from_policies: Word,
        /// The max fee calculated from the gas price and gas used by the transaction.
        max_fee_from_gas_price: Word,
    },
}

/// Performs checks for a transaction
pub trait IntoChecked: FormatValidityChecks + Sized {
    /// Metadata produced during the check.
    type Metadata: Sized;

    /// Returns transaction that passed all `Checks`.
    fn into_checked(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<Checked<Self>, CheckError>
    where
        Checked<Self>: CheckPredicates,
    {
        self.into_checked_reusable_memory(
            block_height,
            consensus_params,
            MemoryInstance::new(),
            &EmptyStorage,
        )
    }

    /// Returns transaction that passed all `Checks` accepting reusable memory
    /// to run predicates.
    fn into_checked_reusable_memory(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<Checked<Self>, CheckError>
    where
        Checked<Self>: CheckPredicates,
    {
        let check_predicate_params = consensus_params.into();
        self.into_checked_basic(block_height, consensus_params)?
            .check_signatures(&consensus_params.chain_id())?
            .check_predicates(&check_predicate_params, memory, storage)
    }

    /// Returns transaction that passed only `Checks::Basic`.
    fn into_checked_basic(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<Checked<Self>, CheckError>;
}

/// The parameters needed for checking a predicate
#[derive(Debug, Clone)]
pub struct CheckPredicateParams {
    /// Gas costs for opcodes
    pub gas_costs: GasCosts,
    /// Chain ID
    pub chain_id: ChainId,
    /// Maximum gas per predicate
    pub max_gas_per_predicate: u64,
    /// Maximum gas per transaction
    pub max_gas_per_tx: u64,
    /// Maximum number of inputs
    pub max_inputs: u16,
    /// Maximum size of the contract in bytes
    pub contract_max_size: u64,
    /// Maximum length of the message data
    pub max_message_data_length: u64,
    /// Offset of the transaction data in the memory
    pub tx_offset: usize,
    /// Fee parameters
    pub fee_params: FeeParameters,
    /// Base Asset ID
    pub base_asset_id: AssetId,
}

#[cfg(feature = "test-helpers")]
impl Default for CheckPredicateParams {
    fn default() -> Self {
        CheckPredicateParams::from(&ConsensusParameters::standard())
    }
}

impl From<ConsensusParameters> for CheckPredicateParams {
    fn from(value: ConsensusParameters) -> Self {
        CheckPredicateParams::from(&value)
    }
}

impl From<&ConsensusParameters> for CheckPredicateParams {
    fn from(value: &ConsensusParameters) -> Self {
        CheckPredicateParams {
            gas_costs: value.gas_costs().clone(),
            chain_id: value.chain_id(),
            max_gas_per_predicate: value.predicate_params().max_gas_per_predicate(),
            max_gas_per_tx: value.tx_params().max_gas_per_tx(),
            max_inputs: value.tx_params().max_inputs(),
            contract_max_size: value.contract_params().contract_max_size(),
            max_message_data_length: value.predicate_params().max_message_data_length(),
            tx_offset: value.tx_params().tx_offset(),
            fee_params: *(value.fee_params()),
            base_asset_id: *value.base_asset_id(),
        }
    }
}

/// Provides predicate verification functionality for the transaction.
#[async_trait::async_trait]
pub trait CheckPredicates: Sized {
    /// Performs predicates verification of the transaction.
    fn check_predicates(
        self,
        params: &CheckPredicateParams,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<Self, CheckError>;

    /// Performs predicates verification of the transaction in parallel.
    async fn check_predicates_async<E: ParallelExecutor>(
        self,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
    ) -> Result<Self, CheckError>;
}

/// Provides predicate estimation functionality for the transaction.
#[async_trait::async_trait]
pub trait EstimatePredicates: Sized {
    /// Estimates predicates of the transaction.
    fn estimate_predicates(
        &mut self,
        params: &CheckPredicateParams,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<(), CheckError>;

    /// Estimates predicates of the transaction in parallel.
    async fn estimate_predicates_async<E: ParallelExecutor>(
        &mut self,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
    ) -> Result<(), CheckError>;
}

/// Executes CPU-heavy tasks in parallel.
#[async_trait::async_trait]
pub trait ParallelExecutor {
    /// Future created from a CPU-heavy task.
    type Task: Future + Send + 'static;

    /// Creates a Future from a CPU-heavy task.
    fn create_task<F>(func: F) -> Self::Task
    where
        F: FnOnce() -> (usize, Result<Word, PredicateVerificationFailed>)
            + Send
            + 'static;

    /// Executes tasks created by `create_task` in parallel.
    async fn execute_tasks(
        futures: Vec<Self::Task>,
    ) -> Vec<(usize, Result<Word, PredicateVerificationFailed>)>;
}

#[async_trait::async_trait]
impl<Tx> CheckPredicates for Checked<Tx>
where
    Tx: ExecutableTransaction + Send + Sync + 'static,
    <Tx as IntoChecked>::Metadata: crate::interpreter::CheckedMetadata + Send + Sync,
    Tx: Inputs<MyInput = Input>,
{
    fn check_predicates(
        mut self,
        params: &CheckPredicateParams,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Predicates) {
            predicates::check_predicates(&self, params, memory, storage)?;
            self.checks_bitmask.insert(Checks::Predicates);
        }
        Ok(self)
    }

    async fn check_predicates_async<E>(
        mut self,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
    ) -> Result<Self, CheckError>
    where
        E: ParallelExecutor,
    {
        if !self.checks_bitmask.contains(Checks::Predicates) {
            predicates::check_predicates_async::<Tx, E>(&self, params, pool, storage)
                .await?;

            self.checks_bitmask.insert(Checks::Predicates);

            Ok(self)
        } else {
            Ok(self)
        }
    }
}

#[async_trait::async_trait]
impl<Tx> EstimatePredicates for Tx
where
    Tx: ExecutableTransaction + Send + Sync + 'static,
    Tx: Inputs<MyInput = Input>,
{
    fn estimate_predicates(
        &mut self,
        params: &CheckPredicateParams,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<(), CheckError> {
        predicates::estimate_predicates(self, params, memory, storage)?;
        Ok(())
    }

    async fn estimate_predicates_async<E>(
        &mut self,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
    ) -> Result<(), CheckError>
    where
        E: ParallelExecutor,
    {
        predicates::estimate_predicates_async::<Self, E>(self, params, pool, storage)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EstimatePredicates for Transaction {
    fn estimate_predicates(
        &mut self,
        params: &CheckPredicateParams,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<(), CheckError> {
        match self {
            Self::Script(tx) => tx.estimate_predicates(params, memory, storage),
            Self::Create(tx) => tx.estimate_predicates(params, memory, storage),
            Self::Mint(_) => Ok(()),
            Self::Upgrade(tx) => tx.estimate_predicates(params, memory, storage),
            Self::Upload(tx) => tx.estimate_predicates(params, memory, storage),
            Self::Blob(tx) => tx.estimate_predicates(params, memory, storage),
            Transaction::ScriptV2(_) => {
                todo!()
            }
        }
    }

    async fn estimate_predicates_async<E: ParallelExecutor>(
        &mut self,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
    ) -> Result<(), CheckError> {
        match self {
            Self::Script(tx) => {
                tx.estimate_predicates_async::<E>(params, pool, storage)
                    .await
            }
            Self::Create(tx) => {
                tx.estimate_predicates_async::<E>(params, pool, storage)
                    .await
            }
            Self::Mint(_) => Ok(()),
            Self::Upgrade(tx) => {
                tx.estimate_predicates_async::<E>(params, pool, storage)
                    .await
            }
            Self::Upload(tx) => {
                tx.estimate_predicates_async::<E>(params, pool, storage)
                    .await
            }
            Self::Blob(tx) => {
                tx.estimate_predicates_async::<E>(params, pool, storage)
                    .await
            }
            Transaction::ScriptV2(_) => {
                todo!()
            }
        }
    }
}

#[async_trait::async_trait]
impl CheckPredicates for Checked<Mint> {
    fn check_predicates(
        mut self,
        _params: &CheckPredicateParams,
        _memory: impl Memory,
        _storage: &impl PredicateStorageRequirements,
    ) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Predicates);
        Ok(self)
    }

    async fn check_predicates_async<E: ParallelExecutor>(
        mut self,
        _params: &CheckPredicateParams,
        _pool: &impl VmMemoryPool,
        _storage: &impl PredicateStorageProvider,
    ) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Predicates);
        Ok(self)
    }
}

#[async_trait::async_trait]
impl CheckPredicates for Checked<Transaction> {
    fn check_predicates(
        self,
        params: &CheckPredicateParams,
        memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
    ) -> Result<Self, CheckError> {
        let checked_transaction: CheckedTransaction = self.into();
        let checked_transaction: CheckedTransaction = match checked_transaction {
            CheckedTransaction::Script(tx) => {
                CheckPredicates::check_predicates(tx, params, memory, storage)?.into()
            }
            CheckedTransaction::Create(tx) => {
                CheckPredicates::check_predicates(tx, params, memory, storage)?.into()
            }
            CheckedTransaction::Mint(tx) => {
                CheckPredicates::check_predicates(tx, params, memory, storage)?.into()
            }
            CheckedTransaction::Upgrade(tx) => {
                CheckPredicates::check_predicates(tx, params, memory, storage)?.into()
            }
            CheckedTransaction::Upload(tx) => {
                CheckPredicates::check_predicates(tx, params, memory, storage)?.into()
            }
            CheckedTransaction::Blob(tx) => {
                CheckPredicates::check_predicates(tx, params, memory, storage)?.into()
            }
        };
        Ok(checked_transaction.into())
    }

    async fn check_predicates_async<E>(
        mut self,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
    ) -> Result<Self, CheckError>
    where
        E: ParallelExecutor,
    {
        let checked_transaction: CheckedTransaction = self.into();

        let checked_transaction: CheckedTransaction = match checked_transaction {
            CheckedTransaction::Script(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params, pool, storage)
                    .await?
                    .into()
            }
            CheckedTransaction::Create(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params, pool, storage)
                    .await?
                    .into()
            }
            CheckedTransaction::Mint(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params, pool, storage)
                    .await?
                    .into()
            }
            CheckedTransaction::Upgrade(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params, pool, storage)
                    .await?
                    .into()
            }
            CheckedTransaction::Upload(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params, pool, storage)
                    .await?
                    .into()
            }
            CheckedTransaction::Blob(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params, pool, storage)
                    .await?
                    .into()
            }
        };

        Ok(checked_transaction.into())
    }
}

/// The Enum version of `Checked<Transaction>` allows getting the inner variant without
/// losing "checked" status.
///
/// It is possible to freely convert `Checked<Transaction>` into `CheckedTransaction` and
/// vice verse without the overhead.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum CheckedTransaction {
    Script(Checked<Script>),
    Create(Checked<Create>),
    Mint(Checked<Mint>),
    Upgrade(Checked<Upgrade>),
    Upload(Checked<Upload>),
    Blob(Checked<Blob>),
}

impl From<Checked<Transaction>> for CheckedTransaction {
    fn from(checked: Checked<Transaction>) -> Self {
        let Checked {
            transaction,
            metadata,
            checks_bitmask,
        } = checked;

        // # Dev note: Avoid wildcard pattern to be sure that all variants are covered.
        match (transaction, metadata) {
            (Transaction::Script(transaction), CheckedMetadata::Script(metadata)) => {
                Self::Script(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Create(transaction), CheckedMetadata::Create(metadata)) => {
                Self::Create(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Mint(transaction), CheckedMetadata::Mint(metadata)) => {
                Self::Mint(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Upgrade(transaction), CheckedMetadata::Upgrade(metadata)) => {
                Self::Upgrade(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Upload(transaction), CheckedMetadata::Upload(metadata)) => {
                Self::Upload(Checked::new(transaction, metadata, checks_bitmask))
            }
            (Transaction::Blob(transaction), CheckedMetadata::Blob(metadata)) => {
                Self::Blob(Checked::new(transaction, metadata, checks_bitmask))
            }
            // The code should produce the `CheckedMetadata` for the corresponding
            // transaction variant. It is done in the implementation of the
            // `IntoChecked` trait for `Transaction`. With the current
            // implementation, the patterns below are unreachable.
            (Transaction::Script(_), _) => unreachable!(),
            (Transaction::Create(_), _) => unreachable!(),
            (Transaction::Mint(_), _) => unreachable!(),
            (Transaction::Upgrade(_), _) => unreachable!(),
            (Transaction::Upload(_), _) => unreachable!(),
            (Transaction::Blob(_), _) => unreachable!(),
            (Transaction::ScriptV2(_), _) => {
                todo!()
            }
        }
    }
}

impl From<Checked<Script>> for CheckedTransaction {
    fn from(checked: Checked<Script>) -> Self {
        Self::Script(checked)
    }
}

impl From<Checked<Create>> for CheckedTransaction {
    fn from(checked: Checked<Create>) -> Self {
        Self::Create(checked)
    }
}

impl From<Checked<Mint>> for CheckedTransaction {
    fn from(checked: Checked<Mint>) -> Self {
        Self::Mint(checked)
    }
}

impl From<Checked<Upgrade>> for CheckedTransaction {
    fn from(checked: Checked<Upgrade>) -> Self {
        Self::Upgrade(checked)
    }
}

impl From<Checked<Upload>> for CheckedTransaction {
    fn from(checked: Checked<Upload>) -> Self {
        Self::Upload(checked)
    }
}

impl From<Checked<Blob>> for CheckedTransaction {
    fn from(checked: Checked<Blob>) -> Self {
        Self::Blob(checked)
    }
}

impl From<CheckedTransaction> for Checked<Transaction> {
    fn from(checked: CheckedTransaction) -> Self {
        match checked {
            CheckedTransaction::Script(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Create(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Mint(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Upgrade(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Upload(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
            CheckedTransaction::Blob(Checked {
                transaction,
                metadata,
                checks_bitmask,
            }) => Checked::new(transaction.into(), metadata.into(), checks_bitmask),
        }
    }
}

/// The `IntoChecked` metadata for `CheckedTransaction`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum CheckedMetadata {
    Script(<Script as IntoChecked>::Metadata),
    Create(<Create as IntoChecked>::Metadata),
    Mint(<Mint as IntoChecked>::Metadata),
    Upgrade(<Upgrade as IntoChecked>::Metadata),
    Upload(<Upload as IntoChecked>::Metadata),
    Blob(<Blob as IntoChecked>::Metadata),
}

impl From<<Script as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Script as IntoChecked>::Metadata) -> Self {
        Self::Script(metadata)
    }
}

impl From<<Create as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Create as IntoChecked>::Metadata) -> Self {
        Self::Create(metadata)
    }
}

impl From<<Mint as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Mint as IntoChecked>::Metadata) -> Self {
        Self::Mint(metadata)
    }
}

impl From<<Upgrade as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Upgrade as IntoChecked>::Metadata) -> Self {
        Self::Upgrade(metadata)
    }
}

impl From<<Upload as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Upload as IntoChecked>::Metadata) -> Self {
        Self::Upload(metadata)
    }
}
impl From<<Blob as IntoChecked>::Metadata> for CheckedMetadata {
    fn from(metadata: <Blob as IntoChecked>::Metadata) -> Self {
        Self::Blob(metadata)
    }
}

impl IntoChecked for Transaction {
    type Metadata = CheckedMetadata;

    fn into_checked_basic(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<Checked<Self>, CheckError> {
        match self {
            Self::Script(tx) => {
                let (transaction, metadata) = tx
                    .into_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Self::Create(tx) => {
                let (transaction, metadata) = tx
                    .into_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Self::Mint(tx) => {
                let (transaction, metadata) = tx
                    .into_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Self::Upgrade(tx) => {
                let (transaction, metadata) = tx
                    .into_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Self::Upload(tx) => {
                let (transaction, metadata) = tx
                    .into_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Self::Blob(tx) => {
                let (transaction, metadata) = tx
                    .into_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Transaction::ScriptV2(_) => {
                todo!()
            }
        }
        .map(|(transaction, metadata)| Checked::basic(transaction, metadata))
    }
}

impl From<ValidityError> for CheckError {
    fn from(value: ValidityError) -> Self {
        CheckError::Validity(value)
    }
}

impl From<PredicateVerificationFailed> for CheckError {
    fn from(value: PredicateVerificationFailed) -> Self {
        CheckError::PredicateVerificationFailed(value)
    }
}

#[cfg(feature = "random")]
#[allow(non_snake_case)]
#[allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]
#[cfg(test)]
mod tests {

    use super::*;
    use alloc::vec;
    use fuel_asm::op;
    use fuel_crypto::SecretKey;
    use fuel_tx::{
        Script,
        TransactionBuilder,
        ValidityError,
        field::{
            ScriptGasLimit,
            Tip,
            WitnessLimit,
            Witnesses,
        },
    };
    use fuel_types::canonical::Serialize;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::{
        Rng,
        SeedableRng,
        rngs::StdRng,
    };

    fn params(factor: u64) -> ConsensusParameters {
        ConsensusParameters::new(
            TxParameters::default(),
            PredicateParameters::default(),
            ScriptParameters::default(),
            ContractParameters::default(),
            FeeParameters::default().with_gas_price_factor(factor),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    #[test]
    fn into_checked__tx_accepts_valid_tx() {
        // simple smoke test that valid txs can be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_limit = 1000;
        let input_amount = 1000;
        let output_amount = 10;
        let max_fee_limit = 500;
        let tx =
            valid_coin_tx(rng, gas_limit, input_amount, output_amount, max_fee_limit);

        let checked = tx
            .clone()
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect("Expected valid transaction");

        // verify transaction getter works
        assert_eq!(checked.transaction(), &tx);
        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - max_fee_limit - output_amount
        );
    }

    #[test]
    fn into_checked__tx_accepts_valid_signed_message_coin_for_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 1000;
        let gas_limit = 1000;
        let zero_fee_limit = 500;
        let tx = signed_message_coin_tx(rng, gas_limit, input_amount, zero_fee_limit);

        let checked = tx
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.transaction.max_fee_limit()
        );
    }

    #[test]
    fn into_checked__tx_excludes_message_output_amount_from_fee() {
        // ensure message outputs aren't deducted from available balance
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_limit = 1000;
        let zero_fee_limit = 50;
        let tx = signed_message_coin_tx(rng, gas_limit, input_amount, zero_fee_limit);

        let checked = tx
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.transaction.max_fee_limit()
        );
    }

    #[test]
    fn into_checked__message_data_signed_message_is_not_used_to_cover_fees() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        // given
        let input_amount = 100;

        // when
        let max_fee = input_amount;
        let tx = TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(max_fee)
            // Add message input with enough to cover max fee
            .add_unsigned_message_input(SecretKey::random(rng), rng.r#gen(), rng.r#gen(), input_amount, vec![0xff; 10])
            // Add empty base coin
            .add_unsigned_coin_input(SecretKey::random(rng), rng.r#gen(), 0, AssetId::BASE, rng.r#gen())
            .finalize();

        let err = tx
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect_err("Expected valid transaction");

        // then
        assert!(matches!(
            err,
            CheckError::Validity(ValidityError::InsufficientFeeAmount {
                expected: _,
                provided: 0
            })
        ));
    }

    #[test]
    fn message_data_predicate_message_is_not_used_to_cover_fees() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_limit = 1000;

        // given
        let input_amount = 100;

        // when
        let max_fee = input_amount;

        let tx = TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(max_fee)
            .script_gas_limit(gas_limit)
            .add_input(Input::message_data_predicate(
                rng.r#gen(),
                rng.r#gen(),
                input_amount,
                rng.r#gen(),
                Default::default(),
                vec![0xff; 10],
                vec![0xaa; 10],
                vec![0xbb; 10],
            ))
            // Add empty base coin
            .add_unsigned_coin_input(SecretKey::random(rng), rng.r#gen(), 0, AssetId::BASE, rng.r#gen())
            .finalize();

        let err = tx
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect_err("Expected valid transaction");

        // then
        assert!(matches!(
            err,
            CheckError::Validity(ValidityError::InsufficientFeeAmount {
                expected: _,
                provided: 0
            })
        ));
    }

    // use quickcheck to fuzz any rounding or precision errors in the max fee w/ coin
    // input
    #[quickcheck]
    fn max_fee_coin_input(
        gas_price: u64,
        gas_limit: u64,
        witness_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify max fee a transaction can consume based on gas limit + bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let predicate_gas_used = rng.r#gen();
        let tx = predicate_tx(
            rng,
            gas_limit,
            witness_limit,
            input_amount,
            predicate_gas_used,
        );

        if let Ok(valid) = is_valid_max_fee(&tx, gas_price, &gas_costs, &fee_params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the min fee w/ coin
    // input
    #[quickcheck]
    fn min_fee_coin_input(
        gas_price: u64,
        gas_limit: u64,
        witness_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify min fee a transaction can consume based on bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }
        let rng = &mut StdRng::seed_from_u64(seed);
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let predicate_gas_used = rng.r#gen();
        let tx = predicate_tx(
            rng,
            gas_limit,
            witness_limit,
            input_amount,
            predicate_gas_used,
        );

        if let Ok(valid) = is_valid_max_fee(&tx, gas_price, &gas_costs, &fee_params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the max fee w/ message
    // input
    #[quickcheck]
    fn max_fee_message_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        tip: u64,
        seed: u64,
    ) -> TestResult {
        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_coin_tx(rng, gas_limit, input_amount, tip);

        if let Ok(valid) = is_valid_max_fee(&tx, gas_price, &gas_costs, &fee_params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in refund calculation
    #[quickcheck]
    fn refund_when_used_gas_is_zero(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
        tip: u64,
    ) -> TestResult {
        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_coin_tx(rng, gas_limit, input_amount, tip);

        // Given
        let used_gas = 0;

        // When
        let refund = tx.refund_fee(&gas_costs, &fee_params, used_gas, gas_price);

        let min_fee = tx.min_fee(&gas_costs, &fee_params, gas_price);
        let max_fee = tx.max_fee(&gas_costs, &fee_params, gas_price);

        // Then
        if let Some(refund) = refund {
            TestResult::from_bool(max_fee - min_fee == refund as u128)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the min fee w/ message
    // input
    #[quickcheck]
    fn min_fee_message_input(
        gas_limit: u64,
        input_amount: u64,
        gas_price: u64,
        gas_price_factor: u64,
        tip: u64,
        seed: u64,
    ) -> TestResult {
        // verify min fee a transaction can consume based on bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard();
        }
        let rng = &mut StdRng::seed_from_u64(seed);
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_coin_tx(rng, gas_limit, input_amount, tip);

        if let Ok(valid) = is_valid_min_fee(&tx, &gas_costs, &fee_params, gas_price) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    #[test]
    fn fee_multiple_signed_inputs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 100;
        let gas_limit = 1000;
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(1);
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            // Set up 3 signed inputs
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64
            * fee_params.gas_per_byte()
            + gas_costs.vm_initialization().resolve(tx.size() as u64)
            + 3 * gas_costs.eck1()
            + gas_costs.s256().resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = expected_min_fee + gas_limit * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    #[test]
    fn fee_multiple_signed_inputs_single_owner() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 100;
        let gas_limit = 1000;
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(1);
        let secret = SecretKey::random(rng);
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            // Set up 3 signed inputs
            .add_unsigned_message_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                secret,
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        // Because all inputs are owned by the same address, the address will only need to
        // be recovered once. Therefore, we charge only once for the address
        // recovery of the signed inputs.
        let expected_min_fee = (tx.metered_bytes_size() as u64
            * fee_params.gas_per_byte()
            + gas_costs.vm_initialization().resolve(tx.size() as u64)
            + gas_costs.eck1()
            + gas_costs.s256().resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee + gas_limit * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    fn random_bytes<const N: usize, R: Rng + ?Sized>(rng: &mut R) -> Box<[u8; N]> {
        let mut bytes = Box::new([0u8; N]);
        for chunk in bytes.chunks_mut(32) {
            rng.fill(chunk);
        }
        bytes
    }

    #[test]
    fn min_fee_multiple_predicate_inputs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 100;
        let gas_limit = 1000;
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(1);
        let predicate_1 = random_bytes::<1024, _>(rng);
        let predicate_2 = random_bytes::<2048, _>(rng);
        let predicate_3 = random_bytes::<4096, _>(rng);
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            // Set up 3 predicate inputs
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                50,
                predicate_1.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                100,
                predicate_2.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                200,
                predicate_3.to_vec(),
                vec![],
            ))
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.size() as u64 * fee_params.gas_per_byte()
            + gas_costs.vm_initialization().resolve(tx.size() as u64)
            + gas_costs.contract_root().resolve(predicate_1.len() as u64)
            + gas_costs.contract_root().resolve(predicate_2.len() as u64)
            + gas_costs.contract_root().resolve(predicate_3.len() as u64)
            + 3 * gas_costs.vm_initialization().resolve(tx.size() as u64)
            + 50
            + 100
            + 200
            + gas_costs.s256().resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee + gas_limit * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    #[test]
    fn min_fee_multiple_signed_and_predicate_inputs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 100;
        let gas_limit = 1000;
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(1);
        let predicate_1 = random_bytes::<1024, _>(rng);
        let predicate_2 = random_bytes::<2048, _>(rng);
        let predicate_3 = random_bytes::<4096, _>(rng);
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            // Set up 3 signed inputs
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen::<u32>() as u64,
                vec![],
            )
            // Set up 3 predicate inputs
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                50,
                predicate_1.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                100,
                predicate_2.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                200,
                predicate_3.to_vec(),
                vec![],
            ))
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64
            * fee_params.gas_per_byte()
            + 3 * gas_costs.eck1()
            + gas_costs.vm_initialization().resolve(tx.size() as u64)
            + gas_costs.contract_root().resolve(predicate_1.len() as u64)
            + gas_costs.contract_root().resolve(predicate_2.len() as u64)
            + gas_costs.contract_root().resolve(predicate_3.len() as u64)
            + 3 * gas_costs.vm_initialization().resolve(tx.size() as u64)
            + 50
            + 100
            + 200
            + gas_costs.s256().resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee + gas_limit * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    #[test]
    fn fee_create_tx() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 100;
        let witness_limit = 1000;
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(1);
        let gen_storage_slot = || rng.r#gen::<StorageSlot>();
        let storage_slots = core::iter::repeat_with(gen_storage_slot)
            .take(100)
            .collect::<Vec<_>>();
        let storage_slots_len = storage_slots.len();
        let bytecode = rng.r#gen::<Witness>();
        let bytecode_len = bytecode.as_ref().len();
        let salt = rng.r#gen::<Salt>();
        let tx = TransactionBuilder::create(bytecode.clone(), salt, storage_slots)
            .witness_limit(witness_limit)
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64
            * fee_params.gas_per_byte()
            + gas_costs.state_root().resolve(storage_slots_len as Word)
            + gas_costs.contract_root().resolve(bytecode_len as Word)
            + gas_costs.vm_initialization().resolve(tx.size() as u64)
            + gas_costs.s256().resolve(100)
            + gas_costs.s256().resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee
            + (witness_limit - bytecode.size() as u64)
                * fee_params.gas_per_byte()
                * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    #[test]
    fn fee_create_tx_no_bytecode() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 100;
        let witness_limit = 1000;
        let gas_costs = GasCosts::default();
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(1);
        let bytecode: Witness = Vec::<u8>::new().into();
        let salt = rng.r#gen::<Salt>();
        let tx = TransactionBuilder::create(bytecode.clone(), salt, vec![])
            .witness_limit(witness_limit)
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64
            * fee_params.gas_per_byte()
            + gas_costs.state_root().resolve(0)
            + gas_costs.contract_root().resolve(0)
            + gas_costs.vm_initialization().resolve(tx.size() as u64)
            + gas_costs.s256().resolve(100)
            + gas_costs.s256().resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee
            + (witness_limit - bytecode.size_static() as u64)
                * fee_params.gas_per_byte()
                * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    #[test]
    fn checked_tx_rejects_invalid_tx() {
        // simple smoke test that invalid txs cannot be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let asset = rng.r#gen();
        let gas_limit = 100;
        let input_amount = 1_000;

        // create a tx with invalid signature
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            .add_input(Input::coin_signed(
                rng.r#gen(),
                rng.r#gen(),
                input_amount,
                asset,
                rng.r#gen(),
                Default::default(),
            ))
            .add_input(Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ))
            .add_output(Output::contract(1, rng.r#gen(), rng.r#gen()))
            .add_output(Output::coin(rng.r#gen(), 10, asset))
            .add_output(Output::change(rng.r#gen(), 0, asset))
            .add_witness(Default::default())
            .finalize();

        let err = tx
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect_err("Expected invalid transaction");

        // assert that tx without base input assets fails
        assert!(matches!(
            err,
            CheckError::Validity(ValidityError::InputInvalidSignature { .. })
        ));
    }

    #[test]
    fn into_checked__tx_fails_when_provided_fees_dont_cover_byte_costs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let arb_input_amount = 1;
        let gas_price = 2; // price > amount
        let gas_limit = 0; // don't include any gas execution fees
        let factor = 1;
        let zero_max_fee = 0;
        let params = params(factor);

        // setup "valid" transaction
        let transaction = base_asset_tx(rng, arb_input_amount, gas_limit, zero_max_fee);
        transaction
            .clone()
            .into_checked(Default::default(), &params)
            .unwrap();
        let fees = TransactionFee::checked_from_tx(
            &GasCosts::default(),
            params.fee_params(),
            &transaction,
            gas_price,
        )
        .unwrap();
        let real_max_fee = fees.max_fee();

        let new_input_amount = real_max_fee;
        let mut new_transaction =
            base_asset_tx(rng, new_input_amount, gas_limit, real_max_fee);
        new_transaction
            .clone()
            .into_checked(Default::default(), &params)
            .unwrap()
            .into_ready(gas_price, &GasCosts::default(), params.fee_params(), None)
            .expect("`new_transaction` should be fully valid");

        // given
        // invalidating the transaction by increasing witness size
        new_transaction.witnesses_mut().push(rng.r#gen());
        let bigger_checked = new_transaction
            .into_checked(Default::default(), &params)
            .unwrap();

        // when
        let err = bigger_checked
            .into_ready(gas_price, &GasCosts::default(), params.fee_params(), None)
            .expect_err("Expected invalid transaction");

        let max_fee_from_policies = match err {
            CheckError::InsufficientMaxFee {
                max_fee_from_policies,
                ..
            } => max_fee_from_policies,
            _ => panic!("expected insufficient max fee; found {err:?}"),
        };

        // then
        assert_eq!(max_fee_from_policies, real_max_fee);
    }

    #[test]
    fn into_checked__tx_fails_when_provided_fees_dont_cover_fee_limit() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 10;
        let factor = 1;
        // make gas price too high for the input amount
        let gas_limit = input_amount + 1; // make gas cost 1 higher than input amount

        // given
        let input_amount = 10;
        let big_fee_limit = input_amount + 1;

        let transaction = base_asset_tx(rng, input_amount, gas_limit, big_fee_limit);

        let consensus_params = params(factor);

        // when
        let err = transaction
            .into_checked(Default::default(), &consensus_params)
            .expect_err("overflow expected");

        // then
        let provided = match err {
            CheckError::Validity(ValidityError::InsufficientFeeAmount {
                provided,
                ..
            }) => provided,
            _ => panic!("expected insufficient fee amount; found {err:?}"),
        };
        assert_eq!(provided, input_amount);
    }

    #[test]
    fn into_ready__bytes_fee_cant_overflow() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 1000;
        let max_gas_price = Word::MAX;
        let gas_limit = 0; // ensure only bytes are included in fee
        let zero_fee_limit = 0;
        let transaction = base_asset_tx(rng, input_amount, gas_limit, zero_fee_limit);
        let gas_costs = GasCosts::default();

        let consensus_params = params(1);

        let fee_params = consensus_params.fee_params();
        let err = transaction
            .into_checked(Default::default(), &consensus_params)
            .unwrap()
            .into_ready(max_gas_price, &gas_costs, fee_params, None)
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::Validity(ValidityError::BalanceOverflow));
    }

    #[test]
    fn into_ready__fails_if_fee_limit_too_low() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 1000;
        let gas_price = 100;
        let gas_limit = 0; // ensure only bytes are included in fee
        let gas_costs = GasCosts::default();

        let consensus_params = params(1);

        let fee_params = consensus_params.fee_params();

        // given
        let zero_fee_limit = 0;
        let transaction = base_asset_tx(rng, input_amount, gas_limit, zero_fee_limit);

        // when
        let err = transaction
            .into_checked(Default::default(), &consensus_params)
            .unwrap()
            .into_ready(gas_price, &gas_costs, fee_params, None)
            .expect_err("overflow expected");

        // then
        assert!(matches!(err, CheckError::InsufficientMaxFee { .. }));
    }

    #[test]
    fn into_ready__tx_fails_if_tip_not_covered() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        // tx without tip and fee limit that is good
        let input_amount = 1;
        let gas_limit = 1000;
        let params = ConsensusParameters::standard();
        let block_height = 1.into();
        let gas_costs = GasCosts::default();
        let max_fee_limit = input_amount;
        let gas_price = 1;

        let tx_without_tip =
            base_asset_tx_with_tip(rng, input_amount, gas_limit, max_fee_limit, None);
        tx_without_tip
            .clone()
            .into_checked(block_height, &params)
            .unwrap()
            .into_ready(gas_price, &gas_costs, params.fee_params(), None)
            .expect("Should be valid");

        // given
        let tip = 100;
        let tx_without_enough_to_pay_for_tip = base_asset_tx_with_tip(
            rng,
            input_amount,
            gas_limit,
            max_fee_limit,
            Some(tip),
        );
        tx_without_enough_to_pay_for_tip
            .into_checked(block_height, &params)
            .unwrap()
            .into_ready(gas_price, &gas_costs, params.fee_params(), None)
            .expect_err("Expected invalid transaction");

        // when
        let new_input_amount = input_amount + tip;
        let new_gas_limit = new_input_amount;
        let tx = base_asset_tx_with_tip(
            rng,
            new_input_amount,
            gas_limit,
            new_gas_limit,
            Some(tip),
        );

        // then
        tx.clone()
            .into_checked(block_height, &params)
            .unwrap()
            .into_ready(gas_price, &GasCosts::default(), params.fee_params(), None)
            .expect("Should be valid");
    }

    #[test]
    fn into_ready__return_overflow_error_if_gas_price_too_high() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 2; // 2 * max should cause gas fee overflow
        let max_fee_limit = 0;

        let transaction = base_asset_tx(rng, input_amount, gas_limit, max_fee_limit);

        let consensus_params = params(1);

        let err = transaction
            .into_checked(Default::default(), &consensus_params)
            .unwrap()
            .into_ready(
                gas_price,
                &GasCosts::default(),
                consensus_params.fee_params(),
                None,
            )
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::Validity(ValidityError::BalanceOverflow));
    }

    #[test]
    fn checked_tx_fails_if_asset_is_overspent_by_coin_output() {
        let input_amount = 1_000;
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let secret = SecretKey::random(rng);
        let any_asset = rng.r#gen();
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(100)
            // base asset
            .add_unsigned_coin_input(
                secret,
                rng.r#gen(),
                input_amount,
                AssetId::default(),
                rng.r#gen(),
            )
            .add_output(Output::change(rng.r#gen(), 0, AssetId::default()))
            // arbitrary spending asset
            .add_unsigned_coin_input(
                secret,
                rng.r#gen(),
                input_amount,
                any_asset,
                rng.r#gen(),
            )
            .add_output(Output::coin(rng.r#gen(), input_amount + 1, any_asset))
            .add_output(Output::change(rng.r#gen(), 0, any_asset))
            .finalize();

        let checked = tx
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect_err("Expected valid transaction");

        assert_eq!(
            CheckError::Validity(ValidityError::InsufficientInputAmount {
                asset: any_asset,
                expected: input_amount + 1,
                provided: input_amount,
            }),
            checked
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn basic_check_marks_basic_flag() {
        let block_height = 1.into();

        let tx = Transaction::default_test_tx();
        // Sets Checks::Basic
        let checked = tx
            .into_checked_basic(block_height, &ConsensusParameters::standard())
            .unwrap();
        assert!(checked.checks().contains(Checks::Basic));
    }

    #[test]
    fn signatures_check_marks_signatures_flag() {
        let mut rng = StdRng::seed_from_u64(1);
        let block_height = 1.into();
        let max_fee_limit = 0;

        let tx = valid_coin_tx(&mut rng, 100000, 1000000, 10, max_fee_limit);
        let chain_id = ChainId::default();
        let checked = tx
            // Sets Checks::Basic
            .into_checked(
                block_height,
                &ConsensusParameters::standard_with_id(chain_id),
            )
            .unwrap()
            // Sets Checks::Signatures
            .check_signatures(&chain_id)
            .unwrap();

        assert!(
            checked
                .checks()
                .contains(Checks::Basic | Checks::Signatures)
        );
    }

    #[test]
    fn predicates_check_marks_predicate_flag() {
        let mut rng = StdRng::seed_from_u64(1);
        let block_height = 1.into();
        let gas_costs = GasCosts::default();

        let tx = predicate_tx(&mut rng, 1000000, 1000000, 1000000, gas_costs.ret());

        let mut consensus_params = ConsensusParameters::standard();
        consensus_params.set_gas_costs(gas_costs);

        let check_predicate_params = CheckPredicateParams::from(&consensus_params);

        let checked = tx
            // Sets Checks::Basic
            .into_checked(
                block_height,
                &consensus_params,
            )
            .unwrap()
            // Sets Checks::Predicates
            .check_predicates(&check_predicate_params, MemoryInstance::new(), &EmptyStorage)
            .unwrap();
        assert!(
            checked
                .checks()
                .contains(Checks::Basic | Checks::Predicates)
        );
    }

    fn is_valid_max_fee(
        tx: &Script,
        gas_price: u64,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
    ) -> Result<bool, ValidityError> {
        fn gas_to_fee(gas: u64, price: u64, factor: u64) -> u128 {
            let prices_gas = gas as u128 * price as u128;
            let fee = prices_gas / factor as u128;
            let fee_remainder = (prices_gas.rem_euclid(factor as u128) > 0) as u128;
            fee + fee_remainder
        }

        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let gas_used_by_bytes = fee_params
            .gas_per_byte()
            .saturating_mul(tx.metered_bytes_size() as u64);
        let gas_used_by_inputs = tx.gas_used_by_inputs(gas_costs);
        let gas_used_by_metadata = tx.gas_used_by_metadata(gas_costs);
        let min_gas = gas_used_by_bytes
            .saturating_add(gas_used_by_inputs)
            .saturating_add(gas_used_by_metadata)
            .saturating_add(
                gas_costs
                    .vm_initialization()
                    .resolve(tx.metered_bytes_size() as u64),
            );

        // use different division mechanism than impl
        let witness_limit_allowance = tx
            .witness_limit()
            .saturating_sub(tx.witnesses().size_dynamic() as u64)
            .saturating_mul(fee_params.gas_per_byte());
        let max_gas = min_gas
            .saturating_add(*tx.script_gas_limit())
            .saturating_add(witness_limit_allowance);
        let max_fee = gas_to_fee(max_gas, gas_price, fee_params.gas_price_factor());

        let max_fee_with_tip = max_fee.saturating_add(tx.tip() as u128);

        let result = max_fee_with_tip == tx.max_fee(gas_costs, fee_params, gas_price);
        Ok(result)
    }

    fn is_valid_min_fee<Tx>(
        tx: &Tx,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
        gas_price: u64,
    ) -> Result<bool, ValidityError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        // cant overflow as (metered bytes + gas_used_by_predicates) * gas_per_byte <
        // u64::MAX
        let gas_used_by_bytes = fee_params
            .gas_per_byte()
            .saturating_mul(tx.metered_bytes_size() as u64);
        let gas_used_by_inputs = tx.gas_used_by_inputs(gas_costs);
        let gas_used_by_metadata = tx.gas_used_by_metadata(gas_costs);
        let gas = gas_used_by_bytes
            .saturating_add(gas_used_by_inputs)
            .saturating_add(gas_used_by_metadata)
            .saturating_add(
                gas_costs
                    .vm_initialization()
                    .resolve(tx.metered_bytes_size() as u64),
            );
        let total = gas as u128 * gas_price as u128;
        // use different division mechanism than impl
        let fee = total / fee_params.gas_price_factor() as u128;
        let fee_remainder =
            (total.rem_euclid(fee_params.gas_price_factor() as u128) > 0) as u128;
        let rounded_fee = fee
            .saturating_add(fee_remainder)
            .saturating_add(tx.tip() as u128);
        let min_fee = rounded_fee;
        let calculated_min_fee = tx.min_fee(gas_costs, fee_params, gas_price);

        Ok(min_fee == calculated_min_fee)
    }

    fn valid_coin_tx(
        rng: &mut StdRng,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
        max_fee_limit: u64,
    ) -> Script {
        let asset = AssetId::default();
        TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            .max_fee_limit(max_fee_limit)
            .add_unsigned_coin_input(
                SecretKey::random(rng),
                rng.r#gen(),
                input_amount,
                asset,
                rng.r#gen(),
            )
            .add_input(Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ))
            .add_output(Output::contract(1, rng.r#gen(), rng.r#gen()))
            .add_output(Output::coin(rng.r#gen(), output_amount, asset))
            .add_output(Output::change(rng.r#gen(), 0, asset))
            .finalize()
    }

    // used when proptesting to avoid expensive crypto signatures
    fn predicate_tx(
        rng: &mut StdRng,
        gas_limit: u64,
        witness_limit: u64,
        fee_input_amount: u64,
        predicate_gas_used: u64,
    ) -> Script {
        let asset = AssetId::default();
        let predicate = vec![op::ret(1)].into_iter().collect::<Vec<u8>>();
        let owner = Input::predicate_owner(&predicate);
        let zero_fee_limit = 0;
        TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(zero_fee_limit)
            .script_gas_limit(gas_limit)
            .witness_limit(witness_limit)
            .add_input(Input::coin_predicate(
                rng.r#gen(),
                owner,
                fee_input_amount,
                asset,
                rng.r#gen(),
                predicate_gas_used,
                predicate,
                vec![],
            ))
            .add_output(Output::change(rng.r#gen(), 0, asset))
            .finalize()
    }

    // used to verify message inputs can cover fees
    fn signed_message_coin_tx(
        rng: &mut StdRng,
        gas_limit: u64,
        input_amount: u64,
        max_fee: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(max_fee)
            .script_gas_limit(gas_limit)
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.r#gen(),
                rng.r#gen(),
                input_amount,
                vec![],
            )
            .finalize()
    }

    fn predicate_message_coin_tx(
        rng: &mut StdRng,
        gas_limit: u64,
        input_amount: u64,
        tip: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .tip(tip)
            .script_gas_limit(gas_limit)
            .add_input(Input::message_coin_predicate(
                rng.r#gen(),
                rng.r#gen(),
                input_amount,
                rng.r#gen(),
                Default::default(),
                vec![],
                vec![],
            ))
            .finalize()
    }

    fn base_asset_tx(
        rng: &mut StdRng,
        input_amount: u64,
        gas_limit: u64,
        max_fee: u64,
    ) -> Script {
        base_asset_tx_with_tip(rng, input_amount, gas_limit, max_fee, None)
    }

    fn base_asset_tx_with_tip(
        rng: &mut StdRng,
        input_amount: u64,
        gas_limit: u64,
        max_fee: u64,
        tip: Option<u64>,
    ) -> Script {
        let mut builder = TransactionBuilder::script(vec![], vec![]);
        if let Some(tip) = tip {
            builder.tip(tip);
        }
        builder
            .max_fee_limit(max_fee)
            .script_gas_limit(gas_limit)
            .add_unsigned_coin_input(
                SecretKey::random(rng),
                rng.r#gen(),
                input_amount,
                AssetId::default(),
                rng.r#gen(),
            )
            .add_output(Output::change(rng.r#gen(), 0, AssetId::default()))
            .finalize()
    }
}
