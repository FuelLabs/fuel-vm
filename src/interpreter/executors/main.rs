use crate::consts::*;
use crate::crypto;
use crate::error::InterpreterError;
use crate::interpreter::{Interpreter, MemoryRange};
use crate::prelude::*;
use crate::state::{ExecuteState, ProgramState, StateTransitionRef};
use crate::storage::{InterpreterStorage, PredicateStorage};

use fuel_asm::PanicReason;
use fuel_tx::{ConsensusParameters, Contract, Input, Output, Receipt, ScriptExecutionResult, Transaction};
use fuel_types::bytes::SerializableVec;
use fuel_types::Word;

impl Interpreter<PredicateStorage> {
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
    pub fn check_predicates(tx: Transaction, params: ConsensusParameters) -> bool {
        let mut vm = Interpreter::with_storage(PredicateStorage::default(), params);

        if !tx.check_predicate_owners() {
            return false;
        }

        #[allow(clippy::needless_collect)] // TODO: the collect could probably be removed
        let predicates: Vec<MemoryRange> = tx
            .inputs()
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| vm.input_to_predicate(&tx, idx))
            .collect();

        predicates
            .into_iter()
            .fold(vm.init_predicate(tx), |result, predicate| -> bool {
                // VM is cloned because the state should be reset for every predicate verification
                result && vm.clone()._check_predicate(predicate)
            })
    }

    /// Initialize the VM with the provided transaction and check the input predicate indexed by
    /// `idx`. If the input isn't of type [`Input::CoinPredicate`], the function will return
    /// `false`.
    ///
    /// For additional information, check [`Self::check_predicates`]
    pub fn check_predicate(&mut self, tx: Transaction, idx: usize) -> bool {
        tx.check_predicate_owner(idx)
            .then(|| self.input_to_predicate(&tx, idx))
            .flatten()
            .map(|predicate| self.init_predicate(tx) && self._check_predicate(predicate))
            .unwrap_or(false)
    }

    fn init_predicate(&mut self, tx: Transaction) -> bool {
        let block_height = 0;

        self.init(true, block_height, tx).is_ok()
    }

    fn input_to_predicate(&self, tx: &Transaction, idx: usize) -> Option<MemoryRange> {
        tx.input_coin_predicate_offset(idx)
            .map(|(ofs, len)| (ofs as Word + self.tx_offset() as Word, len as Word))
            .map(|(ofs, len)| MemoryRange::new(ofs, len))
    }

    /// Validate the predicate, assuming the interpreter is initialized
    fn _check_predicate(&mut self, predicate: MemoryRange) -> bool {
        matches!(self.verify_predicate(&predicate), Ok(ProgramState::Return(0x01)))
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    // TODO maybe infallible?
    pub(crate) fn run(&mut self) -> Result<ProgramState, InterpreterError> {
        let state = match &self.tx {
            Transaction::Create {
                salt, storage_slots, ..
            } => {
                let contract = Contract::try_from(&self.tx)?;
                let root = contract.root();
                let storage_root = Contract::initial_state_root(storage_slots.iter());
                let id = contract.id(salt, &root, &storage_root);

                if !&self
                    .tx
                    .outputs()
                    .iter()
                    .any(|output| matches!(output, Output::ContractCreated { contract_id, state_root } if contract_id == &id && state_root == &storage_root))
                {
                    return Err(InterpreterError::Panic(PanicReason::ContractNotInInputs));
                }

                self.storage
                    .storage_contract_insert(&id, &contract)
                    .map_err(InterpreterError::from_io)?;

                self.storage
                    .storage_contract_root_insert(&id, salt, &root)
                    .map_err(InterpreterError::from_io)?;

                for storage_slot in storage_slots {
                    self.storage
                        .merkle_contract_state_insert(&id, storage_slot.key(), storage_slot.value())
                        .map_err(InterpreterError::from_io)?;
                }

                ProgramState::Return(1)
            }

            Transaction::Script { inputs, .. } => {
                if inputs.iter().any(|input| {
                    if let Input::Contract { contract_id, .. } = input {
                        !self.check_contract_exists(contract_id).unwrap_or(false)
                    } else {
                        false
                    }
                }) {
                    return Err(InterpreterError::Panic(PanicReason::ContractNotFound));
                }

                let offset = (self.tx_offset() + Transaction::script_offset()) as Word;

                self.registers[REG_PC] = offset;
                self.registers[REG_IS] = offset;

                // TODO set tree balance

                let program = self.run_program();
                let gas_used = self.tx.gas_limit() - self.registers[REG_GGAS];

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

                program
            }
        };

        #[cfg(feature = "debug")]
        if state.is_debug() {
            self.debugger_set_last_state(state);
        }

        // TODO optimize
        if self.tx.receipts_root().is_some() {
            let receipts_root = if self.receipts().is_empty() {
                EMPTY_RECEIPTS_MERKLE_ROOT.into()
            } else {
                crypto::ephemeral_merkle_root(self.receipts().iter().map(|r| r.clone().to_bytes()))
            };

            // TODO: also set this on the serialized tx in memory to keep serialized form consistent
            // https://github.com/FuelLabs/fuel-vm/issues/97
            self.tx.set_receipts_root(receipts_root);
        }

        // the consumed balance for the bytes cost is non-refundable so we just check the ggas
        let factor = self.params.gas_price_factor as f64;
        let gas_refund = self
            .tx
            .gas_price()
            .checked_mul(self.registers[REG_GGAS])
            .ok_or(ValidationError::ArithmeticOverflow)? as f64;
        let gas_refund = (gas_refund / factor).floor() as Word;

        let revert = matches!(state, ProgramState::Revert(_));

        self.finalize_outputs(gas_refund, revert)?;

        Ok(state)
    }

    pub(crate) fn run_call(&mut self) -> Result<ProgramState, RuntimeError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(PanicReason::MemoryOverflow.into());
            }

            let state = self
                .execute()
                .map_err(|e| e.panic_reason().expect("Call routine should return only VM panic"))?;

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

    /// Allocate internally a new instance of [`Interpreter`] with the provided
    /// storage, initialize it with the provided transaction and return the
    /// result of th execution in form of [`StateTransition`]
    pub fn transact_owned(
        storage: S,
        tx: Transaction,
        params: ConsensusParameters,
    ) -> Result<StateTransition, InterpreterError> {
        Interpreter::with_storage(storage, params)
            .transact(tx)
            .map(|st| st.into_owned())
    }

    /// Initialize a pre-allocated instance of [`Interpreter`] with the provided
    /// transaction and execute it. The result will be bound to the lifetime
    /// of the interpreter and will avoid unnecessary copy with the data
    /// that can be referenced from the interpreter instance itself.
    pub fn transact(&mut self, tx: Transaction) -> Result<StateTransitionRef<'_>, InterpreterError> {
        let state_result = self.init_with_storage(tx).and_then(|_| self.run());

        #[cfg(feature = "profile-any")]
        self.profiler.on_transaction(&state_result);

        let state = state_result?;

        let transition = StateTransitionRef::new(state, self.transaction(), self.receipts());

        Ok(transition)
    }
}
