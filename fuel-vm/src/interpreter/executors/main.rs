use crate::checked_transaction::{Checked, IntoChecked};
use crate::consts::*;
use crate::context::Context;
use crate::error::{Bug, BugId, BugVariant, InterpreterError, PredicateVerificationFailed};
use crate::gas::GasCosts;
use crate::interpreter::{
    CheckedMetadata, ExecutableTransaction, InitialBalances, Interpreter, RuntimeBalances,
};
use crate::predicate::RuntimePredicate;
use crate::state::{ExecuteState, ProgramState};
use crate::state::{StateTransition, StateTransitionRef};
use crate::storage::{InterpreterStorage, PredicateStorage};

use crate::error::BugVariant::GlobalGasUnderflow;
use fuel_asm::{PanicReason, RegId};
use fuel_tx::input::coin::CoinPredicate;
use fuel_tx::input::message::{MessageCoinPredicate, MessageDataPredicate};
use fuel_tx::{
    field::{Outputs, ReceiptsRoot, Salt, Script as ScriptField, StorageSlots},
    Chargeable, ConsensusParameters, Contract, Create, Input, Output, Receipt, ScriptExecutionResult,
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

// FIXME replace for a type-safe transaction
impl<T> Interpreter<PredicateStorage, T> {
    /// Initialize the VM with the provided transaction and check all predicates defined in the
    /// inputs.
    ///
    /// The storage provider is not used since contract opcodes are not allowed for predicates.
    /// This way, its possible, for the sake of simplicity, it is possible to use
    /// [unit](https://doc.rust-lang.org/core/primitive.unit.html) as storage provider.
    ///
    /// # Debug
    ///
    /// This is not a valid entrypoint for debug calls. It will only return a `bool`, and not the
    /// VM state required to trace the execution steps.
    pub fn check_predicates<Tx>(
        checked: &mut Checked<Tx>,
        params: ConsensusParameters,
        gas_costs: GasCosts,
        malleable_gas: bool,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
        <Tx as IntoChecked>::CheckedMetadata: CheckedMetadata,
    {
        if !checked.transaction().check_predicate_owners(&params) {
            return Err(PredicateVerificationFailed::InvalidOwner);
        }

        let mut vm = Interpreter::with_storage(PredicateStorage::default(), params, gas_costs);

        let mut cumulative_gas_used: Word = 0;

        vm.init_predicate(checked.clone());

        if malleable_gas {
            let tx_gas_limit: u64 = checked.clone().transaction().limit();

            let predicate_gas_limit: u64 = if tx_gas_limit > params.max_gas_per_predicate {
                params.max_gas_per_predicate
            } else {
                tx_gas_limit
            };

            let checked_clone = checked.clone();

            for (idx, input) in checked.transaction_mut().inputs_mut().iter_mut().enumerate() {
                if let Some(predicate) = RuntimePredicate::from_tx(&params, checked_clone.transaction(), idx) {
                    vm.init_predicate_estimation(checked_clone.clone());
                    vm.context = Context::PredicateEstimation { program: predicate };
                    vm.set_gas(predicate_gas_limit);

                    if !matches!(vm.verify_predicate()?, ProgramState::Return(0x01)) {
                        return Err(PredicateVerificationFailed::False);
                    }

                    let gas_used: u64 = tx_gas_limit
                        .checked_sub(vm.remaining_gas())
                        .ok_or_else(|| Bug::new(BugId::ID004, GlobalGasUnderflow))?;

                    cumulative_gas_used += gas_used;

                    match input {
                        Input::CoinPredicate(CoinPredicate { predicate_gas_used, .. })
                        | Input::MessageCoinPredicate(MessageCoinPredicate { predicate_gas_used, .. })
                        | Input::MessageDataPredicate(MessageDataPredicate { predicate_gas_used, .. }) => {
                            *predicate_gas_used = gas_used;
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Needed for now because checked is only freed once the value is collected into a Vec
            #[allow(clippy::needless_collect)]
            let predicates: Vec<_> = (0..checked.transaction().inputs().len())
                .filter_map(|i| RuntimePredicate::from_tx(&params, checked.transaction(), i))
                .collect();

            for predicate in predicates {
                // VM is cloned because the state should be reset for every predicate verification
                let mut vm = vm.clone();

                let gas_used = predicate.gas_used();
                vm.context = Context::PredicateVerification { program: predicate };
                vm.set_gas(gas_used);

                if !matches!(vm.verify_predicate()?, ProgramState::Return(0x01)) {
                    return Err(PredicateVerificationFailed::False);
                }

                if vm.registers[RegId::GGAS] != 0 {
                    return Err(PredicateVerificationFailed::GasMismatch);
                }
                cumulative_gas_used = cumulative_gas_used.checked_add(gas_used).expect("cumulative gas overflow");
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
    fn _deploy(
        create: &mut Create,
        storage: &mut S,
        initial_balances: InitialBalances,
        params: &ConsensusParameters,
    ) -> Result<(), InterpreterError> {
        let salt = create.salt();
        let storage_slots = create.storage_slots();
        let contract = Contract::try_from(&*create)?;
        let root = contract.root();
        let storage_root = Contract::initial_state_root(storage_slots.iter());
        let id = contract.id(salt, &root, &storage_root);

        // TODO: Move this check to `fuel-tx`.
        if !create
            .outputs()
            .iter()
            .any(|output| matches!(output, Output::ContractCreated { contract_id, state_root } if contract_id == &id && state_root == &storage_root))
        {
            return Err(InterpreterError::Panic(PanicReason::ContractNotInInputs));
        }

        // Prevent redeployment of contracts
        if storage
            .storage_contract_exists(&id)
            .map_err(InterpreterError::from_io)?
        {
            return Err(InterpreterError::Panic(PanicReason::ContractIdAlreadyDeployed));
        }

        storage
            .deploy_contract_with_id(salt, storage_slots, &contract, &root, &id)
            .map_err(InterpreterError::from_io)?;

        let remaining_gas = create.limit();
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
            Self::_deploy(create, &mut self.storage, self.initial_balances.clone(), &self.params)?;
            self.update_transaction_outputs()?;
            ProgramState::Return(1)
        } else {
            if self.transaction().inputs().iter().any(|input| {
                if let Input::Contract(contract) = input {
                    !self.check_contract_exists(&contract.contract_id).unwrap_or(false)
                } else {
                    false
                }
            }) {
                return Err(InterpreterError::Panic(PanicReason::ContractNotFound));
            }

            if let Some(script) = self.transaction().as_script() {
                let offset = (self.tx_offset() + script.script_offset()) as Word;

                self.registers[RegId::PC] = offset;
                self.registers[RegId::IS] = offset;
            }

            // TODO set tree balance

            // `Interpreter` supports only `Create` and `Script` transactions. It is not `Create` ->
            // it is `Script`.
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
                    None => {
                        return Err(e);
                    }
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
                return Err(InterpreterError::Panic(PanicReason::MemoryOverflow));
            }

            // Check whether the instruction will be executed in a call context
            let in_call = !self.frames.is_empty();

            let state = self.execute()?;

            if in_call {
                // Only reverts should terminate execution from a call context
                if let ExecuteState::Revert(r) = state {
                    return Ok(ProgramState::Revert(r));
                }
            } else {
                match state {
                    ExecuteState::Return(r) => {
                        return Ok(ProgramState::Return(r));
                    }

                    ExecuteState::ReturnData(d) => {
                        return Ok(ProgramState::ReturnData(d));
                    }

                    ExecuteState::Revert(r) => {
                        return Ok(ProgramState::Revert(r));
                    }

                    ExecuteState::Proceed => (),

                    #[cfg(feature = "debug")]
                    ExecuteState::DebugEvent(d) => {
                        return Ok(ProgramState::RunProgram(d));
                    }
                }
            }
        }
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    <Tx as IntoChecked>::CheckedMetadata: CheckedMetadata,
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
            .map(|state| StateTransition::new(state, interpreter.tx, interpreter.receipts.into()))
    }

    /// Initialize a pre-allocated instance of [`Interpreter`] with the provided
    /// transaction and execute it. The result will be bound to the lifetime
    /// of the interpreter and will avoid unnecessary copy with the data
    /// that can be referenced from the interpreter instance itself.
    pub fn transact(&mut self, tx: Checked<Tx>) -> Result<StateTransitionRef<'_, Tx>, InterpreterError> {
        let state_result = self.init_script(tx).and_then(|_| self.run());

        #[cfg(feature = "profile-any")]
        self.profiler.on_transaction(&state_result);

        let state = state_result?;
        Ok(StateTransitionRef::new(state, self.transaction(), self.receipts()))
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
{
    /// Deploys `Create` transaction without initialization VM and without invalidation of the
    /// last state of execution of the `Script` transaction.
    ///
    /// Returns `Create` transaction with all modifications after execution.
    pub fn deploy(&mut self, tx: Checked<Create>) -> Result<Create, InterpreterError> {
        let (mut create, metadata) = tx.into();
        Self::_deploy(&mut create, &mut self.storage, metadata.balances(), &self.params)?;
        Ok(create)
    }
}
