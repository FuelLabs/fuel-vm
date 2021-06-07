use super::{Interpreter, MemoryRange};
use crate::consts::*;
use crate::data::InterpreterStorage;

use fuel_asm::Word;
use fuel_tx::{Address, Color, ContractAddress, Input};

use std::convert::TryFrom;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn burn(&mut self, a: Word) -> bool {
        let (x, overflow) = self.registers[REG_FP].overflowing_add(Address::size_of() as Word);
        let (xc, of) = x.overflowing_add(Color::size_of() as Word);
        let overflow = overflow || of;

        if overflow || self.is_external_context() || xc >= VM_MAX_RAM {
            return false;
        }

        let color = Color::try_from(&self.memory[x as usize..xc as usize]).expect("Memory bounds logically verified");
        let balance = match self.color_balance(&color) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let (balance, underflow) = balance.overflowing_sub(a);

        if underflow {
            return false;
        }

        self.set_color_balance(color, balance).is_ok()
    }

    pub fn mint(&mut self, a: Word) -> bool {
        let (x, overflow) = self.registers[REG_FP].overflowing_add(Address::size_of() as Word);
        let (xc, of) = x.overflowing_add(Color::size_of() as Word);
        let overflow = overflow || of;

        if overflow || self.is_external_context() || xc >= VM_MAX_RAM {
            return false;
        }

        let color = Color::try_from(&self.memory[x as usize..xc as usize]).expect("Memory bounds logically verified");
        let balance = match self.color_balance(&color) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let (balance, overflow) = balance.overflowing_add(a);

        if overflow {
            return false;
        }

        self.set_color_balance(color, balance).is_ok()
    }

    // TODO add CCP tests
    pub fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> bool {
        let (ad, overflow) = a.overflowing_add(d);
        let (bx, of) = b.overflowing_add(ContractAddress::size_of() as Word);
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
            ContractAddress::try_from(&self.memory[b as usize..bx as usize]).expect("Memory bounds logically checked");

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
