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
use std::hash::Hash;

use fuel_tx::{
    field::MaxFeeLimit,
    ConsensusParameters,
};

mod balances;
pub mod builder;
pub mod types;

pub use types::*;

use crate::{
    error::PredicateVerificationFailed,
    prelude::*,
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
pub struct PartiallyCheckedTx<Tx: IntoChecked> {
    transaction: Tx,
    metadata: Tx::Metadata,
    checks_bitmask: Checks,
}

impl<Tx: IntoChecked> PartiallyCheckedTx<Tx> {
    fn new(transaction: Tx, metadata: Tx::Metadata, checks_bitmask: Checks) -> Self {
        PartiallyCheckedTx {
            transaction,
            metadata,
            checks_bitmask,
        }
    }

    pub(crate) fn basic(transaction: Tx, metadata: Tx::Metadata) -> Self {
        PartiallyCheckedTx::new(transaction, metadata, Checks::Basic)
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
pub struct FullyCheckedTx<Tx: IntoChecked> {
    gas_price: Word,
    transaction: Tx,
    metadata: Tx::Metadata,
    checks_bitmask: Checks,
}

impl<Tx: IntoChecked> FullyCheckedTx<Tx> {
    /// Consume and decompose components of the `Immutable` transaction.
    pub fn decompose(self) -> (Word, Tx, Tx::Metadata, Checks) {
        let FullyCheckedTx {
            gas_price,
            transaction,
            metadata,
            checks_bitmask,
        } = self;
        (gas_price, transaction, metadata, checks_bitmask)
    }
}

impl<Tx: IntoChecked + Chargeable> PartiallyCheckedTx<Tx> {
    /// Run final checks on `Checked` using dynamic values, e.g. `gas_price`
    pub fn into_fully_checked(
        self,
        gas_price: Word,
        gas_costs: &GasCosts,
        fee_parameters: &FeeParameters,
    ) -> Result<FullyCheckedTx<Tx>, CheckError> {
        let PartiallyCheckedTx {
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

        if max_fee_from_gas_price > max_fee_from_policies {
            Err(CheckError::InsufficientMaxFee {
                max_fee_from_policies,
                max_fee_from_gas_price,
            })
        } else {
            Ok(FullyCheckedTx {
                gas_price,
                transaction,
                metadata,
                checks_bitmask,
            })
        }
    }
}

impl<Tx: IntoChecked + UniqueIdentifier> PartiallyCheckedTx<Tx> {
    /// Returns the transaction ID from the computed metadata
    pub fn id(&self) -> TxId {
        self.transaction
            .cached_id()
            .expect("Transaction metadata should be computed for checked transactions")
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoChecked + Default> Default for PartiallyCheckedTx<Tx>
where
    PartiallyCheckedTx<Tx>: CheckPredicates,
{
    fn default() -> Self {
        Tx::default()
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
            .expect("default tx should produce a valid fully checked transaction")
    }
}

impl<Tx: IntoChecked> From<PartiallyCheckedTx<Tx>> for (Tx, Tx::Metadata) {
    fn from(checked: PartiallyCheckedTx<Tx>) -> Self {
        let PartiallyCheckedTx {
            transaction,
            metadata,
            ..
        } = checked;

        (transaction, metadata)
    }
}

impl<Tx: IntoChecked> AsRef<Tx> for PartiallyCheckedTx<Tx> {
    fn as_ref(&self) -> &Tx {
        &self.transaction
    }
}

#[cfg(feature = "test-helpers")]
impl<Tx: IntoChecked> AsMut<Tx> for PartiallyCheckedTx<Tx> {
    fn as_mut(&mut self) -> &mut Tx {
        &mut self.transaction
    }
}

impl<Tx: IntoChecked> Borrow<Tx> for PartiallyCheckedTx<Tx> {
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
    fn into_partially_checked(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<PartiallyCheckedTx<Self>, CheckError>
    where
        PartiallyCheckedTx<Self>: CheckPredicates,
    {
        let check_predicate_params = consensus_params.into();
        self.into_partially_checked_basic(block_height, consensus_params)?
            .check_signatures(&consensus_params.chain_id)?
            .check_predicates(&check_predicate_params)
    }

    /// Returns transaction that passed only `Checks::Basic`.
    fn into_partially_checked_basic(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<PartiallyCheckedTx<Self>, CheckError>;
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
    pub max_inputs: u8,
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
            chain_id: value.chain_id,
            max_gas_per_predicate: value.predicate_params().max_gas_per_predicate,
            max_gas_per_tx: value.tx_params().max_gas_per_tx,
            max_inputs: value.tx_params().max_inputs,
            contract_max_size: value.contract_params().contract_max_size,
            max_message_data_length: value.predicate_params().max_message_data_length,
            tx_offset: value.tx_params().tx_offset(),
            fee_params: *(value.fee_params()),
            base_asset_id: value.base_asset_id,
        }
    }
}

/// Provides predicate verification functionality for the transaction.
#[async_trait::async_trait]
pub trait CheckPredicates: Sized {
    /// Performs predicates verification of the transaction.
    fn check_predicates(self, params: &CheckPredicateParams) -> Result<Self, CheckError>;

    /// Performs predicates verification of the transaction in parallel.
    async fn check_predicates_async<E: ParallelExecutor>(
        self,
        params: &CheckPredicateParams,
    ) -> Result<Self, CheckError>;
}

/// Provides predicate estimation functionality for the transaction.
#[async_trait::async_trait]
pub trait EstimatePredicates: Sized {
    /// Estimates predicates of the transaction.
    fn estimate_predicates(
        &mut self,
        params: &CheckPredicateParams,
    ) -> Result<(), CheckError>;

    /// Estimates predicates of the transaction in parallel.
    async fn estimate_predicates_async<E: ParallelExecutor>(
        &mut self,
        params: &CheckPredicateParams,
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
        F: FnOnce() -> Result<(Word, usize), PredicateVerificationFailed>
            + Send
            + 'static;

    /// Executes tasks created by `create_task` in parallel.
    async fn execute_tasks(
        futures: Vec<Self::Task>,
    ) -> Vec<Result<(Word, usize), PredicateVerificationFailed>>;
}

#[async_trait::async_trait]
impl<Tx> CheckPredicates for PartiallyCheckedTx<Tx>
where
    Tx: ExecutableTransaction + Send + Sync + 'static,
    <Tx as IntoChecked>::Metadata: crate::interpreter::CheckedMetadata + Send + Sync,
{
    fn check_predicates(
        mut self,
        params: &CheckPredicateParams,
    ) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Predicates) {
            Interpreter::<PredicateStorage, _>::check_predicates(&self, params)?;
            self.checks_bitmask.insert(Checks::Predicates);
        }
        Ok(self)
    }

    async fn check_predicates_async<E>(
        mut self,
        params: &CheckPredicateParams,
    ) -> Result<Self, CheckError>
    where
        E: ParallelExecutor,
    {
        if !self.checks_bitmask.contains(Checks::Predicates) {
            Interpreter::<PredicateStorage, _>::check_predicates_async::<E>(
                &self, params,
            )
            .await?;

            self.checks_bitmask.insert(Checks::Predicates);

            Ok(self)
        } else {
            Ok(self)
        }
    }
}

#[async_trait::async_trait]
impl<Tx: ExecutableTransaction + Send + Sync + 'static> EstimatePredicates for Tx {
    fn estimate_predicates(
        &mut self,
        params: &CheckPredicateParams,
    ) -> Result<(), CheckError> {
        Interpreter::<PredicateStorage, _>::estimate_predicates(self, params)?;
        Ok(())
    }

    async fn estimate_predicates_async<E>(
        &mut self,
        params: &CheckPredicateParams,
    ) -> Result<(), CheckError>
    where
        E: ParallelExecutor,
    {
        Interpreter::<PredicateStorage, _>::estimate_predicates_async::<E>(self, params)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EstimatePredicates for Transaction {
    fn estimate_predicates(
        &mut self,
        params: &CheckPredicateParams,
    ) -> Result<(), CheckError> {
        match self {
            Transaction::Script(script) => script.estimate_predicates(params),
            Transaction::Create(create) => create.estimate_predicates(params),
            Transaction::Mint(_) => Ok(()),
        }
    }

    async fn estimate_predicates_async<E: ParallelExecutor>(
        &mut self,
        params: &CheckPredicateParams,
    ) -> Result<(), CheckError> {
        match self {
            Transaction::Script(script) => {
                script.estimate_predicates_async::<E>(params).await
            }
            Transaction::Create(create) => {
                create.estimate_predicates_async::<E>(params).await
            }
            Transaction::Mint(_) => Ok(()),
        }
    }
}

#[async_trait::async_trait]
impl CheckPredicates for PartiallyCheckedTx<Mint> {
    fn check_predicates(
        mut self,
        _params: &CheckPredicateParams,
    ) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Predicates);
        Ok(self)
    }

    async fn check_predicates_async<E: ParallelExecutor>(
        mut self,
        _params: &CheckPredicateParams,
    ) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Predicates);
        Ok(self)
    }
}

#[async_trait::async_trait]
impl CheckPredicates for PartiallyCheckedTx<Transaction> {
    fn check_predicates(self, params: &CheckPredicateParams) -> Result<Self, CheckError> {
        let checked_transaction: CheckedTransaction = self.into();
        let checked_transaction: CheckedTransaction = match checked_transaction {
            CheckedTransaction::Script(tx) => {
                CheckPredicates::check_predicates(tx, params)?.into()
            }
            CheckedTransaction::Create(tx) => {
                CheckPredicates::check_predicates(tx, params)?.into()
            }
            CheckedTransaction::Mint(tx) => {
                CheckPredicates::check_predicates(tx, params)?.into()
            }
        };
        Ok(checked_transaction.into())
    }

    async fn check_predicates_async<E>(
        mut self,
        params: &CheckPredicateParams,
    ) -> Result<Self, CheckError>
    where
        E: ParallelExecutor,
    {
        let checked_transaction: CheckedTransaction = self.into();

        let checked_transaction: CheckedTransaction = match checked_transaction {
            CheckedTransaction::Script(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params)
                    .await?
                    .into()
            }
            CheckedTransaction::Create(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params)
                    .await?
                    .into()
            }
            CheckedTransaction::Mint(tx) => {
                CheckPredicates::check_predicates_async::<E>(tx, params)
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
    Script(PartiallyCheckedTx<Script>),
    Create(PartiallyCheckedTx<Create>),
    Mint(PartiallyCheckedTx<Mint>),
}

impl From<PartiallyCheckedTx<Transaction>> for CheckedTransaction {
    fn from(checked: PartiallyCheckedTx<Transaction>) -> Self {
        let PartiallyCheckedTx {
            transaction,
            metadata,
            checks_bitmask,
        } = checked;

        // # Dev note: Avoid wildcard pattern to be sure that all variants are covered.
        match (transaction, metadata) {
            (Transaction::Script(transaction), CheckedMetadata::Script(metadata)) => {
                Self::Script(PartiallyCheckedTx::new(
                    transaction,
                    metadata,
                    checks_bitmask,
                ))
            }
            (Transaction::Create(transaction), CheckedMetadata::Create(metadata)) => {
                Self::Create(PartiallyCheckedTx::new(
                    transaction,
                    metadata,
                    checks_bitmask,
                ))
            }
            (Transaction::Mint(transaction), CheckedMetadata::Mint(metadata)) => {
                Self::Mint(PartiallyCheckedTx::new(
                    transaction,
                    metadata,
                    checks_bitmask,
                ))
            }
            // The code should produce the `CheckedMetadata` for the corresponding
            // transaction variant. It is done in the implementation of the
            // `IntoChecked` trait for `Transaction`. With the current
            // implementation, the patterns below are unreachable.
            (Transaction::Script(_), _) => unreachable!(),
            (Transaction::Create(_), _) => unreachable!(),
            (Transaction::Mint(_), _) => unreachable!(),
        }
    }
}

impl From<PartiallyCheckedTx<Script>> for CheckedTransaction {
    fn from(checked: PartiallyCheckedTx<Script>) -> Self {
        Self::Script(checked)
    }
}

impl From<PartiallyCheckedTx<Create>> for CheckedTransaction {
    fn from(checked: PartiallyCheckedTx<Create>) -> Self {
        Self::Create(checked)
    }
}

impl From<PartiallyCheckedTx<Mint>> for CheckedTransaction {
    fn from(checked: PartiallyCheckedTx<Mint>) -> Self {
        Self::Mint(checked)
    }
}

impl From<CheckedTransaction> for PartiallyCheckedTx<Transaction> {
    fn from(checked: CheckedTransaction) -> Self {
        match checked {
            CheckedTransaction::Script(PartiallyCheckedTx {
                transaction,
                metadata,
                checks_bitmask,
            }) => PartiallyCheckedTx::new(
                transaction.into(),
                metadata.into(),
                checks_bitmask,
            ),
            CheckedTransaction::Create(PartiallyCheckedTx {
                transaction,
                metadata,
                checks_bitmask,
            }) => PartiallyCheckedTx::new(
                transaction.into(),
                metadata.into(),
                checks_bitmask,
            ),
            CheckedTransaction::Mint(PartiallyCheckedTx {
                transaction,
                metadata,
                checks_bitmask,
            }) => PartiallyCheckedTx::new(
                transaction.into(),
                metadata.into(),
                checks_bitmask,
            ),
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

impl IntoChecked for Transaction {
    type Metadata = CheckedMetadata;

    fn into_partially_checked_basic(
        self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<PartiallyCheckedTx<Self>, CheckError> {
        match self {
            Transaction::Script(script) => {
                let (transaction, metadata) = script
                    .into_partially_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Transaction::Create(create) => {
                let (transaction, metadata) = create
                    .into_partially_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
            Transaction::Mint(mint) => {
                let (transaction, metadata) = mint
                    .into_partially_checked_basic(block_height, consensus_params)?
                    .into();
                Ok((transaction.into(), metadata.into()))
            }
        }
        .map(|(transaction, metadata)| PartiallyCheckedTx::basic(transaction, metadata))
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
#[cfg(test)]
mod tests {
    #![allow(clippy::cast_possible_truncation)]

    use super::*;
    use alloc::vec;
    use fuel_asm::op;
    use fuel_crypto::SecretKey;
    use fuel_tx::{
        field::{
            ScriptGasLimit,
            Tip,
            WitnessLimit,
            Witnesses,
        },
        Script,
        TransactionBuilder,
        ValidityError,
    };
    use fuel_types::canonical::Serialize;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
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
        )
    }

    #[test]
    fn checked_tx_accepts_valid_tx() {
        // simple smoke test that valid txs can be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_limit = 1000;
        let input_amount = 1000;
        let output_amount = 10;
        let max_fee_limit = 0;
        let tx =
            valid_coin_tx(rng, gas_limit, input_amount, output_amount, max_fee_limit);

        let checked = tx
            .clone()
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
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
    fn into_partially_checked__tx_accepts_valid_signed_message_coin_for_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_limit = 1000;
        let zero_fee_limit = 0;
        let tx = signed_message_coin_tx(rng, gas_limit, input_amount, zero_fee_limit);

        let checked = tx
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.transaction.max_fee_limit()
        );
    }

    #[test]
    fn into_partially_checked__tx_excludes_message_output_amount_from_fee() {
        // ensure message outputs aren't deducted from available balance
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_limit = 1000;
        let zero_fee_limit = 0;
        let tx = signed_message_coin_tx(rng, gas_limit, input_amount, zero_fee_limit);

        let checked = tx
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.transaction.max_fee_limit()
        );
    }

    #[test]
    fn into_partially_checked__message_data_signed_message_is_not_used_to_cover_fees() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        // given
        let input_amount = 100;

        // when
        let max_fee = input_amount;
        let tx = TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(max_fee)
            // Add message input with enough to cover max fee
            .add_unsigned_message_input(SecretKey::random(rng), rng.gen(), rng.gen(), input_amount, vec![0xff; 10])
            // Add empty base coin
            .add_unsigned_coin_input(SecretKey::random(rng), rng.gen(), 0, AssetId::BASE, rng.gen())
            .finalize();

        let err = tx
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
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
                rng.gen(),
                rng.gen(),
                input_amount,
                rng.gen(),
                Default::default(),
                vec![0xff; 10],
                vec![0xaa; 10],
                vec![0xbb; 10],
            ))
            // Add empty base coin
            .add_unsigned_coin_input(SecretKey::random(rng), rng.gen(), 0, AssetId::BASE, rng.gen())
            .finalize();

        let err = tx
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
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
        let predicate_gas_used = rng.gen();
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
        let predicate_gas_used = rng.gen();
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
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64 * fee_params.gas_per_byte
            + gas_costs.vm_initialization.resolve(tx.size() as u64)
            + 3 * gas_costs.ecr1
            + gas_costs.s256.resolve(tx.size() as u64))
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
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                secret,
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                secret,
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
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
        let expected_min_fee = (tx.metered_bytes_size() as u64 * fee_params.gas_per_byte
            + gas_costs.vm_initialization.resolve(tx.size() as u64)
            + gas_costs.ecr1
            + gas_costs.s256.resolve(tx.size() as u64))
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
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                50,
                predicate_1.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                100,
                predicate_2.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                200,
                predicate_3.to_vec(),
                vec![],
            ))
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.size() as u64 * fee_params.gas_per_byte
            + gas_costs.vm_initialization.resolve(tx.size() as u64)
            + gas_costs.contract_root.resolve(predicate_1.len() as u64)
            + gas_costs.contract_root.resolve(predicate_2.len() as u64)
            + gas_costs.contract_root.resolve(predicate_3.len() as u64)
            + 3 * gas_costs.vm_initialization.resolve(tx.size() as u64)
            + 50
            + 100
            + 200
            + gas_costs.s256.resolve(tx.size() as u64))
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
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            .add_unsigned_message_input(
                SecretKey::random(rng),
                rng.gen(),
                rng.gen(),
                rng.gen::<u32>() as u64,
                vec![],
            )
            // Set up 3 predicate inputs
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                50,
                predicate_1.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                100,
                predicate_2.to_vec(),
                vec![],
            ))
            .add_input(Input::message_coin_predicate(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                200,
                predicate_3.to_vec(),
                vec![],
            ))
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64 * fee_params.gas_per_byte
            + 3 * gas_costs.ecr1
            + gas_costs.vm_initialization.resolve(tx.size() as u64)
            + gas_costs.contract_root.resolve(predicate_1.len() as u64)
            + gas_costs.contract_root.resolve(predicate_2.len() as u64)
            + gas_costs.contract_root.resolve(predicate_3.len() as u64)
            + 3 * gas_costs.vm_initialization.resolve(tx.size() as u64)
            + 50
            + 100
            + 200
            + gas_costs.s256.resolve(tx.size() as u64))
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
        let gen_storage_slot = || rng.gen::<StorageSlot>();
        let storage_slots = core::iter::repeat_with(gen_storage_slot)
            .take(100)
            .collect::<Vec<_>>();
        let storage_slots_len = storage_slots.len();
        let bytecode = rng.gen::<Witness>();
        let bytecode_len = bytecode.as_ref().len();
        let salt = rng.gen::<Salt>();
        let tx = TransactionBuilder::create(bytecode.clone(), salt, storage_slots)
            .witness_limit(witness_limit)
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64 * fee_params.gas_per_byte
            + gas_costs.state_root.resolve(storage_slots_len as Word)
            + gas_costs.contract_root.resolve(bytecode_len as Word)
            + gas_costs.vm_initialization.resolve(tx.size() as u64)
            + gas_costs.s256.resolve(100)
            + gas_costs.s256.resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee
            + (witness_limit - bytecode.size() as u64)
                * fee_params.gas_per_byte
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
        let salt = rng.gen::<Salt>();
        let tx = TransactionBuilder::create(bytecode.clone(), salt, vec![])
            .witness_limit(witness_limit)
            .finalize();
        let fee =
            TransactionFee::checked_from_tx(&gas_costs, &fee_params, &tx, gas_price)
                .unwrap();

        let min_fee = fee.min_fee();
        let expected_min_fee = (tx.metered_bytes_size() as u64 * fee_params.gas_per_byte
            + gas_costs.state_root.resolve(0)
            + gas_costs.contract_root.resolve(0)
            + gas_costs.vm_initialization.resolve(tx.size() as u64)
            + gas_costs.s256.resolve(100)
            + gas_costs.s256.resolve(tx.size() as u64))
            * gas_price;
        assert_eq!(min_fee, expected_min_fee);

        let max_fee = fee.max_fee();
        let expected_max_fee = min_fee
            + (witness_limit - bytecode.size_static() as u64)
                * fee_params.gas_per_byte
                * gas_price;
        assert_eq!(max_fee, expected_max_fee);
    }

    #[test]
    fn checked_tx_rejects_invalid_tx() {
        // simple smoke test that invalid txs cannot be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let asset = rng.gen();
        let gas_limit = 100;
        let input_amount = 1_000;

        // create a tx with invalid signature
        let tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(gas_limit)
            .add_input(Input::coin_signed(
                rng.gen(),
                rng.gen(),
                input_amount,
                asset,
                rng.gen(),
                Default::default(),
            ))
            .add_input(Input::contract(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
            ))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), 10, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
            .add_witness(Default::default())
            .finalize();

        let err = tx
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
            .expect_err("Expected invalid transaction");

        // assert that tx without base input assets fails
        assert!(matches!(
            err,
            CheckError::Validity(ValidityError::InputInvalidSignature { .. })
        ));
    }

    #[test]
    fn into_partially_checked__tx_fails_when_provided_fees_dont_cover_byte_costs() {
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
            .into_partially_checked(Default::default(), &params)
            .unwrap();
        let fees = TransactionFee::checked_from_tx(
            &GasCosts::default(),
            &params.fee_params(),
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
            .into_partially_checked(Default::default(), &params)
            .unwrap()
            .into_fully_checked(gas_price, &GasCosts::default(), &params.fee_params())
            .expect("`new_transaction` should be fully valid");

        // given
        // invalidating the transaction by increasing witness size
        new_transaction.witnesses_mut().push(rng.gen());
        let bigger_checked = new_transaction
            .into_partially_checked(Default::default(), &params)
            .unwrap();

        // when
        let err = bigger_checked
            .into_fully_checked(gas_price, &GasCosts::default(), &params.fee_params())
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
    fn into_partially_checked__tx_fails_when_provided_fees_dont_cover_fee_limit() {
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
            .into_partially_checked(Default::default(), &consensus_params)
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
    fn into_immutable__bytes_fee_cant_overflow() {
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
            .into_partially_checked(Default::default(), &consensus_params)
            .unwrap()
            .into_fully_checked(max_gas_price, &gas_costs, &fee_params)
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::Validity(ValidityError::BalanceOverflow));
    }

    #[test]
    fn into_immutable__fails_if_fee_limit_too_low() {
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
            .into_partially_checked(Default::default(), &consensus_params)
            .unwrap()
            .into_fully_checked(gas_price, &gas_costs, &fee_params)
            .expect_err("overflow expected");

        // then
        assert!(matches!(err, CheckError::InsufficientMaxFee { .. }));
    }

    #[test]
    fn into_immutable__tx_fails_if_tip_not_covered() {
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
            .into_partially_checked(block_height, &params)
            .unwrap()
            .into_fully_checked(gas_price, &gas_costs, &params.fee_params())
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
            .into_partially_checked(block_height, &params)
            .unwrap()
            .into_fully_checked(gas_price, &gas_costs, &params.fee_params())
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
            .into_partially_checked(block_height, &params)
            .unwrap()
            .into_fully_checked(gas_price, &GasCosts::default(), &params.fee_params())
            .expect("Should be valid");
    }

    #[test]
    fn into_immutable__return_overflow_error_if_gas_price_too_high() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 2; // 2 * max should cause gas fee overflow
        let max_fee_limit = 0;

        let transaction = base_asset_tx(rng, input_amount, gas_limit, max_fee_limit);

        let consensus_params = params(1);

        let err = transaction
            .into_partially_checked(Default::default(), &consensus_params)
            .unwrap()
            .into_fully_checked(
                gas_price,
                &GasCosts::default(),
                &consensus_params.fee_params(),
            )
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::Validity(ValidityError::BalanceOverflow));
    }

    #[test]
    fn checked_tx_fails_if_asset_is_overspent_by_coin_output() {
        let input_amount = 1_000;
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let secret = SecretKey::random(rng);
        let any_asset = rng.gen();
        let zero_gas_limit = 0;
        let tx = TransactionBuilder::script(vec![], vec![])
            .max_fee_limit(zero_gas_limit)
            .script_gas_limit(100)
            // base asset
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                input_amount,
                AssetId::default(),
                rng.gen(),
            )
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            // arbitrary spending asset
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                input_amount,
                any_asset,
                rng.gen(),
            )
            .add_output(Output::coin(rng.gen(), input_amount + 1, any_asset))
            .add_output(Output::change(rng.gen(), 0, any_asset))
            .finalize();

        let checked = tx
            .into_partially_checked(Default::default(), &ConsensusParameters::standard())
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
            .into_partially_checked_basic(block_height, &ConsensusParameters::standard())
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
            .into_partially_checked(
                block_height,
                &ConsensusParameters::standard_with_id(chain_id),
            )
            .unwrap()
            // Sets Checks::Signatures
            .check_signatures(&chain_id)
            .unwrap();

        assert!(checked
            .checks()
            .contains(Checks::Basic | Checks::Signatures));
    }

    #[test]
    fn predicates_check_marks_predicate_flag() {
        let mut rng = StdRng::seed_from_u64(1);
        let block_height = 1.into();
        let gas_costs = GasCosts::default();

        let tx = predicate_tx(&mut rng, 1000000, 1000000, 1000000, gas_costs.ret);

        let consensus_params = ConsensusParameters {
            gas_costs,
            ..ConsensusParameters::standard()
        };

        let check_predicate_params = CheckPredicateParams::from(&consensus_params);

        let checked = tx
            // Sets Checks::Basic
            .into_partially_checked(
                block_height,
                &consensus_params,
            )
            .unwrap()
            // Sets Checks::Predicates
            .check_predicates(&check_predicate_params)
            .unwrap();
        assert!(checked
            .checks()
            .contains(Checks::Basic | Checks::Predicates));
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
            .gas_per_byte
            .saturating_mul(tx.metered_bytes_size() as u64);
        let gas_used_by_inputs = tx.gas_used_by_inputs(gas_costs);
        let gas_used_by_metadata = tx.gas_used_by_metadata(gas_costs);
        let min_gas = gas_used_by_bytes
            .saturating_add(gas_used_by_inputs)
            .saturating_add(gas_used_by_metadata)
            .saturating_add(
                gas_costs
                    .vm_initialization
                    .resolve(tx.metered_bytes_size() as u64),
            );

        // use different division mechanism than impl
        let witness_limit_allowance = tx
            .witness_limit()
            .saturating_sub(tx.witnesses().size_dynamic() as u64)
            .saturating_mul(fee_params.gas_per_byte);
        let max_gas = min_gas
            .saturating_add(*tx.script_gas_limit())
            .saturating_add(witness_limit_allowance);
        let max_fee = gas_to_fee(max_gas, gas_price, fee_params.gas_price_factor);

        let max_fee_with_tip = max_fee.saturating_add(tx.tip() as u128);

        let result = max_fee_with_tip == tx.max_fee(&gas_costs, &fee_params, gas_price);
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
            .gas_per_byte
            .saturating_mul(tx.metered_bytes_size() as u64);
        let gas_used_by_inputs = tx.gas_used_by_inputs(gas_costs);
        let gas_used_by_metadata = tx.gas_used_by_metadata(gas_costs);
        let gas = gas_used_by_bytes
            .saturating_add(gas_used_by_inputs)
            .saturating_add(gas_used_by_metadata)
            .saturating_add(
                gas_costs
                    .vm_initialization
                    .resolve(tx.metered_bytes_size() as u64),
            );
        let total = gas as u128 * gas_price as u128;
        // use different division mechanism than impl
        let fee = total / fee_params.gas_price_factor as u128;
        let fee_remainder =
            (total.rem_euclid(fee_params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = fee
            .saturating_add(fee_remainder)
            .saturating_add(tx.tip() as u128);
        let min_fee = rounded_fee;
        let calculated_min_fee = tx.min_fee(&gas_costs, &fee_params, gas_price);

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
                rng.gen(),
                input_amount,
                asset,
                rng.gen(),
            )
            .add_input(Input::contract(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
                rng.gen(),
            ))
            .add_output(Output::contract(1, rng.gen(), rng.gen()))
            .add_output(Output::coin(rng.gen(), output_amount, asset))
            .add_output(Output::change(rng.gen(), 0, asset))
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
                rng.gen(),
                owner,
                fee_input_amount,
                asset,
                rng.gen(),
                predicate_gas_used,
                predicate,
                vec![],
            ))
            .add_output(Output::change(rng.gen(), 0, asset))
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
                rng.gen(),
                rng.gen(),
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
                rng.gen(),
                rng.gen(),
                input_amount,
                rng.gen(),
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
                rng.gen(),
                input_amount,
                AssetId::default(),
                rng.gen(),
            )
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            .finalize()
    }
}
