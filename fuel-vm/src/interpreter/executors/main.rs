#[cfg(test)]
mod tests;

use crate::{
    checked_transaction::{
        Checked,
        IntoChecked,
        ParallelExecutor,
    },
    consts::*,
    context::Context,
    error::{
        Bug,
        BugId,
        BugVariant,
        InterpreterError,
        PredicateVerificationFailed,
    },
    gas::GasCosts,
    interpreter::{
        CheckedMetadata,
        ExecutableTransaction,
        InitialBalances,
        Interpreter,
        RuntimeBalances,
    },
    predicate::RuntimePredicate,
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

use crate::error::BugVariant::GlobalGasUnderflow;
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
    ConsensusParameters,
    Contract,
    Create,
    Input,
    Receipt,
    ScriptExecutionResult,
};
use fuel_types::Word;

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

impl<T> Interpreter<PredicateStorage, T> {
    /// Initialize the VM with the provided transaction and check all predicates defined
    /// in the inputs.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub fn check_predicates<Tx>(
        checked: &Checked<Tx>,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
    {
        let tx = checked.transaction();
        let balances = checked.metadata().balances();
        Self::run_predicate(PredicateRunKind::Verifying(tx), balances, params, gas_costs)
    }

    /// Initialize the VM with the provided transaction and check all predicates defined
    /// in the inputs in parallel.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for
    /// predicates.
    pub async fn check_predicates_async<Tx, E>(
        checked: &Checked<Tx>,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
        E: ParallelExecutor
            + ParallelExecutor<TaskResult = Result<Word, PredicateVerificationFailed>>,
    {
        let tx = checked.transaction();
        let balances = checked.metadata().balances();

        let predicates_checked =
            Self::verify_predicate_async::<Tx, E>(tx, balances, params, gas_costs)
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
        balances: InitialBalances,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<(), PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        Self::run_predicate(
            PredicateRunKind::Estimating(transaction),
            balances,
            params,
            gas_costs,
        )?;
        Ok(())
    }

    async fn verify_predicate_async<Tx, E>(
        tx: &Tx,
        balances: InitialBalances,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction + Send + 'static,
        E: ParallelExecutor
            + ParallelExecutor<TaskResult = Result<Word, PredicateVerificationFailed>>,
    {
        if !tx.check_predicate_owners(&params.chain_id) {
            return Err(PredicateVerificationFailed::InvalidOwner)
        }

        let mut verifications = vec![];

        for index in 0..tx.inputs().len() {
            let is_predicate = matches!(
                tx.inputs()[index],
                Input::CoinPredicate(_)
                    | Input::MessageCoinPredicate(_)
                    | Input::MessageDataPredicate(_)
            );

            if !is_predicate {
                continue
            }

            let tx = tx.clone();

            if let Some(predicate) = RuntimePredicate::from_tx(&params, &tx, index) {
                let gas_costs = gas_costs.clone();
                let balances = balances.clone();

                let verify_task = E::create_task(move || {
                    let mut vm = Interpreter::with_storage(
                        PredicateStorage::default(),
                        params,
                        gas_costs,
                    );

                    let context = Context::PredicateVerification { program: predicate };

                    let available_gas =
                        if let Some(x) = tx.inputs()[index].predicate_gas_used() {
                            x
                        } else {
                            return Err(PredicateVerificationFailed::GasNotSpecified)
                        };

                    vm.init_predicate(context, &tx, balances.clone(), available_gas)?;

                    let result = vm.verify_predicate();
                    let is_successful = matches!(result, Ok(ProgramState::Return(0x01)));

                    let gas_used = available_gas
                        .checked_sub(vm.remaining_gas())
                        .ok_or_else(|| Bug::new(BugId::ID004, GlobalGasUnderflow))?;

                    if !is_successful {
                        result?;
                        return Err(PredicateVerificationFailed::False)
                    }

                    if vm.remaining_gas() != 0 {
                        return Err(PredicateVerificationFailed::GasMismatch)
                    }

                    Ok(gas_used)
                });

                verifications.push(verify_task);
            }
        }

        let verifications = E::execute_tasks(verifications).await;
        let cumulative_gas_used = verifications
            .into_iter()
            .try_fold(0, |acc, x| Ok::<u64, PredicateVerificationFailed>(acc + x?))?;

        if cumulative_gas_used > tx.limit() {
            return Err(
                PredicateVerificationFailed::CumulativePredicateGasExceededTxGasLimit,
            )
        }

        Ok(PredicatesChecked {
            gas_used: cumulative_gas_used,
        })
    }

    fn run_predicate<Tx>(
        mut kind: PredicateRunKind<Tx>,
        balances: InitialBalances,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
    {
        if !kind.tx().check_predicate_owners(&params.chain_id) {
            return Err(PredicateVerificationFailed::InvalidOwner)
        }

        let mut cumulative_gas_used: Word = 0;

        for i in 0..kind.tx().inputs().len() {
            let is_predicate = matches!(
                kind.tx().inputs()[i],
                Input::CoinPredicate(_)
                    | Input::MessageCoinPredicate(_)
                    | Input::MessageDataPredicate(_)
            );

            if !is_predicate {
                continue
            }

            if let Some(predicate) = RuntimePredicate::from_tx(&params, kind.tx(), i) {
                let mut vm = Interpreter::with_storage(
                    PredicateStorage::default(),
                    params,
                    gas_costs.clone(),
                );

                let available_gas = match &kind {
                    PredicateRunKind::Verifying(tx) => {
                        let context =
                            Context::PredicateVerification { program: predicate };

                        let available_gas =
                            if let Some(x) = kind.tx().inputs()[i].predicate_gas_used() {
                                x
                            } else {
                                return Err(PredicateVerificationFailed::GasNotSpecified)
                            };

                        vm.init_predicate(context, *tx, balances.clone(), available_gas)?;
                        available_gas
                    }
                    PredicateRunKind::Estimating(tx) => {
                        let context = Context::PredicateEstimation { program: predicate };
                        let tx_available_gas = params
                            .max_gas_per_tx
                            .checked_sub(cumulative_gas_used)
                            .ok_or_else(|| Bug::new(BugId::ID003, GlobalGasUnderflow))?;
                        let available_gas = core::cmp::min(
                            params.max_gas_per_predicate,
                            tx_available_gas,
                        );

                        vm.init_predicate(context, *tx, balances.clone(), available_gas)?;
                        available_gas
                    }
                };

                let result = vm.verify_predicate();
                let is_successful = matches!(result, Ok(ProgramState::Return(0x01)));

                let gas_used = available_gas
                    .checked_sub(vm.remaining_gas())
                    .ok_or_else(|| Bug::new(BugId::ID004, GlobalGasUnderflow))?;
                cumulative_gas_used = cumulative_gas_used
                    .checked_add(gas_used)
                    .ok_or_else(|| PredicateVerificationFailed::OutOfGas)?;

                match &mut kind {
                    PredicateRunKind::Verifying(_) => {
                        if !is_successful {
                            result?;
                            return Err(PredicateVerificationFailed::False)
                        }

                        if vm.remaining_gas() != 0 {
                            return Err(PredicateVerificationFailed::GasMismatch)
                        }
                    }
                    PredicateRunKind::Estimating(tx) => {
                        match &mut tx.inputs_mut()[i] {
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
                                *predicate_gas_used = gas_used;
                            }
                            _ => {
                                unreachable!("It was checked before during iteration over predicates")
                            }
                        }
                    }
                }
            }
        }

        match kind {
            PredicateRunKind::Verifying(tx) => {
                if cumulative_gas_used > tx.limit() {
                    return Err(PredicateVerificationFailed::CumulativePredicateGasExceededTxGasLimit);
                }
            }
            PredicateRunKind::Estimating(_) => {
                if cumulative_gas_used > params.max_gas_per_tx {
                    return Err(PredicateVerificationFailed::CumulativePredicateGasExceededTxGasLimit);
                }
            }
        }

        Ok(PredicatesChecked {
            gas_used: cumulative_gas_used,
        })
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
{
    fn deploy_inner(
        create: &mut Create,
        storage: &mut S,
        initial_balances: InitialBalances,
        params: &ConsensusParameters,
    ) -> Result<(), InterpreterError> {
        let remaining_gas = create
            .limit()
            .checked_sub(create.gas_used_by_predicates())
            .ok_or_else(|| InterpreterError::Panic(PanicReason::OutOfGas))?;

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
            .map_err(InterpreterError::from_io)?
        {
            return Err(InterpreterError::Panic(
                PanicReason::ContractIdAlreadyDeployed,
            ))
        }

        storage
            .deploy_contract_with_id(salt, storage_slots, &contract, &root, &id)
            .map_err(InterpreterError::from_io)?;
        Self::finalize_outputs(
            create,
            false,
            remaining_gas,
            &initial_balances,
            &RuntimeBalances::try_from(initial_balances.clone())?,
            params,
        )?;
        Ok(())
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    fn update_transaction_outputs(&mut self) -> Result<(), InterpreterError> {
        let outputs = self.transaction().outputs().len();
        (0..outputs).try_for_each(|o| self.update_memory_output(o))?;
        Ok(())
    }

    pub(crate) fn run(&mut self) -> Result<ProgramState, InterpreterError> {
        // TODO: Remove `Create` from here
        let state = if let Some(create) = self.tx.as_create_mut() {
            Self::deploy_inner(
                create,
                &mut self.storage,
                self.initial_balances.clone(),
                &self.params,
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
                .ok_or_else(|| Bug::new(BugId::ID002, BugVariant::GlobalGasUnderflow))?;

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

            #[cfg(feature = "debug")]
            if program.is_debug() {
                self.debugger_set_last_state(program);
            }

            if let Some(script) = self.tx.as_script_mut() {
                let receipts_root = self.receipts.root();
                *script.receipts_root_mut() = receipts_root;
            }

            let revert = matches!(program, ProgramState::Revert(_));
            let remaining_gas = self.remaining_gas();
            Self::finalize_outputs(
                &mut self.tx,
                revert,
                remaining_gas,
                &self.initial_balances,
                &self.balances,
                &self.params,
            )?;
            self.update_transaction_outputs()?;

            program
        };

        Ok(state)
    }

    pub(crate) fn run_program(&mut self) -> Result<ProgramState, InterpreterError> {
        loop {
            if self.registers[RegId::PC] >= VM_MAX_RAM {
                return Err(InterpreterError::Panic(PanicReason::MemoryOverflow))
            }

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

                    #[cfg(feature = "debug")]
                    ExecuteState::DebugEvent(d) => return Ok(ProgramState::RunProgram(d)),
                }
            }
        }
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::Metadata: CheckedMetadata,
{
    /// Allocate internally a new instance of [`Interpreter`] with the provided
    /// storage, initialize it with the provided transaction and return the
    /// result of th execution in form of [`StateTransition`]
    pub fn transact_owned(
        storage: S,
        tx: Checked<Tx>,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<StateTransition<Tx>, InterpreterError> {
        let mut interpreter = Interpreter::with_storage(storage, params, gas_costs);
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
    ) -> Result<StateTransitionRef<'_, Tx>, InterpreterError> {
        let state_result = self.init_script(tx).and_then(|_| self.run());

        #[cfg(feature = "profile-any")]
        self.profiler.on_transaction(&state_result);

        let state = state_result?;
        Ok(StateTransitionRef::new(
            state,
            self.transaction(),
            self.receipts(),
        ))
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
{
    /// Deploys `Create` transaction without initialization VM and without invalidation of
    /// the last state of execution of the `Script` transaction.
    ///
    /// Returns `Create` transaction with all modifications after execution.
    pub fn deploy(&mut self, tx: Checked<Create>) -> Result<Create, InterpreterError> {
        let (mut create, metadata) = tx.into();
        Self::deploy_inner(
            &mut create,
            &mut self.storage,
            metadata.balances(),
            &self.params,
        )?;
        Ok(create)
    }
}
