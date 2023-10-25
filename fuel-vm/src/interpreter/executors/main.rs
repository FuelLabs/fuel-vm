#[cfg(test)]
mod tests;

use alloc::{
    vec,
    vec::Vec,
};

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
        PredicateErrorEcal,
        RuntimeBalances,
    },
    predicate::RuntimePredicate,
    prelude::{
        BugVariant,
        RuntimeError,
    },
    state::{
        ExecuteState,
        ProgramState,
        StateTransition,
        StateTransitionRef,
    },
    storage::{
        InterpreterStorage,
        PredicateStorage,
    },
};

use crate::{
    checked_transaction::CheckPredicateParams,
    interpreter::InterpreterParams,
};
use fuel_asm::{
    PanicReason,
    RegId,
};
use fuel_tx::{
    field::{
        ReceiptsRoot,
        Salt,
        Script as ScriptField,
        StorageSlots,
    },
    input::{
        coin::CoinPredicate,
        message::{
            MessageCoinPredicate,
            MessageDataPredicate,
        },
    },
    Chargeable,
    Contract,
    Create,
    FeeParameters,
    Input,
    Receipt,
    ScriptExecutionResult,
};
use fuel_types::{
    AssetId,
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

impl<'a, Tx> PredicateRunKind<'a, Tx> {
    fn tx(&self) -> &Tx {
        match self {
            PredicateRunKind::Verifying(tx) => tx,
            PredicateRunKind::Estimating(tx) => tx,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PredicateAction {
    Verifying,
    Estimating,
}

impl<Tx> From<&PredicateRunKind<'_, Tx>> for PredicateAction {
    fn from(kind: &PredicateRunKind<'_, Tx>) -> Self {
        match kind {
            PredicateRunKind::Verifying(_) => PredicateAction::Verifying,
            PredicateRunKind::Estimating(_) => PredicateAction::Estimating,
        }
    }
}

impl<T> Interpreter<PredicateStorage, PredicateErrorEcal, T> {
    /// Initialize the VM with the provided transaction and check all predicates defined
    /// in the inputs.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub fn check_predicates<Tx>(
        checked: &Checked<Tx>,
        params: &CheckPredicateParams,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
    {
        let tx = checked.transaction();
        Self::run_predicate(PredicateRunKind::Verifying(tx), params)
    }

    /// Initialize the VM with the provided transaction and check all predicates defined
    /// in the inputs in parallel.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub async fn check_predicates_async<Tx, E>(
        checked: &Checked<Tx>,
        params: &CheckPredicateParams,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
        E: ParallelExecutor,
    {
        let tx = checked.transaction();

        let predicates_checked =
            Self::run_predicate_async::<Tx, E>(PredicateRunKind::Verifying(tx), params)
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
    ) -> Result<(), PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        Self::run_predicate(PredicateRunKind::Estimating(transaction), params)?;
        Ok(())
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
    ) -> Result<(), PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        E: ParallelExecutor,
    {
        Self::run_predicate_async::<Tx, E>(
            PredicateRunKind::Estimating(transaction),
            params,
        )
        .await?;

        Ok(())
    }

    async fn run_predicate_async<Tx, E>(
        kind: PredicateRunKind<'_, Tx>,
        params: &CheckPredicateParams,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        E: ParallelExecutor,
    {
        let mut checks = vec![];
        let predicate_action = PredicateAction::from(&kind);
        let max_gas_per_tx = params.max_gas_per_tx;
        let tx_offset = params.tx_offset;

        for index in 0..kind.tx().inputs().len() {
            if let Some(predicate) =
                RuntimePredicate::from_tx(kind.tx(), tx_offset, index)
            {
                let tx = kind.tx().clone();
                let my_params = params.clone();

                let verify_task = E::create_task(move || {
                    Self::check_predicate(
                        tx,
                        index,
                        predicate_action,
                        predicate,
                        my_params,
                    )
                });

                checks.push(verify_task);
            }
        }

        let checks = E::execute_tasks(checks).await;

        Self::finalize_check_predicate(kind, checks, predicate_action, max_gas_per_tx)
    }

    fn run_predicate<Tx>(
        kind: PredicateRunKind<'_, Tx>,
        params: &CheckPredicateParams,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        let predicate_action = PredicateAction::from(&kind);
        let mut checks = vec![];

        for index in 0..kind.tx().inputs().len() {
            let tx = kind.tx().clone();

            if let Some(predicate) =
                RuntimePredicate::from_tx(&tx, params.tx_offset, index)
            {
                checks.push(Self::check_predicate(
                    tx,
                    index,
                    predicate_action,
                    predicate,
                    params.clone(),
                ));
            }
        }

        Self::finalize_check_predicate(
            kind,
            checks,
            predicate_action,
            params.max_gas_per_tx,
        )
    }

    fn check_predicate<Tx>(
        tx: Tx,
        index: usize,
        predicate_action: PredicateAction,
        predicate: RuntimePredicate,
        params: CheckPredicateParams,
    ) -> Result<(Word, usize), PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
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
                if !Input::is_predicate_owner_valid(address, predicate) {
                    return Err(PredicateVerificationFailed::InvalidOwner)
                }
            }
            _ => {}
        }

        let max_gas_per_tx = params.max_gas_per_tx;
        let max_gas_per_predicate = params.max_gas_per_predicate;
        let interpreter_params = params.into();

        let mut vm = Interpreter::<_, PredicateErrorEcal, _>::with_storage(
            PredicateStorage {},
            interpreter_params,
        );

        let available_gas = match predicate_action {
            PredicateAction::Verifying => {
                let context = Context::PredicateVerification { program: predicate };
                let available_gas =
                    if let Some(x) = tx.inputs()[index].predicate_gas_used() {
                        x
                    } else {
                        return Err(PredicateVerificationFailed::GasNotSpecified)
                    };

                vm.init_predicate(context, tx, available_gas)?;
                available_gas
            }
            PredicateAction::Estimating => {
                let context = Context::PredicateEstimation { program: predicate };
                let available_gas = core::cmp::min(max_gas_per_predicate, max_gas_per_tx);

                vm.init_predicate(context, tx, available_gas)?;
                available_gas
            }
        };

        let result = vm.verify_predicate();
        let is_successful = matches!(result, Ok(ProgramState::Return(0x01)));

        let gas_used = available_gas
            .checked_sub(vm.remaining_gas())
            .ok_or_else(|| Bug::new(BugVariant::GlobalGasUnderflow))?;

        if let PredicateAction::Verifying = predicate_action {
            if !is_successful {
                result?;
                return Err(PredicateVerificationFailed::False)
            }

            if vm.remaining_gas() != 0 {
                return Err(PredicateVerificationFailed::GasMismatch)
            }
        }

        Ok((gas_used, index))
    }

    fn finalize_check_predicate<Tx>(
        mut kind: PredicateRunKind<Tx>,
        checks: Vec<Result<(Word, usize), PredicateVerificationFailed>>,
        predicate_action: PredicateAction,
        max_gas_per_tx: u64,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        if let PredicateRunKind::Estimating(tx) = &mut kind {
            checks.iter().for_each(|result| {
                if let Ok((gas_used, index)) = result {
                    match &mut tx.inputs_mut()[*index] {
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
            })
        }

        let cumulative_gas_used = checks.into_iter().try_fold(0u64, |acc, result| {
            acc.checked_add(result.map(|(gas_used, _)| gas_used)?)
                .ok_or(PredicateVerificationFailed::OutOfGas)
        })?;

        match predicate_action {
            PredicateAction::Verifying => {
                if cumulative_gas_used > kind.tx().limit() {
                    return Err(PredicateVerificationFailed::CumulativePredicateGasExceededTxGasLimit);
                }
            }
            PredicateAction::Estimating => {
                if cumulative_gas_used > max_gas_per_tx {
                    return Err(PredicateVerificationFailed::CumulativePredicateGasExceededTxGasLimit);
                }
            }
        }

        Ok(PredicatesChecked {
            gas_used: cumulative_gas_used,
        })
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    S: InterpreterStorage,
{
    fn deploy_inner(
        create: &mut Create,
        storage: &mut S,
        initial_balances: InitialBalances,
        fee_params: &FeeParameters,
        base_asset_id: &AssetId,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let remaining_gas = create
            .limit()
            .checked_sub(create.gas_used_by_predicates())
            .ok_or(InterpreterError::Panic(PanicReason::OutOfGas))?;

        let metadata = create.metadata().as_ref();
        debug_assert!(
            metadata.is_some(),
            "`deploy_inner` is called without cached metadata"
        );
        let salt = create.salt();
        let storage_slots = create.storage_slots();
        let contract = Contract::try_from(&*create)?;
        let root = if let Some(m) = metadata {
            m.contract_root
        } else {
            contract.root()
        };

        let storage_root = if let Some(m) = metadata {
            m.state_root
        } else {
            Contract::initial_state_root(storage_slots.iter())
        };

        let id = if let Some(m) = metadata {
            m.contract_id
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
            ))
        }

        storage
            .deploy_contract_with_id(salt, storage_slots, &contract, &root, &id)
            .map_err(RuntimeError::Storage)?;
        Self::finalize_outputs(
            create,
            fee_params,
            base_asset_id,
            false,
            remaining_gas,
            &initial_balances,
            &RuntimeBalances::try_from(initial_balances.clone())?,
        )?;
        Ok(())
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    fn update_transaction_outputs(
        &mut self,
    ) -> Result<(), InterpreterError<S::DataError>> {
        let outputs = self.transaction().outputs().len();
        (0..outputs).try_for_each(|o| self.update_memory_output(o))?;
        Ok(())
    }

    pub(crate) fn run(&mut self) -> Result<ProgramState, InterpreterError<S::DataError>> {
        // TODO: Remove `Create` from here
        let fee_params = *self.fee_params();
        let base_asset_id = *self.base_asset_id();
        let state = if let Some(create) = self.tx.as_create_mut() {
            Self::deploy_inner(
                create,
                &mut self.storage,
                self.initial_balances.clone(),
                &fee_params,
                &base_asset_id,
            )?;
            self.update_transaction_outputs()?;
            ProgramState::Return(1)
        } else {
            if self.transaction().inputs().iter().any(|input| {
                if let Input::Contract(contract) = input {
                    !self
                        .check_contract_exists(&contract.contract_id)
                        .unwrap_or(false)
                } else {
                    false
                }
            }) {
                return Err(InterpreterError::Panic(PanicReason::ContractNotFound))
            }

            if let Some(script) = self.transaction().as_script() {
                let offset = (self.tx_offset() + script.script_offset()) as Word;

                self.registers[RegId::PC] = offset;
                self.registers[RegId::IS] = offset;
            }

            // TODO set tree balance

            // `Interpreter` supports only `Create` and `Script` transactions. It is not
            // `Create` -> it is `Script`.
            let program = if !self
                .transaction()
                .as_script()
                .expect("It should be `Script` transaction")
                .script()
                .is_empty()
            {
                self.run_program()
            } else {
                // Return `1` as successful execution.
                let return_val = 1;
                self.ret(return_val)?;
                Ok(ProgramState::Return(return_val))
            };

            let gas_used = self
                .transaction()
                .limit()
                .checked_sub(self.remaining_gas())
                .ok_or_else(|| Bug::new(BugVariant::GlobalGasUnderflow))?;

            // Catch VM panic and don't propagate, generating a receipt
            let (status, program) = match program {
                Ok(s) => {
                    // either a revert or success
                    let res = if let ProgramState::Revert(_) = &s {
                        ScriptExecutionResult::Revert
                    } else {
                        ScriptExecutionResult::Success
                    };
                    (res, s)
                }

                Err(e) => match e.instruction_result() {
                    Some(result) => {
                        self.append_panic_receipt(result);

                        (ScriptExecutionResult::Panic, ProgramState::Revert(0))
                    }

                    // This isn't a specified case of an erroneous program and should be
                    // propagated. If applicable, OS errors will fall into this category.
                    None => return Err(e),
                },
            };

            let receipt = Receipt::script_result(status, gas_used);

            self.append_receipt(receipt);

            if program.is_debug() {
                self.debugger_set_last_state(program);
            }

            if let Some(script) = self.tx.as_script_mut() {
                let receipts_root = self.receipts.root();
                *script.receipts_root_mut() = receipts_root;
            }

            let revert = matches!(program, ProgramState::Revert(_));
            let remaining_gas = self.remaining_gas();
            let fee_params = *self.fee_params();
            Self::finalize_outputs(
                &mut self.tx,
                &fee_params,
                &base_asset_id,
                revert,
                remaining_gas,
                &self.initial_balances,
                &self.balances,
            )?;
            self.update_transaction_outputs()?;

            program
        };

        Ok(state)
    }

    pub(crate) fn run_program(
        &mut self,
    ) -> Result<ProgramState, InterpreterError<S::DataError>> {
        loop {
            // Check whether the instruction will be executed in a call context
            let in_call = !self.frames.is_empty();

            let state = self.execute()?;

            if in_call {
                // Only reverts should terminate execution from a call context
                if let ExecuteState::Revert(r) = state {
                    return Ok(ProgramState::Revert(r))
                }
            } else {
                match state {
                    ExecuteState::Return(r) => return Ok(ProgramState::Return(r)),

                    ExecuteState::ReturnData(d) => return Ok(ProgramState::ReturnData(d)),

                    ExecuteState::Revert(r) => return Ok(ProgramState::Revert(r)),

                    ExecuteState::Proceed => (),

                    ExecuteState::DebugEvent(d) => return Ok(ProgramState::RunProgram(d)),
                }
            }
        }
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
    Ecal: EcalHandler,
{
    /// Allocate internally a new instance of [`Interpreter`] with the provided
    /// storage, initialize it with the provided transaction and return the
    /// result of th execution in form of [`StateTransition`]
    pub fn transact_owned(
        storage: S,
        tx: Checked<Tx>,
        params: InterpreterParams,
    ) -> Result<StateTransition<Tx>, InterpreterError<S::DataError>> {
        let mut interpreter: Self =
            Interpreter::<_, Ecal, _>::with_storage(storage, params).with_ecal();
        interpreter
            .transact(tx)
            .map(ProgramState::from)
            .map(|state| {
                StateTransition::new(state, interpreter.tx, interpreter.receipts.into())
            })
    }

    /// Initialize a pre-allocated instance of [`Interpreter`] with the provided
    /// transaction and execute it. The result will be bound to the lifetime
    /// of the interpreter and will avoid unnecessary copy with the data
    /// that can be referenced from the interpreter instance itself.
    pub fn transact(
        &mut self,
        tx: Checked<Tx>,
    ) -> Result<StateTransitionRef<'_, Tx>, InterpreterError<S::DataError>> {
        let state_result = self.init_script(tx).and_then(|_| self.run());

        #[cfg(feature = "profile-any")]
        {
            let r = match &state_result {
                Ok(state) => Ok(state),
                Err(err) => Err(err.erase_generics()),
            };
            self.profiler.on_transaction(r);
        }

        let state = state_result?;
        Ok(StateTransitionRef::new(
            state,
            self.transaction(),
            self.receipts(),
        ))
    }
}

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    S: InterpreterStorage,
{
    /// Deploys `Create` transaction without initialization VM and without invalidation of
    /// the last state of execution of the `Script` transaction.
    ///
    /// Returns `Create` transaction with all modifications after execution.
    pub fn deploy(
        &mut self,
        tx: Checked<Create>,
    ) -> Result<Create, InterpreterError<S::DataError>> {
        let (mut create, metadata) = tx.into();
        let fee_params = *self.fee_params();
        let base_asset_id = *self.base_asset_id();
        Self::deploy_inner(
            &mut create,
            &mut self.storage,
            metadata.balances(),
            &fee_params,
            &base_asset_id,
        )?;
        Ok(create)
    }
}
