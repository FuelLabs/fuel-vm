use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::error::RuntimeError;
use std::io::Read;

use fuel_asm::{Instruction, PanicReason};
use fuel_tx::{Output, Transaction};
use fuel_types::{Address, Color, ContractId, RegisterId, Word};

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

    /// Retrieve the unspent balance for a given color
    pub(crate) fn external_color_balance(&self, color: &Color) -> Result<Word, RuntimeError> {
        let offset = *self
            .unused_balance_index
            .get(&color)
            .ok_or(PanicReason::ColorNotFound)?;
        let balance_memory = &self.memory[offset..offset + WORD_SIZE];

        let balance = <[u8; WORD_SIZE]>::try_from(&*balance_memory).expect("Sized chunk expected to fit!");
        let balance = Word::from_be_bytes(balance);

        Ok(balance)
    }

    /// Reduces the unspent balance of a given color
    pub(crate) fn external_color_balance_sub(&mut self, color: &Color, value: Word) -> Result<(), RuntimeError> {
        if value == 0 {
            return Ok(());
        }

        let offset = *self
            .unused_balance_index
            .get(&color)
            .ok_or(PanicReason::ColorNotFound)?;

        let balance_memory = &mut self.memory[offset..offset + WORD_SIZE];

        let balance = <[u8; WORD_SIZE]>::try_from(&*balance_memory).expect("Sized chunk expected to fit!");
        let balance = Word::from_be_bytes(balance);
        let balance = balance.checked_sub(value).ok_or(PanicReason::NotEnoughBalance)?;
        let balance = balance.to_be_bytes();

        balance_memory.copy_from_slice(&balance);

        Ok(())
    }

    /// Increase the variable output with a given color. Modifies both the referenced tx and the
    /// serialized tx in vm memory.
    pub(crate) fn set_variable_output(
        &mut self,
        out_idx: usize,
        color_to_update: Color,
        amount_to_set: Word,
        owner_to_set: Address,
    ) -> Result<(), RuntimeError> {
        let outputs = self.tx.outputs();

        if out_idx >= outputs.len() {
            return Err(PanicReason::OutputNotFound.into());
        }
        let output = outputs[out_idx];
        match output {
            Output::Variable { amount, .. } if amount == 0 => Ok(()),
            Output::Variable { amount, .. } if amount != 0 => Err(PanicReason::MemoryWriteOverlap),
            _ => Err(PanicReason::ExpectedOutputVariable),
        }?;

        // update the local copy of the output
        let mut output = Output::variable(owner_to_set, amount_to_set, color_to_update);

        // update serialized memory state
        let offset = self.tx.output_offset(out_idx).ok_or(PanicReason::OutputNotFound)?;
        let bytes = &mut self.memory[offset..];
        let _ = output.read(bytes)?;

        let outputs = match &mut self.tx {
            Transaction::Script { outputs, .. } => outputs,
            Transaction::Create { outputs, .. } => outputs,
        };
        // update referenced tx
        outputs[out_idx] = output;

        Ok(())
    }
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::io::Write;

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

    #[test]
    fn variable_output_updates_in_memory() {
        let mut rng = StdRng::seed_from_u64(2322u64);

        let mut vm = Interpreter::with_memory_storage();

        let gas_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;
        let byte_price = 0;
        let color_to_update: Color = rng.gen();
        let amount_to_set: Word = 100;
        let owner: Address = rng.gen();

        let variable_output = Output::Variable {
            to: rng.gen(),
            amount: 0,
            color: rng.gen(),
        };

        let tx = Transaction::script(
            gas_price,
            gas_limit,
            byte_price,
            maturity,
            vec![],
            vec![],
            vec![],
            vec![variable_output],
            vec![Witness::default()],
        );

        vm.init(tx).expect("Failed to init VM!");

        // increase variable output
        vm.set_variable_output(0, color_to_update, amount_to_set, owner)
            .unwrap();

        // verify the referenced tx output is updated properly
        assert!(matches!(
            vm.tx.outputs()[0],
            Output::Variable {amount, color, to} if amount == amount_to_set
                                                    && color == color_to_update
                                                    && to == owner
        ));

        // verify the vm memory is updated properly
        let position = vm.tx.output_offset(0).unwrap();
        let mut mem_output = Output::variable(Default::default(), Default::default(), Default::default());
        let _ = mem_output.write(&vm.memory()[position..]).unwrap();
        assert_eq!(vm.tx.outputs()[0], mem_output);
    }
}
