use crate::consts::*;
use crate::contract::Contract;
use crate::crypto;
use crate::error::{InterpreterError, RuntimeError};
use crate::interpreter::{Interpreter, MemoryRange};
use crate::state::{ExecuteState, ProgramState, StateTransitionRef};
use crate::storage::InterpreterStorage;

use fuel_asm::{Instruction, InstructionResult, Opcode, OpcodeRepr, PanicReason};
use fuel_tx::{Input, Output, Receipt, Transaction};
use fuel_types::bytes::SerializableVec;
use fuel_types::Word;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    // TODO maybe infallible?
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
                    Err(InterpreterError::Panic(PanicReason::ContractNotFound))?
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
                    Err(InterpreterError::Panic(PanicReason::ContractNotInInputs))?;
                }

                self.storage
                    .storage_contract_insert(&id, &contract)
                    .map_err(InterpreterError::from_io)?;

                self.storage
                    .storage_contract_root_insert(&id, salt, &root)
                    .map_err(InterpreterError::from_io)?;

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

                let program = self.run_program();
                let gas_used = self.tx.gas_limit() - self.registers[REG_GGAS];

                // Catch VM panic and don't propagate, generating a receipt
                let (status, program) = match program {
                    Ok(s) => (InstructionResult::success(), s),

                    Err(e) => match e.instruction_result() {
                        Some(result) => {
                            const RVRT: Instruction = Instruction::new((OpcodeRepr::RVRT as u32) << 24);
                            debug_assert_eq!(RVRT, Opcode::RVRT(REG_ZERO).into());

                            // The only possible well-formed panic for `RVRT` is out of gas.
                            match self.instruction(RVRT).err() {
                                // Recoverable panic that consumes all remaining local gas and reverts the tx's state
                                // changes.
                                Some(e) if e.panic_reason() == Some(PanicReason::OutOfGas) => (),
                                None => (),

                                // This case is unreachable according to specs.
                                //
                                // If this code is reached, it is an implementation problem and a
                                // bug should be filed.
                                Some(e) if e.panic_reason().is_some() => return Err(e),

                                // Any other variant is a halt error.
                                Some(e) => return Err(e),
                            }

                            (*result, ProgramState::Revert(0))
                        }

                        // This isn't a specified case of an erroneous program and should be
                        // propagated. If applicable, OS errors will fall into this category.
                        None => {
                            return Err(e);
                        }
                    },
                };

                let receipt = Receipt::script_result(status, gas_used);

                self.receipts.push(receipt);

                state = program;
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

    /// Initialize a pre-allocated instance of [`Interpreter`] with the provided
    /// transaction and execute it. The result will be bound to the lifetime
    /// of the interpreter and will avoid unnecessary copy with the data
    /// that can be referenced from the interpreter instance itself.
    pub fn transact(&mut self, tx: Transaction) -> Result<StateTransitionRef<'_>, InterpreterError> {
        let state = self.init(tx).and_then(|_| self.run())?;

        let transition = StateTransitionRef::new(state, self.transaction(), self.receipts());

        Ok(transition)
    }
}
