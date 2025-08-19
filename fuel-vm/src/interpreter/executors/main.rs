#[cfg(test)]
mod tests;

use crate::{
    checked_transaction::{
        Checked,
        IntoChecked,
        ParallelExecutor,
    },
    context::Context,
    error::{
        Bug,
        InterpreterError,
        PredicateVerificationFailed,
    },
    interpreter::{
        CheckedMetadata,
        EcalHandler,
        ExecutableTransaction,
        InitialBalances,
        Interpreter,
        Memory,
        RuntimeBalances,
    },
    pool::VmMemoryPool,
    predicate::RuntimePredicate,
    prelude::{
        BugVariant,
        RuntimeError,
    },
    state::{
        ExecuteState,
        ProgramState,
        StateTransitionRef,
    },
    storage::{
        BlobData,
        InterpreterStorage,
        predicate::PredicateStorage,
    },
    verification::Verifier,
};
use alloc::{
    vec,
    vec::Vec,
};
use core::fmt::Debug;

use crate::{
    checked_transaction::{
        CheckError,
        CheckPredicateParams,
        Ready,
    },
    interpreter::InterpreterParams,
    prelude::MemoryInstance,
    storage::{
        UploadedBytecode,
        UploadedBytecodes,
        predicate::PredicateStorageRequirements,
    },
};
use fuel_asm::PanicReason;
use fuel_storage::{
    StorageAsMut,
    StorageAsRef,
};
use fuel_tx::{
    Blob,
    BlobIdExt,
    ConsensusParameters,
    Contract,
    Create,
    FeeParameters,
    GasCosts,
    Input,
    Receipt,
    ScriptExecutionResult,
    Transaction,
    Upgrade,
    UpgradeMetadata,
    UpgradePurpose,
    Upload,
    ValidityError,
    field::{
        BlobId as _,
        BytecodeRoot,
        BytecodeWitnessIndex,
        ReceiptsRoot,
        Salt,
        Script as ScriptField,
        ScriptGasLimit,
        StorageSlots,
        SubsectionIndex,
        SubsectionsNumber,
        UpgradePurpose as UpgradePurposeField,
        Witnesses,
    },
    input::{
        coin::CoinPredicate,
        message::{
            MessageCoinPredicate,
            MessageDataPredicate,
        },
    },
};
use fuel_types::{
    AssetId,
    BlobId,
    Word,
};

/// Predicates were checked succesfully
#[derive(Debug, Clone, Copy)]
pub struct PredicatesChecked {
    gas_used: Word,
}

impl PredicatesChecked {
    pub fn gas_used(&self) -> Word {
        self.gas_used
    }
}

