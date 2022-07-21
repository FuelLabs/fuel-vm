use super::Interpreter;
use crate::consts::*;
use crate::context::Context;
use crate::crypto;
use crate::error::RuntimeError;

use fuel_asm::{Instruction, PanicReason};
use fuel_tx::{Output, Receipt};
use fuel_types::bytes::SerializableVec;
use fuel_types::{AssetId, Bytes32, ContractId, RegisterId, Word};

use core::mem;

impl<S> Interpreter<S> {
    pub(crate) fn reserve_stack(&mut self, len: Word) -> Result<Word, RuntimeError> {
        let (ssp, overflow) = self.registers[REG_SSP].overflowing_add(len);

        if overflow || !self.is_external_context() && ssp > self.registers[REG_SP] {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            Ok(mem::replace(&mut self.registers[REG_SSP], ssp))
        }
    }

    pub(crate) fn push_stack(&mut self, data: &[u8]) -> Result<(), RuntimeError> {
        let ssp = self.reserve_stack(data.len() as Word)?;

        self.memory[ssp as usize..self.registers[REG_SSP] as usize].copy_from_slice(data);

        Ok(())
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

    /// Reduces the unspent balance of a given asset ID
    pub(crate) fn external_asset_id_balance_sub(
        &mut self,
        asset_id: &AssetId,
        value: Word,
    ) -> Result<(), RuntimeError> {
        let balances = &mut self.balances;
        let memory = &mut self.memory;

        balances
            .checked_balance_sub(memory, asset_id, value)
            .ok_or(PanicReason::NotEnoughBalance)?;

        Ok(())
    }

    /// Reduces the unspent balance of the base asset
    pub(crate) fn base_asset_balance_sub(&mut self, value: Word) -> Result<(), RuntimeError> {
        self.external_asset_id_balance_sub(&AssetId::default(), value)
    }

    /// Increase the variable output with a given asset ID. Modifies both the referenced tx and the
    /// serialized tx in vm memory.
    pub(crate) fn set_variable_output(&mut self, idx: usize, variable: Output) -> Result<(), RuntimeError> {
        self.tx.tx_replace_variable_output(idx, variable)?;
        self.update_memory_output(idx)?;

        Ok(())
    }

    pub(crate) fn set_message_output(&mut self, idx: usize, message: Output) -> Result<(), RuntimeError> {
        self.tx.tx_replace_message_output(idx, message)?;
        self.update_memory_output(idx)?;

        Ok(())
    }

    pub(crate) fn update_memory_output(&mut self, idx: usize) -> Result<(), RuntimeError> {
        let offset = self.tx_offset()
            + self
                .transaction()
                .output_offset(idx)
                .ok_or(PanicReason::OutputNotFound)?;

        let tx = &mut self.tx;
        let mem = &mut self.memory[offset..];

        tx.tx_output_to_mem(idx, mem)?;

        Ok(())
    }

    pub(crate) fn append_receipt(&mut self, receipt: Receipt) {
        self.receipts.push(receipt);

        self.transaction()
            .receipts_root_offset()
            .map(|offset| offset + self.tx_offset())
            .map(|offset| {
                // TODO this generates logarithmic gas cost to the receipts count. This won't fit the
                // linear monadic model and should be discussed. Maybe the receipts tree should have
                // constant capacity so the gas cost is also constant to the maximum depth?
                let root = if self.receipts().is_empty() {
                    EMPTY_RECEIPTS_MERKLE_ROOT.into()
                } else {
                    crypto::ephemeral_merkle_root(self.receipts().iter().map(|r| r.clone().to_bytes()))
                };

                self.tx.tx_set_receipts_root(root);

                // Transaction memory space length is already checked on initialization so its
                // guaranteed to fit
                (&mut self.memory[offset..offset + Bytes32::LEN]).copy_from_slice(&root[..]);
            });
    }

    pub(crate) const fn tx_offset(&self) -> usize {
        self.params.tx_offset()
    }

    pub(crate) fn tx_id(&self) -> &Bytes32 {
        // Safety: vm parameters guarantees enough space for txid
        unsafe { Bytes32::as_ref_unchecked(&self.memory[..Bytes32::LEN]) }
    }
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use crate::prelude::*;
    use fuel_tx::TransactionBuilder;
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
        let height = 0;

        let script = vec![Opcode::RET(0x01)].iter().copied().collect();
        let balances = vec![(rng.gen(), 100), (rng.gen(), 500)];

        let mut tx = TransactionBuilder::script(script, Default::default());

        balances.iter().copied().for_each(|(asset, amount)| {
            tx.add_unsigned_coin_input(rng.gen(), rng.gen(), amount, asset, maturity);
        });

        let tx = tx
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .gas_limit(100)
            .maturity(maturity)
            .finalize_checked(height as Word, &Default::default());

        vm.init_with_storage(tx).expect("Failed to init VM!");

        for (asset_id, amount) in balances {
            assert!(vm.external_asset_id_balance_sub(&asset_id, amount + 1).is_err());
            vm.external_asset_id_balance_sub(&asset_id, amount - 10).unwrap();
            assert!(vm.external_asset_id_balance_sub(&asset_id, 11).is_err());
            vm.external_asset_id_balance_sub(&asset_id, 10).unwrap();
            assert!(vm.external_asset_id_balance_sub(&asset_id, 1).is_err());
        }
    }

    #[test]
    fn variable_output_updates_in_memory() {
        let mut rng = StdRng::seed_from_u64(2322u64);

        let mut vm = Interpreter::with_memory_storage();

        let gas_price = 0;
        let gas_limit = 1_000_000;
        let maturity = 0;
        let height = 0;
        let asset_id_to_update: AssetId = rng.gen();
        let amount_to_set: Word = 100;
        let owner: Address = rng.gen();

        let variable_output = Output::Variable {
            to: rng.gen(),
            amount: 0,
            asset_id: rng.gen(),
        };

        let tx = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            vec![],
            vec![],
            vec![],
            vec![variable_output],
            vec![Witness::default()],
        )
        .check(height, vm.params())
        .expect("failed to check tx");

        vm.init_with_storage(tx).expect("Failed to init VM!");

        // increase variable output
        let variable = Output::variable(owner, amount_to_set, asset_id_to_update);

        vm.set_variable_output(0, variable).unwrap();

        // verify the referenced tx output is updated properly
        assert!(matches!(
            vm.transaction().outputs()[0],
            Output::Variable {amount, asset_id, to} if amount == amount_to_set
                                                    && asset_id == asset_id_to_update
                                                    && to == owner
        ));

        // verify the vm memory is updated properly
        let position = vm.params.tx_offset() + vm.transaction().output_offset(0).unwrap();
        let mut mem_output = Output::variable(Default::default(), Default::default(), Default::default());
        let _ = mem_output.write(&vm.memory()[position..]).unwrap();
        assert_eq!(vm.transaction().outputs()[0], mem_output);
    }
}
