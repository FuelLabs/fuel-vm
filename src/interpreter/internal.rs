use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::RuntimeError;

use fuel_asm::{Instruction, PanicReason};
use fuel_tx::Transaction;
use fuel_types::{Color, ContractId, RegisterId, Word};

impl<S> Interpreter<S> {
    pub(crate) fn push_stack(&mut self, data: &[u8]) -> Result<(), RuntimeError> {
        let (ssp, overflow) = self.registers[REG_SSP].overflowing_add(data.len() as Word);

        if overflow || !self.is_external_context() && ssp > self.registers[REG_SP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.memory[self.registers[REG_SSP] as usize..ssp as usize].copy_from_slice(data);
            self.registers[REG_SSP] = ssp;

            Ok(())
        }
    }

    pub(crate) const fn block_height(&self) -> u32 {
        self.block_height
    }

    pub(crate) fn set_flag(&mut self, a: Word) -> Result<(), RuntimeError> {
        self.registers[REG_FLAG] = a;

        self.inc_pc()
    }

    pub(crate) fn clear_err(&mut self) {
        self.registers[REG_ERR] = 0;
    }

    pub(crate) fn set_err(&mut self) {
        self.registers[REG_ERR] = 1;
    }

    pub(crate) fn inc_pc(&mut self) -> Result<(), RuntimeError> {
        self.registers[REG_PC]
            .checked_add(Instruction::LEN as Word)
            .ok_or(PanicReason::ArithmeticOverflow.into())
            .map(|pc| self.registers[REG_PC] = pc)
    }

    pub(crate) const fn context(&self) -> Context {
        if self.registers[REG_FP] == 0 {
            self.context
        } else {
            Context::Call
        }
    }

    pub(crate) const fn is_external_context(&self) -> bool {
        self.context().is_external()
    }

    pub(crate) const fn is_internal_context(&self) -> bool {
        !self.is_external_context()
    }

    pub(crate) const fn is_predicate(&self) -> bool {
        matches!(self.context, Context::Predicate)
    }

    pub(crate) const fn is_register_writable(ra: RegisterId) -> Result<(), RuntimeError> {
        if ra >= REG_WRITABLE {
            Ok(())
        } else {
            Err(RuntimeError::Recoverable(PanicReason::ReservedRegisterNotWritable))
        }
    }

    pub(crate) const fn transaction(&self) -> &Transaction {
        &self.tx
    }

    pub(crate) fn internal_contract(&self) -> Result<&ContractId, RuntimeError> {
        let (c, cx) = self.internal_contract_bounds()?;

        // Safety: Memory bounds logically verified by the interpreter
        let contract = unsafe { ContractId::as_ref_unchecked(&self.memory[c..cx]) };

        Ok(contract)
    }

    pub(crate) fn internal_contract_or_default(&self) -> ContractId {
        // Safety: memory bounds checked by `internal_contract_bounds`
        self.internal_contract_bounds()
            .map(|(c, cx)| unsafe { ContractId::from_slice_unchecked(&self.memory[c..cx]) })
            .unwrap_or_default()
    }

    pub(crate) fn internal_contract_bounds(&self) -> Result<(usize, usize), RuntimeError> {
        self.is_internal_context()
            .then(|| {
                let c = self.registers[REG_FP] as usize;
                let cx = c + ContractId::LEN;

                (c, cx)
            })
            .ok_or(PanicReason::ExpectedInternalContext.into())
    }

    pub(crate) fn external_color_balance_sub(&mut self, color: &Color, value: Word) -> Result<(), RuntimeError> {
        if value == 0 {
            return Ok(());
        }

        let balance = self.free_balances.get_mut(&color).ok_or(PanicReason::ColorNotFound)?;
        *balance = balance.checked_sub(value).ok_or(PanicReason::NotEnoughBalance)?;

        Ok(())
    }
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn external_balance() {
        let mut rng = StdRng::seed_from_u64(2322u64);

        let mut vm = Interpreter::with_memory_storage();

        let gas_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;
        let byte_price = 0;

        let script = vec![Opcode::RET(0x01)].iter().copied().collect();
        let balances = vec![(rng.gen(), 100), (rng.gen(), 500)];

        let inputs = balances
            .iter()
            .map(|(color, amount)| Input::coin(rng.gen(), rng.gen(), *amount, *color, 0, maturity, vec![], vec![]))
            .collect();

        let tx = Transaction::script(
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            script,
            vec![],
            inputs,
            vec![],
            vec![vec![].into()],
        );

        vm.init(tx).expect("Failed to init VM!");

        for (color, amount) in balances {
            assert!(vm.external_color_balance_sub(&color, amount + 1).is_err());
            vm.external_color_balance_sub(&color, amount - 10).unwrap();
            assert!(vm.external_color_balance_sub(&color, 11).is_err());
            vm.external_color_balance_sub(&color, 10).unwrap();
            assert!(vm.external_color_balance_sub(&color, 1).is_err());
        }
    }
}
