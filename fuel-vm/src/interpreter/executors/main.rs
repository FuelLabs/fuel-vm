use crate::checked_transaction::{Checked, IntoChecked};
use crate::consts::*;
use crate::context::Context;
use crate::crypto;
use crate::error::{Bug, BugId, BugVariant, InterpreterError, PredicateVerificationFailed, RuntimeError};
use crate::gas::GasCosts;
use crate::interpreter::{CheckedMetadata, ExecutableTransaction, InitialBalances, Interpreter, RuntimeBalances};
use crate::predicate::RuntimePredicate;
use crate::state::{ExecuteState, ProgramState};
use crate::state::{StateTransition, StateTransitionRef};
use crate::storage::{InterpreterStorage, PredicateStorage};

use crate::error::BugVariant::GlobalGasUnderflow;
use fuel_asm::PanicReason;
use fuel_tx::{
    field::{Outputs, ReceiptsRoot, Salt, Script as ScriptField, StorageSlots},
    Chargeable, ConsensusParameters, Contract, Create, Input, Output, Receipt, ScriptExecutionResult,
};
use fuel_types::bytes::SerializableVec;
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
        checked: Checked<Tx>,
        params: ConsensusParameters,
        gas_costs: GasCosts,
    ) -> Result<PredicatesChecked, PredicateVerificationFailed>
    where
        Tx: ExecutableTransaction,
        <Tx as IntoChecked>::Metadata: CheckedMetadata,
    {
        if !checked.transaction().check_predicate_owners() {
            return Err(PredicateVerificationFailed::InvalidOwner);
        }

        let mut vm = Interpreter::with_storage(PredicateStorage::default(), params, gas_costs);

        // Needed for now because checked is only freed once the value is collected into a Vec
        #[allow(clippy::needless_collect)]
        let predicates: Vec<_> = (0..checked.transaction().inputs().len())
            .filter_map(|i| RuntimePredicate::from_tx(&params, checked.transaction(), i))
            .collect();

        // Since we reuse the vm objects otherwise, we need to keep the actual gas here
        let tx_gas_limit = checked.transaction().limit();
        let mut remaining_gas = tx_gas_limit;

        vm.init_predicate(checked);

        for predicate in predicates {
            // VM is cloned because the state should be reset for every predicate verification
            let mut vm = vm.clone();

            vm.context = Context::Predicate { program: predicate };
            vm.set_remaining_gas(remaining_gas);

            if !matches!(vm.verify_predicate()?, ProgramState::Return(0x01)) {
                return Err(PredicateVerificationFailed::False);
            }

            remaining_gas = vm.registers[REG_GGAS];
        }

        Ok(PredicatesChecked {
            gas_used: tx_gas_limit
                .checked_sub(remaining_gas)
                .ok_or_else(|| Bug::new(BugId::ID004, GlobalGasUnderflow))?,
        })
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    pub(crate) fn run_call(&mut self) -> Result<ProgramState, RuntimeError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(PanicReason::MemoryOverflow.into());
            }

            let state = self.execute().map_err(|e| {
                e.panic_reason()
                    .map(RuntimeError::Recoverable)
                    .unwrap_or_else(|| RuntimeError::Halt(e.into()))
            })?;

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
            &RuntimeBalances::from(initial_balances.clone()),
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
                if let Input::Contract { contract_id, .. } = input {
                    !self.check_contract_exists(contract_id).unwrap_or(false)
                } else {
                    false
                }
            }) {
                return Err(InterpreterError::Panic(PanicReason::ContractNotFound));
            }

            if let Some(script) = self.transaction().as_script() {
                let offset = (self.tx_offset() + script.script_offset()) as Word;

                self.registers[REG_PC] = offset;
                self.registers[REG_IS] = offset;
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
                        self.append_panic_receipt(*result);

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

            let receipts_root = if self.receipts().is_empty() {
                EMPTY_RECEIPTS_MERKLE_ROOT.into()
            } else {
                crypto::ephemeral_merkle_root(self.receipts().iter().map(|r| r.clone().to_bytes()))
            };

            // TODO optimize
            if let Some(script) = self.tx.as_script_mut() {
                // TODO: also set this on the serialized tx in memory to keep serialized form consistent
                // https://github.com/FuelLabs/fuel-vm/issues/97
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
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(InterpreterError::Panic(PanicReason::MemoryOverflow));
            }

            match self.execute()? {
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
            .map(|state| StateTransition::new(state, interpreter.tx, interpreter.receipts))
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
