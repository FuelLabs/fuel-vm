//! A checked transaction is type-wrapper for transactions which have been checked.
//! It is impossible to construct a checked transaction without performing necessary
//! checks.
//!
//! This allows the VM to accept transactions with metadata that have been already
//! verified upstream.

#![allow(non_upper_case_globals)]

use fuel_tx::{
    CheckError,
    Create,
    Mint,
    Script,
    Transaction,
};
use fuel_types::{
    BlockHeight,
    ChainId,
};

use core::borrow::Borrow;
use std::future::Future;

mod balances;
pub mod builder;
pub mod types;

pub use types::*;

use crate::{
    checked_transaction::balances::{
        initial_free_balances,
        AvailableBalances,
    },
    error::PredicateVerificationFailed,
    gas::GasCosts,
    interpreter::{
        CheckedMetadata as CheckedMetadataAccessTrait,
        InitialBalances,
    },
    prelude::*,
};

bitflags::bitflags! {
    /// Possible types of transaction checks.
    pub struct Checks: u32 {
        /// Basic checks defined in the specification for each transaction:
        /// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/transaction.md#transaction
        const Basic         = 0b00000001;
        /// Check that signature in the transactions are valid.
        const Signatures    = 0b00000010;
        /// Check that predicate in the transactions are valid.
        const Predicates    = 0b00000100;
        /// All possible checks.
        const All           = Self::Basic.bits
                            | Self::Signatures.bits
                            | Self::Predicates.bits;
    }
}

impl core::fmt::Display for Checks {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:032b}", self.bits)
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
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
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

#[derive(Debug, Clone, Copy)]
pub struct ConsensusParams<'a> {
    tx_params: &'a TxParameters,
    predicate_params: &'a PredicateParameters,
    script_params: &'a ScriptParameters,
    contract_params: &'a ContractParameters,
    fee_params: &'a FeeParameters,
}

impl<'a> ConsensusParams<'a> {
    pub fn standard() -> Self {
        Self {
            tx_params: &TxParameters::DEFAULT,
            predicate_params: &PredicateParameters::DEFAULT,
            script_params: &ScriptParameters::DEFAULT,
            contract_params: &ContractParameters::DEFAULT,
            fee_params: &FeeParameters::DEFAULT,
        }
    }

    pub fn new(
        tx_params: &'a TxParameters,
        predicate_params: &'a PredicateParameters,
        script_params: &'a ScriptParameters,
        contract_params: &'a ContractParameters,
        fee_params: &'a FeeParameters,
    ) -> Self {
        Self {
            tx_params,
            predicate_params,
            script_params,
            contract_params,
            fee_params,
        }
    }

    pub fn tx_params(&self) -> &TxParameters {
        self.tx_params
    }

    pub fn predicate_params(&self) -> &PredicateParameters {
        self.predicate_params
    }

    pub fn script_params(&self) -> &ScriptParameters {
        self.script_params
    }

    pub fn contract_params(&self) -> &ContractParameters {
        self.contract_params
    }

    pub fn fee_params(&self) -> &FeeParameters {
        self.fee_params
    }
}

/// Performs checks for a transaction
pub trait IntoChecked: FormatValidityChecks + Sized {
    /// Metadata produced during the check.
    type Metadata: Sized;

    /// Returns transaction that passed all `Checks`.
    fn into_checked(
        self,
        block_height: BlockHeight,
        consensus_params: ConsensusParams<'_>,
        chain_id: ChainId,
        gas_costs: GasCosts,
    ) -> Result<Checked<Self>, CheckError>
    where
        Checked<Self>: CheckPredicates,
    {
        let check_predicate_params = CheckPredicateParams {
            gas_costs,
            chain_id,
            max_gas_per_predicate: consensus_params
                .predicate_params()
                .max_gas_per_predicate,
            max_gas_per_tx: consensus_params.tx_params().max_gas_per_tx,
            max_inputs: consensus_params.tx_params().max_inputs,
            contract_max_size: consensus_params.contract_params().contract_max_size,
            max_message_data_length: consensus_params
                .predicate_params()
                .max_message_data_length,
            tx_offset: consensus_params.tx_params().tx_offset(),
            fee_params: consensus_params.fee_params().clone(),
        };
        self.into_checked_basic(block_height, consensus_params, &chain_id)?
            .check_signatures(&chain_id)?
            .check_predicates(check_predicate_params)
    }

