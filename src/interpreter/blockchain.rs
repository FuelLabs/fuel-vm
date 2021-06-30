use super::{ExecuteError, Interpreter, MemoryRange};
use crate::consts::*;
use crate::data::InterpreterStorage;

use fuel_asm::Word;
use fuel_tx::{ContractId, Input};

use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn burn(&mut self, a: Word) -> Result<bool, ExecuteError> {
        self.internal_color()
            .and_then(|color| self.balance_sub(color, a))
            .map(|_| self.inc_pc())
    }

    pub fn mint(&mut self, a: Word) -> Result<bool, ExecuteError> {
        self.internal_color()
            .and_then(|color| self.balance_add(color, a))
            .map(|_| self.inc_pc())
    }

    // TODO add CCP tests
    pub fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> bool {
        let (ad, overflow) = a.overflowing_add(d);
        let (bx, of) = b.overflowing_add(ContractId::size_of() as Word);
        let overflow = overflow || of;
        let (cd, of) = c.overflowing_add(d);
        let overflow = overflow || of;

        let range = MemoryRange::new(a, d);
        if overflow
            || ad >= VM_MAX_RAM
            || bx >= VM_MAX_RAM
            || d > MEM_MAX_ACCESS_SIZE
            || !self.has_ownership_range(&range)
        {
            return false;
        }

        let contract =
            ContractId::try_from(&self.memory[b as usize..bx as usize]).expect("Memory bounds logically checked");

        if !self
            .tx
            .inputs()
            .iter()
            .any(|input| matches!(input, Input::Contract { contract_id, .. } if contract_id == &contract))
        {
            return false;
        }

        // TODO optmize
        let contract = match self.contract(&contract) {
            Ok(Some(c)) => c,
            _ => return false,
        };

        let memory = &mut self.memory[a as usize..ad as usize];
        if contract.as_ref().len() < cd as usize {
            memory.iter_mut().for_each(|m| *m = 0);
        } else {
            memory.copy_from_slice(&contract.as_ref()[..d as usize]);
        }

        true
    }
}