enum PredicateRunKind<'a, Tx> {
    Verifying(&'a Tx),
    Estimating(&'a mut Tx),
}

impl<Tx> PredicateRunKind<'_, Tx> {
    fn tx(&self) -> &Tx {
        match self {
            PredicateRunKind::Verifying(tx) => tx,
            PredicateRunKind::Estimating(tx) => tx,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PredicateAction {
    Verifying,
    Estimating { available_gas: Word },
}

/// The module contains functions to check predicates defined in the inputs of a
/// transaction.
pub mod predicates {
    use super::*;
    use crate::storage::predicate::PredicateStorageProvider;

    /// Initialize the VM with the provided transaction and check all predicates defined
    /// in the inputs.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub fn check_predicates<Tx>(
        checked: &Checked<Tx>,
        params: &CheckPredicateParams,
        mut memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
        ecal_handler: impl EcalHandler,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
    {
        let tx = checked.transaction();
        run_predicates(
            PredicateRunKind::Verifying(tx),
            params,
            memory.as_mut(),
            storage,
            ecal_handler,
        )
    }

    /// Initialize the VM with the provided transaction and check all predicates defined
    /// in the inputs in parallel.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub async fn check_predicates_async<Tx, E>(
        checked: &Checked<Tx>,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
        ecal_handler: impl EcalHandler,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
        E: ParallelExecutor,
    {
        let tx = checked.transaction();

        let predicates_checked = run_predicate_async::<Tx, E>(
            PredicateRunKind::Verifying(tx),
            params,
            pool,
            storage,
            ecal_handler,
        )
        .await?;

        Ok(predicates_checked)
    }

    /// Initialize the VM with the provided transaction, check all predicates defined in
    /// the inputs and set the predicate_gas_used to be the actual gas consumed during
    /// execution for each predicate.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub fn estimate_predicates<Tx>(
        transaction: &mut Tx,
        params: &CheckPredicateParams,
        mut memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
        ecal_handler: impl EcalHandler,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        let predicates_checked = run_predicates(
            PredicateRunKind::Estimating(transaction),
            params,
            memory.as_mut(),
            storage,
            ecal_handler,
        )?;
        Ok(predicates_checked)
    }

    /// Initialize the VM with the provided transaction, check all predicates defined in
    /// the inputs and set the predicate_gas_used to be the actual gas consumed during
    /// execution for each predicate in parallel.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub async fn estimate_predicates_async<Tx, E>(
        transaction: &mut Tx,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
        ecal_handler: impl EcalHandler,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        E: ParallelExecutor,
    {
        let predicates_checked = run_predicate_async::<Tx, E>(
            PredicateRunKind::Estimating(transaction),
            params,
            pool,
            storage,
            ecal_handler,
        )
        .await?;

        Ok(predicates_checked)
    }

    async fn run_predicate_async<Tx, E>(
        kind: PredicateRunKind<'_, Tx>,
        params: &CheckPredicateParams,
        pool: &impl VmMemoryPool,
        storage: &impl PredicateStorageProvider,
        ecal_handler: impl EcalHandler,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        E: ParallelExecutor,
    {
        let mut checks = vec![];
        let tx_offset = params.tx_offset;

        let predicate_action = match kind {
            PredicateRunKind::Verifying(_) => PredicateAction::Verifying,
            PredicateRunKind::Estimating(_) => {
                let max_gas_per_tx = params.max_gas_per_tx;
                let max_gas_per_predicate = params.max_gas_per_predicate;
                let available_gas = core::cmp::min(max_gas_per_predicate, max_gas_per_tx);

                PredicateAction::Estimating { available_gas }
            }
        };

        for index in 0..kind.tx().inputs().len() {
            if let Some(predicate) =
                RuntimePredicate::from_tx(kind.tx(), tx_offset, index)
            {
                let tx = kind.tx().clone();
                let my_params = params.clone();
                let mut memory = pool.get_new().await;
                let storage_instance = storage.storage();
                let ecal_handler = ecal_handler.clone();

                let verify_task = E::create_task(move || {
                    let (used_gas, result) = check_predicate(
                        tx,
                        index,
                        predicate_action,
                        predicate,
                        my_params,
                        memory.as_mut(),
                        &storage_instance,
                        ecal_handler,
                    );

                    (index, result.map(|()| used_gas))
                });

                checks.push(verify_task);
            }
        }

        let checks = E::execute_tasks(checks).await;

        finalize_check_predicate(kind, checks, params)
    }

    fn run_predicates<Tx>(
        kind: PredicateRunKind<'_, Tx>,
        params: &CheckPredicateParams,
        mut memory: impl Memory,
        storage: &impl PredicateStorageRequirements,
        ecal_handler: impl EcalHandler,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        let mut checks = vec![];

        let max_gas = kind.tx().max_gas(&params.gas_costs, &params.fee_params);
        let max_gas_per_tx = params.max_gas_per_tx;
        let max_gas_per_predicate = params.max_gas_per_predicate;
        let mut global_available_gas = max_gas_per_tx.saturating_sub(max_gas);

        for index in 0..kind.tx().inputs().len() {
            let tx = kind.tx().clone();

            if let Some(predicate) =
                RuntimePredicate::from_tx(&tx, params.tx_offset, index)
            {
                let available_gas = global_available_gas.min(max_gas_per_predicate);
                let predicate_action = match kind {
                    PredicateRunKind::Verifying(_) => PredicateAction::Verifying,
                    PredicateRunKind::Estimating(_) => {
                        PredicateAction::Estimating { available_gas }
                    }
                };
                let (gas_used, result) = check_predicate(
                    tx,
                    index,
                    predicate_action,
                    predicate,
                    params.clone(),
                    memory.as_mut(),
                    storage,
                    ecal_handler.clone(),
                );
                global_available_gas = global_available_gas.saturating_sub(gas_used);
                checks.push((index, result.map(|()| gas_used)));
            }
        }

        finalize_check_predicate(kind, checks, params)
    }

    #[allow(clippy::too_many_arguments)]
    fn check_predicate<Tx, Ecal>(
        tx: Tx,
        index: usize,
        predicate_action: PredicateAction,
        predicate: RuntimePredicate,
        params: CheckPredicateParams,
        memory: &mut MemoryInstance,
        storage: &impl PredicateStorageRequirements,
        ecal_handler: Ecal,
    ) -> (Word, Result<(), PredicateVerificationFailed>)
    where
        Tx: ExecutableTransaction,
        Ecal: EcalHandler,
    {
        if predicate_action == PredicateAction::Verifying {
            match &tx.inputs()[index] {
                Input::CoinPredicate(CoinPredicate {
                    owner: address,
                    predicate,
                    ..
                })
                | Input::MessageDataPredicate(MessageDataPredicate {
                    recipient: address,
                    predicate,
                    ..
                })
                | Input::MessageCoinPredicate(MessageCoinPredicate {
                    predicate,
                    recipient: address,
                    ..
                }) => {
                    if !Input::is_predicate_owner_valid(address, &**predicate) {
                        return (
                            0,
                            Err(PredicateVerificationFailed::InvalidOwner { index }),
                        );
                    }
                }
                _ => {}
            }
        }

        let zero_gas_price = 0;
        let interpreter_params = InterpreterParams::new(zero_gas_price, params);

        let mut vm = Interpreter::<_, _, _, Ecal>::with_storage_and_ecal(
            memory,
            PredicateStorage::new(storage),
            interpreter_params,
            ecal_handler,
        );

        let (context, available_gas) = match predicate_action {
            PredicateAction::Verifying => {
                let context = Context::PredicateVerification { program: predicate };
                let available_gas = tx.inputs()[index]
                    .predicate_gas_used()
                    .expect("We only run predicates at this stage, so it should exist.");

                (context, available_gas)
            }
            PredicateAction::Estimating { available_gas } => {
                let context = Context::PredicateEstimation { program: predicate };

                (context, available_gas)
            }
        };

        if let Err(err) = vm.init_predicate(context, tx, available_gas) {
            return (
                0,
                Err(PredicateVerificationFailed::interpreter_error(index, err)),
            );
        }

        let result = vm.verify_predicate();
        let is_successful = matches!(result, Ok(ProgramState::Return(0x01)));

        let Some(gas_used) = available_gas.checked_sub(vm.remaining_gas()) else {
            return (0, Err(Bug::new(BugVariant::GlobalGasUnderflow).into()));
        };

        if let PredicateAction::Verifying = predicate_action {
            if !is_successful {
                return if let Err(err) = result {
                    (
                        gas_used,
                        Err(PredicateVerificationFailed::interpreter_error(index, err)),
                    )
                } else {
                    (gas_used, Err(PredicateVerificationFailed::False { index }))
                }
            }

            if vm.remaining_gas() != 0 {
                return (
                    gas_used,
                    Err(PredicateVerificationFailed::GasMismatch { index }),
                );
            }
        }

        (gas_used, Ok(()))
    }

    fn finalize_check_predicate<Tx>(
        mut kind: PredicateRunKind<Tx>,
        checks: Vec<(usize, Result<Word, PredicateVerificationFailed>)>,
        params: &CheckPredicateParams,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        if let PredicateRunKind::Estimating(tx) = &mut kind {
            checks.iter().for_each(|(input_index, result)| {
                if let Ok(gas_used) = result {
                    match &mut tx.inputs_mut()[*input_index] {
                        Input::CoinPredicate(CoinPredicate {
                            predicate_gas_used,
                            ..
                        })
                        | Input::MessageCoinPredicate(MessageCoinPredicate {
                            predicate_gas_used,
                            ..
                        })
                        | Input::MessageDataPredicate(MessageDataPredicate {
                            predicate_gas_used,
                            ..
                        }) => {
                            *predicate_gas_used = *gas_used;
                        }
                        _ => {
                            unreachable!(
                                "It was checked before during iteration over predicates"
                            )
                        }
                    }
                }
            });
        }

        let max_gas = kind.tx().max_gas(&params.gas_costs, &params.fee_params);
        if max_gas > params.max_gas_per_tx {
            return Err(
                PredicateVerificationFailed::TransactionExceedsTotalGasAllowance(max_gas),
            );
        }

        let mut cumulative_gas_used: u64 = 0;
        for (input_index, result) in checks {
            match result {
                Ok(gas_used) => {
                    cumulative_gas_used =
                        cumulative_gas_used.checked_add(gas_used).ok_or(
                            PredicateVerificationFailed::OutOfGas { index: input_index },
                        )?;
                }
                Err(failed) => {
                    return Err(failed);
                }
            }
        }

        Ok(PredicatesChecked {
            gas_used: cumulative_gas_used,
        })
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    fn deploy_inner(
        create: &mut Create,
        storage: &mut S,
        initial_balances: InitialBalances,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
        base_asset_id: &AssetId,
        gas_price: Word,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let metadata = create.metadata().as_ref();
        debug_assert!(
            metadata.is_some(),
            "`deploy_inner` is called without cached metadata"
        );
        let salt = create.salt();
        let storage_slots = create.storage_slots();
        let contract = Contract::try_from(&*create)?;
        let root = if let Some(m) = metadata {
            m.body.contract_root
        } else {
            contract.root()
        };

        let storage_root = if let Some(m) = metadata {
            m.body.state_root
        } else {
            Contract::initial_state_root(storage_slots.iter())
        };

        let id = if let Some(m) = metadata {
            m.body.contract_id
        } else {
            contract.id(salt, &root, &storage_root)
        };

        // Prevent redeployment of contracts
        if storage
            .storage_contract_exists(&id)
            .map_err(RuntimeError::Storage)?
        {
            return Err(InterpreterError::Panic(
                PanicReason::ContractIdAlreadyDeployed,
            ));
        }

        storage
            .deploy_contract_with_id(storage_slots, &contract, &id)
            .map_err(RuntimeError::Storage)?;
        Self::finalize_outputs(
            create,
            gas_costs,
            fee_params,
            base_asset_id,
            false,
            0,
            &initial_balances,
            &RuntimeBalances::try_from(initial_balances.clone())?,
            gas_price,
        )?;
        Ok(())
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    fn upgrade_inner(
        upgrade: &mut Upgrade,
        storage: &mut S,
        initial_balances: InitialBalances,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
        base_asset_id: &AssetId,
        gas_price: Word,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let metadata = upgrade.metadata().as_ref();
        debug_assert!(
            metadata.is_some(),
            "`upgrade_inner` is called without cached metadata"
        );

        match upgrade.upgrade_purpose() {
            UpgradePurpose::ConsensusParameters { .. } => {
                let consensus_parameters = if let Some(metadata) = metadata {
                    Self::get_consensus_parameters(&metadata.body)?
                } else {
                    let metadata = UpgradeMetadata::compute(upgrade)?;
                    Self::get_consensus_parameters(&metadata)?
                };

                let current_version = storage
                    .consensus_parameters_version()
                    .map_err(RuntimeError::Storage)?;
                let next_version = current_version.saturating_add(1);

                let prev = storage
                    .set_consensus_parameters(next_version, &consensus_parameters)
                    .map_err(RuntimeError::Storage)?;

                if prev.is_some() {
                    return Err(InterpreterError::Panic(
                        PanicReason::OverridingConsensusParameters,
                    ));
                }
            }
            UpgradePurpose::StateTransition { root } => {
                let exists = storage
                    .contains_state_transition_bytecode_root(root)
                    .map_err(RuntimeError::Storage)?;

                if !exists {
                    return Err(InterpreterError::Panic(
                        PanicReason::UnknownStateTransactionBytecodeRoot,
                    ))
                }

                let current_version = storage
                    .state_transition_version()
                    .map_err(RuntimeError::Storage)?;
                let next_version = current_version.saturating_add(1);

                let prev = storage
                    .set_state_transition_bytecode(next_version, root)
                    .map_err(RuntimeError::Storage)?;

                if prev.is_some() {
                    return Err(InterpreterError::Panic(
                        PanicReason::OverridingStateTransactionBytecode,
                    ));
                }
            }
        }

        Self::finalize_outputs(
            upgrade,
            gas_costs,
            fee_params,
            base_asset_id,
            false,
            0,
            &initial_balances,
            &RuntimeBalances::try_from(initial_balances.clone())?,
            gas_price,
        )?;
        Ok(())
    }

    fn get_consensus_parameters(
        metadata: &UpgradeMetadata,
    ) -> Result<ConsensusParameters, InterpreterError<S::DataError>> {
        match &metadata {
            UpgradeMetadata::ConsensusParameters {
                consensus_parameters,
                ..
            } => Ok(consensus_parameters.as_ref().clone()),
            UpgradeMetadata::StateTransition => {
                // It shouldn't be possible since `Check<Upgrade>` guarantees that.
                Err(InterpreterError::CheckError(CheckError::Validity(
                    ValidityError::TransactionMetadataMismatch,
                )))
            }
        }
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    fn upload_inner(
        upload: &mut Upload,
        storage: &mut S,
        initial_balances: InitialBalances,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
        base_asset_id: &AssetId,
        gas_price: Word,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let root = *upload.bytecode_root();
        let uploaded_bytecode = storage
            .storage_as_ref::<UploadedBytecodes>()
            .get(&root)
            .map_err(RuntimeError::Storage)?
            .map(|x| x.into_owned())
            .unwrap_or_else(|| UploadedBytecode::Uncompleted {
                bytecode: vec![],
                uploaded_subsections_number: 0,
            });

        let new_bytecode = match uploaded_bytecode {
            UploadedBytecode::Uncompleted {
                bytecode,
                uploaded_subsections_number,
            } => Self::upload_bytecode_subsection(
                upload,
                bytecode,
                uploaded_subsections_number,
            )?,
            UploadedBytecode::Completed(_) => {
                return Err(InterpreterError::Panic(
                    PanicReason::BytecodeAlreadyUploaded,
                ));
            }
        };

        storage
            .storage_as_mut::<UploadedBytecodes>()
            .insert(&root, &new_bytecode)
            .map_err(RuntimeError::Storage)?;

        Self::finalize_outputs(
            upload,
            gas_costs,
            fee_params,
            base_asset_id,
            false,
            0,
            &initial_balances,
            &RuntimeBalances::try_from(initial_balances.clone())?,
            gas_price,
        )?;
        Ok(())
    }

    fn upload_bytecode_subsection(
        upload: &Upload,
        mut uploaded_bytecode: Vec<u8>,
        uploaded_subsections_number: u16,
    ) -> Result<UploadedBytecode, InterpreterError<S::DataError>> {
        let index_of_next_subsection = uploaded_subsections_number;

        if *upload.subsection_index() != index_of_next_subsection {
            return Err(InterpreterError::Panic(
                PanicReason::ThePartIsNotSequentiallyConnected,
            ));
        }

        let bytecode_subsection = upload
            .witnesses()
            .get(*upload.bytecode_witness_index() as usize)
            .ok_or(InterpreterError::Bug(Bug::new(
                // It shouldn't be possible since `Checked<Upload>` guarantees
                // the existence of the witness.
                BugVariant::WitnessIndexOutOfBounds,
            )))?;

        uploaded_bytecode.extend(bytecode_subsection.as_ref());

        let new_uploaded_subsections_number = uploaded_subsections_number
            .checked_add(1)
            .ok_or(InterpreterError::Panic(PanicReason::ArithmeticOverflow))?;

        // It shouldn't be possible since `Checked<Upload>` guarantees
        // the validity of the Merkle proof.
        if new_uploaded_subsections_number > *upload.subsections_number() {
            return Err(InterpreterError::Bug(Bug::new(
                BugVariant::NextSubsectionIndexIsHigherThanTotalNumberOfParts,
            )))
        }

        let updated_uploaded_bytecode =
            if *upload.subsections_number() == new_uploaded_subsections_number {
                UploadedBytecode::Completed(uploaded_bytecode)
            } else {
                UploadedBytecode::Uncompleted {
                    bytecode: uploaded_bytecode,
                    uploaded_subsections_number: new_uploaded_subsections_number,
                }
            };

        Ok(updated_uploaded_bytecode)
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    fn blob_inner(
        blob: &mut Blob,
        storage: &mut S,
        initial_balances: InitialBalances,
        gas_costs: &GasCosts,
        fee_params: &FeeParameters,
        base_asset_id: &AssetId,
        gas_price: Word,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let blob_data = blob
            .witnesses()
            .get(*blob.bytecode_witness_index() as usize)
            .ok_or(InterpreterError::Bug(Bug::new(
                // It shouldn't be possible since `Checked<Blob>` guarantees
                // the existence of the witness.
                BugVariant::WitnessIndexOutOfBounds,
            )))?;

        let blob_id = blob.blob_id();

        debug_assert_eq!(
            BlobId::compute(blob_data.as_ref()),
            *blob_id,
            "Tx has invalid BlobId",
        );

        let old = storage
            .storage_as_mut::<BlobData>()
            .replace(blob_id, blob_data.as_ref())
            .map_err(RuntimeError::Storage)?;

        if old.is_some() {
            return Err(InterpreterError::Panic(PanicReason::BlobIdAlreadyUploaded));
        }

        Self::finalize_outputs(
            blob,
            gas_costs,
            fee_params,
            base_asset_id,
            false,
            0,
            &initial_balances,
            &RuntimeBalances::try_from(initial_balances.clone())?,
            gas_price,
        )?;

        Ok(())
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier,
{
    fn update_transaction_outputs(
        &mut self,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let outputs = self.transaction().outputs().len();
        (0..outputs).try_for_each(|o| self.update_memory_output(o))?;
        Ok(())
    }

    pub(crate) fn run(&mut self) -> Result<ProgramState, InterpreterError<S::DataError>> {
        for input in self.transaction().inputs() {
            if let Input::Contract(contract) = input {
                if !self.check_contract_exists(&contract.contract_id)? {
                    return Err(InterpreterError::Panic(
                        PanicReason::InputContractDoesNotExist,
                    ));
                }
            }
        }

        // TODO: Remove `Create`, `Upgrade`, and `Upload` from here
        //  https://github.com/FuelLabs/fuel-vm/issues/251
        let gas_costs = self.gas_costs().clone();
        let fee_params = *self.fee_params();
        let base_asset_id = *self.base_asset_id();
        let gas_price = self.gas_price();

        #[cfg(debug_assertions)]
        // The `match` statement exists to ensure that all variants of `Transaction`
        // are handled below. If a new variant is added, the compiler will
        // emit an error.
        {
            let mint: Transaction = Transaction::mint(
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            )
            .into();
            match mint {
                Transaction::Create(_) => {
                    // Handled in the `self.tx.as_create_mut()` branch.
                }
                Transaction::Upgrade(_) => {
                    // Handled in the `self.tx.as_upgrade_mut()` branch.
                }
                Transaction::Upload(_) => {
                    // Handled in the `self.tx.as_upload_mut()` branch.
                }
                Transaction::Blob(_) => {
                    // Handled in the `self.tx.as_blob_mut()` branch.
                }
                Transaction::Script(_) => {
                    // Handled in the `else` branch.
                }
                Transaction::Mint(_) => {
                    // The `Mint` transaction doesn't implement `ExecutableTransaction`.
                }
            };
        }

        let state = if let Some(create) = self.tx.as_create_mut() {
            Self::deploy_inner(
                create,
                &mut self.storage,
                self.initial_balances.clone(),
                &gas_costs,
                &fee_params,
                &base_asset_id,
                gas_price,
            )?;
            self.update_transaction_outputs()?;
            ProgramState::Return(1)
        } else if let Some(upgrade) = self.tx.as_upgrade_mut() {
            Self::upgrade_inner(
                upgrade,
                &mut self.storage,
                self.initial_balances.clone(),
                &gas_costs,
                &fee_params,
                &base_asset_id,
                gas_price,
            )?;
            self.update_transaction_outputs()?;
            ProgramState::Return(1)
        } else if let Some(upload) = self.tx.as_upload_mut() {
            Self::upload_inner(
                upload,
                &mut self.storage,
                self.initial_balances.clone(),
                &gas_costs,
                &fee_params,
                &base_asset_id,
                gas_price,
            )?;
            self.update_transaction_outputs()?;
            ProgramState::Return(1)
        } else if let Some(blob) = self.tx.as_blob_mut() {
            Self::blob_inner(
                blob,
                &mut self.storage,
                self.initial_balances.clone(),
                &gas_costs,
                &fee_params,
                &base_asset_id,
                gas_price,
            )?;
            self.update_transaction_outputs()?;
            ProgramState::Return(1)
        } else {
            // This must be a `Script`.
            self.run_program()?
        };

        Ok(state)
    }

    pub(crate) fn run_program(
        &mut self,
    ) -> Result<ProgramState, InterpreterError<S::DataError>> {
        let Some(script) = self.tx.as_script() else {
            unreachable!("Only `Script` transactions can be executed inside of the VM")
        };
        let gas_limit = *script.script_gas_limit();

        let (result, state) = if script.script().is_empty() {
            // Empty script is special-cased to simply return `1` as successful execution.
            let return_val = 1;
            self.ret(return_val)?;
            (
                ScriptExecutionResult::Success,
                ProgramState::Return(return_val),
            )
        } else {
            // TODO set tree balance
            loop {
                // Check whether the instruction will be executed in a call context
                let in_call = !self.frames.is_empty();

                match self.execute::<false>() {
                    // Proceeding with the execution normally
                    Ok(ExecuteState::Proceed) => continue,
                    // Debugger events are returned directly to the caller
                    Ok(ExecuteState::DebugEvent(d)) => {
                        self.debugger_set_last_state(ProgramState::RunProgram(d));
                        return Ok(ProgramState::RunProgram(d));
                    }
                    // Reverting terminated execution immediately
                    Ok(ExecuteState::Revert(r)) => {
                        break (ScriptExecutionResult::Revert, ProgramState::Revert(r))
                    }
                    // Returning in call context is ignored
                    Ok(ExecuteState::Return(_) | ExecuteState::ReturnData(_))
                        if in_call =>
                    {
                        continue
                    }
                    // In non-call context, returning terminates the execution
                    Ok(ExecuteState::Return(r)) => {
                        break (ScriptExecutionResult::Success, ProgramState::Return(r))
                    }
                    Ok(ExecuteState::ReturnData(d)) => {
                        break (
                            ScriptExecutionResult::Success,
                            ProgramState::ReturnData(d),
                        )
                    }
                    // Error always terminates the execution
                    Err(e) => match e.instruction_result() {
                        Some(result) => {
                            self.append_panic_receipt(result);
                            break (ScriptExecutionResult::Panic, ProgramState::Revert(0));
                        }
                        // This isn't a specified case of an erroneous program and should
                        // be propagated. If applicable, OS errors
                        // will fall into this category.
                        // The VM state is not finalized in this case.
                        None => return Err(e),
                    },
                }
            }
        };

        // Produce result receipt
        let gas_used = gas_limit
            .checked_sub(self.remaining_gas())
            .ok_or_else(|| Bug::new(BugVariant::GlobalGasUnderflow))?;
        self.receipts
            .push(Receipt::script_result(result, gas_used))?;

        // Finalize the outputs
        let fee_params = *self.fee_params();
        let base_asset_id = *self.base_asset_id();
        let gas_costs = self.gas_costs().clone();
        let gas_price = self.gas_price();
        Self::finalize_outputs(
            &mut self.tx,
            &gas_costs,
            &fee_params,
            &base_asset_id,
            matches!(state, ProgramState::Revert(_)),
            gas_used,
            &self.initial_balances,
            &self.balances,
            gas_price,
        )?;
        self.update_transaction_outputs()?;

        let Some(script) = self.tx.as_script_mut() else {
            unreachable!("This is checked to hold in the beginning of this function");
        };
        *script.receipts_root_mut() = self.receipts.root();

        Ok(state)
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
    Ecal: EcalHandler,
    V: Verifier,
{
    /// Initialize a pre-allocated instance of [`Interpreter`] with the provided
    /// transaction and execute it. The result will be bound to the lifetime
    /// of the interpreter and will avoid unnecessary copy with the data
    /// that can be referenced from the interpreter instance itself.
    pub fn transact(
        &mut self,
        tx: Ready<Tx>,
    ) -> Result<StateTransitionRef<'_, Tx>, InterpreterError<S::DataError>> {
        self.verify_ready_tx(&tx)?;

        let state_result = self.init_script(tx).and_then(|_| self.run());

        let state = state_result?;
        Ok(StateTransitionRef::new(
            state,
            self.transaction(),
            self.receipts(),
        ))
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    /// Deploys `Create` transaction without initialization VM and without invalidation of
    /// the last state of execution of the `Script` transaction.
    ///
    /// Returns `Create` transaction with all modifications after execution.
    pub fn deploy(
        &mut self,
        tx: Ready<Create>,
    ) -> Result<Create, InterpreterError<S::DataError>> {
        self.verify_ready_tx(&tx)?;

        let (_, checked) = tx.decompose();
        let (mut create, metadata): (Create, <Create as IntoChecked>::Metadata) =
            checked.into();
        let base_asset_id = *self.base_asset_id();
        let gas_price = self.gas_price();
        Self::deploy_inner(
            &mut create,
            &mut self.storage,
            metadata.balances(),
            &self.interpreter_params.gas_costs,
            &self.interpreter_params.fee_params,
            &base_asset_id,
            gas_price,
        )?;
        Ok(create)
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    /// Executes `Upgrade` transaction without initialization VM and without invalidation
    /// of the last state of execution of the `Script` transaction.
    ///
    /// Returns `Upgrade` transaction with all modifications after execution.
    pub fn upgrade(
        &mut self,
        tx: Ready<Upgrade>,
    ) -> Result<Upgrade, InterpreterError<S::DataError>> {
        self.verify_ready_tx(&tx)?;

        let (_, checked) = tx.decompose();
        let (mut upgrade, metadata): (Upgrade, <Upgrade as IntoChecked>::Metadata) =
            checked.into();
        let base_asset_id = *self.base_asset_id();
        let gas_price = self.gas_price();
        Self::upgrade_inner(
            &mut upgrade,
            &mut self.storage,
            metadata.balances(),
            &self.interpreter_params.gas_costs,
            &self.interpreter_params.fee_params,
            &base_asset_id,
            gas_price,
        )?;
        Ok(upgrade)
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    /// Executes `Upload` transaction without initialization VM and without invalidation
    /// of the last state of execution of the `Script` transaction.
    ///
    /// Returns `Upload` transaction with all modifications after execution.
    pub fn upload(
        &mut self,
        tx: Ready<Upload>,
    ) -> Result<Upload, InterpreterError<S::DataError>> {
        self.verify_ready_tx(&tx)?;

        let (_, checked) = tx.decompose();
        let (mut upload, metadata): (Upload, <Upload as IntoChecked>::Metadata) =
            checked.into();
        let base_asset_id = *self.base_asset_id();
        let gas_price = self.gas_price();
        Self::upload_inner(
            &mut upload,
            &mut self.storage,
            metadata.balances(),
            &self.interpreter_params.gas_costs,
            &self.interpreter_params.fee_params,
            &base_asset_id,
            gas_price,
        )?;
        Ok(upload)
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    S: InterpreterStorage,
{
    /// Executes `Blob` transaction without initialization VM and without invalidation
    /// of the last state of execution of the `Script` transaction.
    ///
    /// Returns `Blob` transaction with all modifications after execution.
    pub fn blob(
        &mut self,
        tx: Ready<Blob>,
    ) -> Result<Blob, InterpreterError<S::DataError>> {
        self.verify_ready_tx(&tx)?;

        let (_, checked) = tx.decompose();
        let (mut blob, metadata): (Blob, <Blob as IntoChecked>::Metadata) =
            checked.into();
        let base_asset_id = *self.base_asset_id();
        let gas_price = self.gas_price();
        Self::blob_inner(
            &mut blob,
            &mut self.storage,
            metadata.balances(),
            &self.interpreter_params.gas_costs,
            &self.interpreter_params.fee_params,
            &base_asset_id,
            gas_price,
        )?;
        Ok(blob)
    }
}

impl<M, S: InterpreterStorage, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V> {
    fn verify_ready_tx<Tx2: IntoChecked>(
        &self,
        tx: &Ready<Tx2>,
    ) -> Result<(), InterpreterError<S::DataError>> {
        self.gas_price_matches(tx)?;
        Ok(())
    }

    fn gas_price_matches<Tx2: IntoChecked>(
        &self,
        tx: &Ready<Tx2>,
    ) -> Result<(), InterpreterError<S::DataError>> {
        if tx.gas_price() != self.gas_price() {
            Err(InterpreterError::ReadyTransactionWrongGasPrice {
                expected: self.gas_price(),
                actual: tx.gas_price(),
            })
        } else {
            Ok(())
        }
    }
}
