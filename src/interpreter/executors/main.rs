use crate::consts::*;
use crate::contract::Contract;
use crate::crypto;
use crate::error::{Backtrace, InterpreterError};
use crate::interpreter::{Interpreter, MemoryRange};
use crate::state::{ExecuteState, ProgramState, StateTransition, StateTransitionRef};
use crate::storage::InterpreterStorage;

use fuel_tx::{Input, Output, Receipt, Transaction};
use fuel_types::bytes::SerializableVec;
use fuel_types::Word;

use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    fn into_inner(self) -> (Transaction, Vec<Receipt>) {
        (self.tx, self.receipts)
    }

    pub(crate) fn run(&mut self) -> Result<ProgramState, InterpreterError> {
        let mut state: ProgramState;

        match &self.tx {
            Transaction::Create {
                salt, static_contracts, ..
            } => {
                if static_contracts
                    .iter()
                    .any(|id| !self.check_contract_exists(id).unwrap_or(false))
                {
                    Err(InterpreterError::TransactionCreateStaticContractNotFound)?
                }

                let contract = Contract::try_from(&self.tx)?;
                let root = contract.root();
                let id = contract.id(salt, &root);

                if !&self
                    .tx
                    .outputs()
                    .iter()
                    .any(|output| matches!(output, Output::ContractCreated { contract_id } if contract_id == &id))
                {
                    Err(InterpreterError::TransactionCreateIdNotInTx)?;
                }

                self.storage.storage_contract_insert(&id, &contract)?;
                self.storage.storage_contract_root_insert(&id, salt, &root)?;

                // Verify predicates
                // https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_validity.md#predicate-verification
                // TODO this should be abstracted with the client
                let predicates: Vec<MemoryRange> = self
                    .tx
                    .inputs()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, input)| match input {
                        Input::Coin { predicate, .. } if !predicate.is_empty() => self
                            .tx
                            .input_coin_predicate_offset(i)
                            .map(|ofs| (ofs as Word, predicate.len() as Word)),
                        _ => None,
                    })
                    .map(|(ofs, len)| (ofs + VM_TX_MEMORY as Word, len))
                    .map(|(ofs, len)| MemoryRange::new(ofs, len))
                    .collect();

                state = ProgramState::Return(1);
                for predicate in predicates {
                    state = self.verify_predicate(&predicate)?;

                    #[cfg(feature = "debug")]
                    if state.is_debug() {
                        // TODO should restore the constructed predicates and continue from current
                        // predicate
                        return Ok(state);
                    }
                }
            }

            Transaction::Script { .. } => {
                let offset = (VM_TX_MEMORY + Transaction::script_offset()) as Word;

                self.registers[REG_PC] = offset;
                self.registers[REG_IS] = offset;
                self.registers[REG_GGAS] = self.tx.gas_limit();
                self.registers[REG_CGAS] = self.tx.gas_limit();

                // TODO set tree balance

                state = self.run_program()?;
            }
        }

        #[cfg(feature = "debug")]
        if state.is_debug() {
            self.debugger_set_last_state(state.clone());
        }

        // TODO optimize
        if self.tx.receipts_root().is_some() {
            let receipts_root = if self.receipts().is_empty() {
                EMPTY_RECEIPTS_MERKLE_ROOT.into()
            } else {
                crypto::ephemeral_merkle_root(self.receipts().iter().map(|r| r.clone().to_bytes()))
            };

            self.tx.set_receipts_root(receipts_root);
        }

        Ok(state)
    }

    pub(crate) fn run_program(&mut self) -> Result<ProgramState, InterpreterError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(InterpreterError::ProgramOverflow);
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

    pub fn transition(storage: S, tx: Transaction) -> Result<StateTransition, InterpreterError> {
        let mut vm = Interpreter::with_storage(storage);

        vm.init(tx)?;

        let state = vm.run()?;
        let (tx, receipts) = vm.into_inner();
        let transition = StateTransition::new(state, tx, receipts);

        Ok(transition)
    }

    pub fn transition_with_backtrace(storage: S, tx: Transaction) -> Result<StateTransition, Backtrace> {
        let mut vm = Interpreter::with_storage(storage);

        let state = vm.init(tx).and_then(|_| vm.run()).map_err(|e| e.backtrace(&vm))?;

        let (tx, receipts) = vm.into_inner();
        let transition = StateTransition::new(state, tx, receipts);

        Ok(transition)
    }

    pub fn transact(&mut self, tx: Transaction) -> Result<StateTransitionRef<'_>, InterpreterError> {
        self.init(tx)?;

        let state = self.run()?;
        let transition = StateTransitionRef::new(state, self.transaction(), self.receipts());

        Ok(transition)
    }

    pub fn transact_with_backtrace(&mut self, tx: Transaction) -> Result<StateTransitionRef<'_>, Backtrace> {
        let state = self.init(tx).and_then(|_| self.run()).map_err(|e| e.backtrace(&self))?;

        let transition = StateTransitionRef::new(state, self.transaction(), self.receipts());

        Ok(transition)
    }
}