    /// Returns transaction that passed only `Checks::Basic`.
    fn into_checked_basic(
        self,
        block_height: BlockHeight,
        // tx_params: &TxParameters,
        // predicate_params: &PredicateParameters,
        // script_params: &ScriptParameters,
        // contract_params: &ContractParameters,
        // fee_params: &FeeParameters,
        consensus_params: ConsensusParams<'_>,
        chain_id: &ChainId,
    ) -> Result<Checked<Self>, CheckError>;
}

#[derive(Debug, Clone)]
pub struct CheckPredicateParams {
    pub gas_costs: GasCosts,
    pub chain_id: ChainId,
    pub max_gas_per_predicate: u64,
    pub max_gas_per_tx: u64,
    pub max_inputs: u64,
    pub contract_max_size: u64,
    pub max_message_data_length: u64,
    pub tx_offset: usize,
    pub fee_params: FeeParameters,
}

impl Default for CheckPredicateParams {
    fn default() -> Self {
        CheckPredicateParams {
            gas_costs: Default::default(),
            chain_id: Default::default(),
            max_gas_per_predicate: PredicateParameters::DEFAULT.max_gas_per_predicate,
            max_gas_per_tx: TxParameters::DEFAULT.max_gas_per_tx,
            max_inputs: TxParameters::DEFAULT.max_inputs,
            contract_max_size: ContractParameters::DEFAULT.contract_max_size,
            max_message_data_length: PredicateParameters::DEFAULT.max_message_data_length,
            tx_offset: TxParameters::DEFAULT.tx_offset(),
            fee_params: Default::default(),
        }
    }
}

/// Provides predicate verification functionality for the transaction.
#[async_trait::async_trait]
pub trait CheckPredicates: Sized {
    /// Performs predicates verification of the transaction.
    fn check_predicates(self, params: CheckPredicateParams) -> Result<Self, CheckError>;

    /// Performs predicates verification of the transaction in parallel.
    async fn check_predicates_async<E: ParallelExecutor>(
        self,
        params: CheckPredicateParams,
    ) -> Result<Self, CheckError>;
}

/// Provides predicate estimation functionality for the transaction.
#[async_trait::async_trait]
pub trait EstimatePredicates: Sized {
    /// Estimates predicates of the transaction.
    fn estimate_predicates(
        &mut self,
        params: CheckPredicateParams,
    ) -> Result<(), CheckError>;

