use super::{ExecuteState, ProgramState, StateTransition, StateTransitionRef};
use crate::consts::*;
use crate::data::InterpreterStorage;
use crate::interpreter::{Contract, ContractData, ExecuteError, Interpreter, LogEvent, MemoryRange};

use fuel_asm::{Opcode, Word};
use fuel_tx::{Input, Output, Transaction};

use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    fn into_inner(self) -> (Transaction, Vec<LogEvent>) {
        (self.tx, self.log)
    }

    pub(crate) fn run(&mut self) -> Result<ProgramState, ExecuteError> {
        let mut state: ProgramState;

        match &self.tx {
            Transaction::Create {
                salt, static_contracts, ..
            } => {
                if static_contracts
                    .iter()
                    .any(|id| !self.check_contract_exists(id).unwrap_or(false))
                {
                    Err(ExecuteError::TransactionCreateStaticContractNotFound)?
                }

                let contract = Contract::try_from(&self.tx)?;
                let contract = ContractData::new(contract, *salt);
                let id = contract.id();

                if !&self
                    .tx
                    .outputs()
                    .iter()
                    .any(|output| matches!(output, Output::ContractCreated { contract_id } if contract_id == &id))
                {
                    Err(ExecuteError::TransactionCreateIdNotInTx)?;
                }

                self.storage.update(&id, contract)?;

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
                    .map(|(ofs, len)| (ofs + Self::tx_mem_address() as Word, len))
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
                let offset = (Self::tx_mem_address() + Transaction::script_offset()) as Word;

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

        Ok(state)
    }

    pub(crate) fn run_program(&mut self) -> Result<ProgramState, ExecuteError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(ExecuteError::ProgramOverflow);
            }

            let op = self.memory[self.registers[REG_PC] as usize..]
                .chunks_exact(4)
                .next()
                .map(Opcode::from_bytes_unchecked)
                .ok_or(ExecuteError::ProgramOverflow)?;

            match self.execute(op)? {
                ExecuteState::Return(r) => {
                    return Ok(ProgramState::Return(r));
                }

                #[cfg(feature = "debug")]
                ExecuteState::DebugEvent(d) => {
                    return Ok(ProgramState::RunProgram(d));
                }

                _ => (),
            }
        }
    }

    pub fn transition(storage: S, tx: Transaction) -> Result<StateTransition, ExecuteError> {
        let mut vm = Interpreter::with_storage(storage);

        vm.init(tx)?;

        let state = vm.run()?;
        let (tx, log) = vm.into_inner();
        let transition = StateTransition::new(state, tx, log);

        Ok(transition)
    }

    pub fn transact(&mut self, tx: Transaction) -> Result<StateTransitionRef<'_>, ExecuteError> {
        self.init(tx)?;

        let state = self.run()?;
        let transition = StateTransitionRef::new(state, self.transaction(), self.log());

        Ok(transition)
    }
}