    /// Estimates predicates of the transaction in parallel.
    async fn estimate_predicates_async<E: ParallelExecutor>(
        &mut self,
        params: CheckPredicateParams,
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
impl<Tx> CheckPredicates for Checked<Tx>
where
    Tx: ExecutableTransaction + Send + Sync + 'static,
    <Tx as IntoChecked>::Metadata: crate::interpreter::CheckedMetadata + Send + Sync,
{
    fn check_predicates(
        mut self,
        params: CheckPredicateParams,
    ) -> Result<Self, CheckError> {
        if !self.checks_bitmask.contains(Checks::Predicates) {
            let checked =
                Interpreter::<PredicateStorage>::check_predicates(&self, params)?;
            self.checks_bitmask.insert(Checks::Predicates);
            self.metadata.set_gas_used_by_predicates(checked.gas_used());
        }
        Ok(self)
    }

    async fn check_predicates_async<E>(
        mut self,
        params: CheckPredicateParams,
    ) -> Result<Self, CheckError>
    where
        E: ParallelExecutor,
    {
        if !self.checks_bitmask.contains(Checks::Predicates) {
            let predicates_checked =
                Interpreter::<PredicateStorage>::check_predicates_async::<_, E>(
                    &self, params,
                )
                .await?;

            self.checks_bitmask.insert(Checks::Predicates);
            self.metadata
                .set_gas_used_by_predicates(predicates_checked.gas_used());

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
        params: CheckPredicateParams,
    ) -> Result<(), CheckError> {
        // validate fees and compute free balances
        let AvailableBalances {
            non_retryable_balances,
            retryable_balance,
            ..
        } = initial_free_balances(self, &params.fee_params)?;

        let balances: InitialBalances = InitialBalances {
            non_retryable: NonRetryableFreeBalances(non_retryable_balances),
            retryable: Some(RetryableAmount(retryable_balance)),
        };

        Interpreter::<PredicateStorage>::estimate_predicates(self, balances, params)?;
        Ok(())
    }

    async fn estimate_predicates_async<E>(
        &mut self,
        params: CheckPredicateParams,
    ) -> Result<(), CheckError>
    where
        E: ParallelExecutor,
    {
        // validate fees and compute free balances
        let AvailableBalances {
            non_retryable_balances,
            retryable_balance,
            ..
        } = initial_free_balances(self, &params.fee_params)?;

        let balances: InitialBalances = InitialBalances {
            non_retryable: NonRetryableFreeBalances(non_retryable_balances),
            retryable: Some(RetryableAmount(retryable_balance)),
        };

        Interpreter::<PredicateStorage>::estimate_predicates_async::<_, E>(
            self, balances, params,
        )
        .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EstimatePredicates for Transaction {
    fn estimate_predicates(
        &mut self,
        params: CheckPredicateParams,
    ) -> Result<(), CheckError> {
        match self {
            Transaction::Script(script) => script.estimate_predicates(params),
            Transaction::Create(create) => create.estimate_predicates(params),
            Transaction::Mint(_) => Ok(()),
        }
    }

    async fn estimate_predicates_async<E: ParallelExecutor>(
        &mut self,
        params: CheckPredicateParams,
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
impl CheckPredicates for Checked<Mint> {
    fn check_predicates(
        mut self,
        _params: CheckPredicateParams,
    ) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Predicates);
        Ok(self)
    }

    async fn check_predicates_async<E: ParallelExecutor>(
        mut self,
        _params: CheckPredicateParams,
    ) -> Result<Self, CheckError> {
        self.checks_bitmask.insert(Checks::Predicates);
        Ok(self)
    }
}

#[async_trait::async_trait]
impl CheckPredicates for Checked<Transaction> {
    fn check_predicates(self, params: CheckPredicateParams) -> Result<Self, CheckError> {
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
        params: CheckPredicateParams,
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
    Script(Checked<Script>),
    Create(Checked<Create>),
    Mint(Checked<Mint>),
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

    fn into_checked_basic(
        self,
        block_height: BlockHeight,
        consensus_params: ConsensusParams<'_>,
        chain_id: &ChainId,
    ) -> Result<Checked<Self>, CheckError> {
        let (transaction, metadata) = match self {
            Transaction::Script(script) => {
                let (transaction, metadata) = script
                    .into_checked_basic(block_height, consensus_params, chain_id)?
                    .into();
                (transaction.into(), metadata.into())
            }
            Transaction::Create(create) => {
                let (transaction, metadata) = create
                    .into_checked_basic(block_height, consensus_params, chain_id)?
                    .into();
                (transaction.into(), metadata.into())
            }
            Transaction::Mint(mint) => {
                let (transaction, metadata) = mint
                    .into_checked_basic(block_height, consensus_params, chain_id)?
                    .into();
                (transaction.into(), metadata.into())
            }
        };

        Ok(Checked::basic(transaction, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuel_asm::op;
    use fuel_crypto::SecretKey;
    use fuel_tx::{
        CheckError,
        Script,
        TransactionBuilder,
    };
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    #[test]
    fn checked_tx_accepts_valid_tx() {
        // simple smoke test that valid txs can be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let gas_price = 10;
        let gas_limit = 1000;
        let input_amount = 1000;
        let output_amount = 10;
        let tx = valid_coin_tx(rng, gas_price, gas_limit, input_amount, output_amount);

        let checked = tx
            .clone()
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect("Expected valid transaction");

        // verify transaction getter works
        assert_eq!(checked.transaction(), &tx);
        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.metadata().fee.max_fee() - output_amount
        );
    }

    #[test]
    fn checked_tx_accepts_valid_signed_message_input_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = signed_message_coin_tx(rng, gas_price, gas_limit, input_amount);

        let checked = tx
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.metadata().fee.max_fee()
        );
    }

    #[test]
    fn checked_tx_excludes_message_output_amount_from_fee() {
        // ensure message outputs aren't deducted from available balance
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = signed_message_coin_tx(rng, gas_price, gas_limit, input_amount);

        let checked = tx
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert_eq!(
            checked.metadata().non_retryable_balances[&AssetId::default()],
            input_amount - checked.metadata().fee.max_fee()
        );
    }

    #[test]
    fn message_data_signed_message_is_not_used_to_cover_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_message_input(rng.gen(), rng.gen(), rng.gen(), input_amount, vec![0xff; 10])
            // Add empty base coin
            .add_unsigned_coin_input(rng.gen(), rng.gen(), 0, AssetId::BASE, rng.gen(), rng.gen())
            .finalize();

        let err = tx
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert!(matches!(
            err,
            CheckError::InsufficientFeeAmount {
                expected: _,
                provided: 0
            }
        ));
    }

    #[test]
    fn message_data_predicate_message_is_not_used_to_cover_fees() {
        // simple test to ensure a tx that only has a message input can cover fees
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 100;
        let gas_price = 100;
        let gas_limit = 1000;
        let tx = TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
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
            .add_unsigned_coin_input(rng.gen(), rng.gen(), 0, AssetId::BASE, rng.gen(), rng.gen())
            .finalize();

        let err = tx
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("Expected valid transaction");

        // verify available balance was decreased by max fee
        assert!(matches!(
            err,
            CheckError::InsufficientFeeAmount {
                expected: _,
                provided: 0
            }
        ));
    }

    // use quickcheck to fuzz any rounding or precision errors in the max fee w/ coin
    // input
    #[quickcheck]
    fn max_fee_coin_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify max fee a transaction can consume based on gas limit + bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard()
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let predicate_gas_used = rng.gen();
        let tx =
            predicate_tx(rng, gas_price, gas_limit, input_amount, predicate_gas_used);

        if let Ok(valid) = is_valid_max_fee(&tx, &fee_params) {
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
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify min fee a transaction can consume based on bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard()
        }
        let rng = &mut StdRng::seed_from_u64(seed);
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let predicate_gas_used = rng.gen();
        let tx =
            predicate_tx(rng, gas_price, gas_limit, input_amount, predicate_gas_used);

        if let Ok(valid) = is_valid_max_fee(&tx, &fee_params) {
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
        seed: u64,
    ) -> TestResult {
        // verify max fee a transaction can consume based on gas limit + bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard()
        }

        let rng = &mut StdRng::seed_from_u64(seed);
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_coin_tx(rng, gas_price, gas_limit, input_amount);

        if let Ok(valid) = is_valid_max_fee(&tx, &fee_params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    // use quickcheck to fuzz any rounding or precision errors in the min fee w/ message
    // input
    #[quickcheck]
    fn min_fee_message_input(
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        gas_price_factor: u64,
        seed: u64,
    ) -> TestResult {
        // verify min fee a transaction can consume based on bytes is correct

        // dont divide by zero
        if gas_price_factor == 0 {
            return TestResult::discard()
        }
        let rng = &mut StdRng::seed_from_u64(seed);
        let fee_params = FeeParameters::DEFAULT.with_gas_price_factor(gas_price_factor);
        let tx = predicate_message_coin_tx(rng, gas_price, gas_limit, input_amount);

        if let Ok(valid) = is_valid_min_fee(&tx, &fee_params) {
            TestResult::from_bool(valid)
        } else {
            TestResult::discard()
        }
    }

    #[test]
    fn checked_tx_rejects_invalid_tx() {
        // simple smoke test that invalid txs cannot be checked
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let asset = rng.gen();
        let gas_price = 1;
        let gas_limit = 100;
        let input_amount = 1_000;

        // create a tx with invalid signature
        let tx = TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::coin_signed(
                rng.gen(),
                rng.gen(),
                input_amount,
                asset,
                rng.gen(),
                Default::default(),
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

        let checked = tx
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("Expected invalid transaction");

        // assert that tx without base input assets fails
        assert_eq!(
            CheckError::InsufficientFeeAmount {
                expected: 1,
                provided: 0
            },
            checked
        );
    }

    #[test]
    fn checked_tx_fails_when_provided_fees_dont_cover_byte_costs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 1;
        let gas_price = 2; // price > amount
        let gas_limit = 0; // don't include any gas execution fees
        let factor = 1;

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let tx_params = TxParameters::default();
        let predicate_params = PredicateParameters::default();
        let script_params = ScriptParameters::default();
        let contract_params = ContractParameters::default();
        let fee_params = FeeParameters::default().with_gas_price_factor(factor);

        let consensus_params = ConsensusParams::new(
            &tx_params,
            &predicate_params,
            &script_params,
            &contract_params,
            &fee_params,
        );

        let err = transaction
            .into_checked(
                Default::default(),
                consensus_params,
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("overflow expected");

        let provided = match err {
            CheckError::InsufficientFeeAmount { provided, .. } => provided,
            _ => panic!("expected insufficient fee amount; found {err:?}"),
        };

        assert_eq!(provided, input_amount);
    }

    #[test]
    fn checked_tx_fails_when_provided_fees_dont_cover_gas_costs() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 10;
        let factor = 1;
        // make gas price too high for the input amount
        let gas_price = 1;
        let gas_limit = input_amount + 1; // make gas cost 1 higher than input amount

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let tx_params = TxParameters::default();
        let predicate_params = PredicateParameters::default();
        let script_params = ScriptParameters::default();
        let contract_params = ContractParameters::default();
        let fee_params = FeeParameters::default().with_gas_price_factor(factor);

        let consensus_params = ConsensusParams::new(
            &tx_params,
            &predicate_params,
            &script_params,
            &contract_params,
            &fee_params,
        );

        let err = transaction
            .into_checked(
                Default::default(),
                consensus_params,
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("overflow expected");

        let provided = match err {
            CheckError::InsufficientFeeAmount { provided, .. } => provided,
            _ => panic!("expected insufficient fee amount; found {err:?}"),
        };

        assert_eq!(provided, input_amount);
    }

    #[test]
    fn bytes_fee_cant_overflow() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 0; // ensure only bytes are included in fee
        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let tx_params = TxParameters::default();
        let predicate_params = PredicateParameters::default();
        let script_params = ScriptParameters::default();
        let contract_params = ContractParameters::default();
        let fee_params = FeeParameters::default().with_gas_price_factor(1);

        let consensus_params = ConsensusParams::new(
            &tx_params,
            &predicate_params,
            &script_params,
            &contract_params,
            &fee_params,
        );

        let err = transaction
            .into_checked(
                Default::default(),
                consensus_params,
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::ArithmeticOverflow);
    }

    #[test]
    fn gas_fee_cant_overflow() {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let input_amount = 1000;
        let gas_price = Word::MAX;
        let gas_limit = 2; // 2 * max should cause gas fee overflow

        let transaction = base_asset_tx(rng, input_amount, gas_price, gas_limit);

        let tx_params = TxParameters::default();
        let predicate_params = PredicateParameters::default();
        let script_params = ScriptParameters::default();
        let contract_params = ContractParameters::default();
        let fee_params = FeeParameters::default().with_gas_price_factor(1);

        let consensus_params = ConsensusParams::new(
            &tx_params,
            &predicate_params,
            &script_params,
            &contract_params,
            &fee_params,
        );

        let err = transaction
            .into_checked(
                Default::default(),
                consensus_params,
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("overflow expected");

        assert_eq!(err, CheckError::ArithmeticOverflow);
    }

    #[test]
    fn checked_tx_fails_if_asset_is_overspent_by_coin_output() {
        let input_amount = 1_000;
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let secret = SecretKey::random(rng);
        let any_asset = rng.gen();
        let tx = TransactionBuilder::script(vec![], vec![])
            .gas_price(1)
            .gas_limit(100)
            // base asset
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                input_amount,
                AssetId::default(),
                rng.gen(),
                Default::default(),
            )
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            // arbitrary spending asset
            .add_unsigned_coin_input(
                secret,
                rng.gen(),
                input_amount,
                any_asset,
                rng.gen(),
                Default::default(),
            )
            .add_output(Output::coin(rng.gen(), input_amount + 1, any_asset))
            .add_output(Output::change(rng.gen(), 0, any_asset))
            .finalize();

        let checked = tx
            .into_checked(
                Default::default(),
                ConsensusParams::standard(),
                ChainId::new(0),
                Default::default(),
            )
            .expect_err("Expected valid transaction");

        assert_eq!(
            CheckError::InsufficientInputAmount {
                asset: any_asset,
                expected: input_amount + 1,
                provided: input_amount
            },
            checked
        );
    }

    #[test]
    fn basic_check_marks_basic_flag() {
        let block_height = 1.into();

        let tx = Transaction::default_test_tx();
        // Sets Checks::Basic
        let checked = tx
            .into_checked_basic(
                block_height,
                ConsensusParams::standard(),
                &ChainId::new(0),
            )
            .unwrap();
        assert!(checked.checks().contains(Checks::Basic));
    }

    #[test]
    fn signatures_check_marks_signatures_flag() {
        let mut rng = StdRng::seed_from_u64(1);
        let block_height = 1.into();

        let tx = valid_coin_tx(&mut rng, 1, 100000, 1000000, 10);
        let chain_id = ChainId::default();
        let checked = tx
            // Sets Checks::Basic
            .into_checked(
                block_height,
                ConsensusParams::standard(),
                chain_id,
                Default::default(),
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
        let gas_costs = GasCosts::free();

        let tx = predicate_tx(&mut rng, 1, 1000000, 1000000, 0);

        let params = CheckPredicateParams {
            gas_costs: gas_costs.clone(),
            ..Default::default()
        };

        let checked = tx
            // Sets Checks::Basic
            .into_checked(
                block_height,
                ConsensusParams::standard(),
                ChainId::new(0),
                gas_costs,
            )
            .unwrap()
            // Sets Checks::Predicates
            .check_predicates(params)
            .unwrap();
        assert!(checked
            .checks()
            .contains(Checks::Basic | Checks::Predicates));
    }

    fn is_valid_max_fee<Tx>(
        tx: &Tx,
        fee_params: &FeeParameters,
    ) -> Result<bool, CheckError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        let available_balances = balances::initial_free_balances(tx, fee_params)?;
        // cant overflow as metered bytes * gas_per_byte < u64::MAX
        let bytes = (tx.metered_bytes_size() as u128)
            * fee_params.gas_per_byte as u128
            * tx.price() as u128;
        let gas = tx.limit() as u128 * tx.price() as u128;
        let total = bytes + gas;
        // use different division mechanism than impl
        let fee = total / fee_params.gas_price_factor as u128;
        let fee_remainder =
            (total.rem_euclid(fee_params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.fee.max_fee())
    }

    fn is_valid_min_fee<Tx>(
        tx: &Tx,
        fee_params: &FeeParameters,
    ) -> Result<bool, CheckError>
    where
        Tx: Chargeable + field::Inputs + field::Outputs,
    {
        let available_balances = balances::initial_free_balances(tx, fee_params)?;
        // cant overflow as (metered bytes + gas_used_by_predicates) * gas_per_byte <
        // u64::MAX
        let bytes = (tx.metered_bytes_size() as u128
            + tx.gas_used_by_predicates() as u128)
            * fee_params.gas_per_byte as u128
            * tx.price() as u128;
        // use different division mechanism than impl
        let fee = bytes / fee_params.gas_price_factor as u128;
        let fee_remainder =
            (bytes.rem_euclid(fee_params.gas_price_factor as u128) > 0) as u128;
        let rounded_fee = (fee + fee_remainder) as u64;

        Ok(rounded_fee == available_balances.fee.min_fee())
    }

    fn valid_coin_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
        output_amount: u64,
    ) -> Script {
        let asset = AssetId::default();
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(
                rng.gen(),
                rng.gen(),
                input_amount,
                asset,
                rng.gen(),
                Default::default(),
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
        gas_price: u64,
        gas_limit: u64,
        fee_input_amount: u64,
        predicate_gas_used: u64,
    ) -> Script {
        let asset = AssetId::default();
        let predicate = vec![op::ret(1)].into_iter().collect::<Vec<u8>>();
        let owner = Input::predicate_owner(&predicate, &ChainId::new(0));
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_input(Input::coin_predicate(
                rng.gen(),
                owner,
                fee_input_amount,
                asset,
                rng.gen(),
                Default::default(),
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
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_message_input(
                rng.gen(),
                rng.gen(),
                rng.gen(),
                input_amount,
                vec![],
            )
            .finalize()
    }

    fn predicate_message_coin_tx(
        rng: &mut StdRng,
        gas_price: u64,
        gas_limit: u64,
        input_amount: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
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
        gas_price: u64,
        gas_limit: u64,
    ) -> Script {
        TransactionBuilder::script(vec![], vec![])
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .add_unsigned_coin_input(
                rng.gen(),
                rng.gen(),
                input_amount,
                AssetId::default(),
                rng.gen(),
                Default::default(),
            )
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            .finalize()
    }
}
