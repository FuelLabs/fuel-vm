use super::contract::{balance, balance_decrease, contract_size};
use super::internal::{base_asset_balance_sub, tx_id};
use super::{ExecutableTransaction, Interpreter, MemoryRange};
use crate::arith::{add_usize, checked_add_usize, checked_add_word};
use crate::call::CallFrame;
use crate::constraints::reg_key::*;
use crate::consts::*;
use crate::error::{Bug, BugId, BugVariant, RuntimeError};
use crate::interpreter::PanicContext;
use crate::storage::InterpreterStorage;

use fuel_asm::{PanicReason, RegId};
use fuel_tx::Receipt;
use fuel_types::bytes;
use fuel_types::{Address, AssetId, Bytes32, ContractId, RegisterId, Word};

use std::borrow::Borrow;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let ssp = self.registers[RegId::SSP];
        let sp = self.registers[RegId::SP];
        let fp = self.registers[RegId::FP] as usize;

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into());
        }

        let contract_id = ContractId::from(self.mem_read_bytes(a)?);

        let contract_offset = b as usize;
        let length = bytes::padded_len_usize(c as usize);

        let memory_offset = ssp;

        if length > self.params.contract_max_size as usize || length > MEM_MAX_ACCESS_SIZE {
            return Err(PanicReason::MemoryAccessSize.into());
        }

        // the contract must be declared in the transaction inputs
        if !self.tx.input_contracts().any(|id| *id == contract_id) {
            self.panic_context = PanicContext::ContractId(contract_id);
            return Err(PanicReason::ContractNotInInputs.into());
        };

        self.registers[RegId::SP] = self.registers[RegId::SP]
            .checked_add(length as Word)
            .ok_or_else(|| Bug::new(BugId::ID007, BugVariant::StackPointerOverflow))?;

        self.update_allocations()?;

        self.mem_write(memory_offset, length)?.fill(0);

        // fetch the storage contract
        let contract = super::contract::contract(&self.storage, &contract_id)?;
        let contract = contract.as_ref().as_ref();

        if contract_offset > contract.len() {
            return Err(PanicReason::MemoryAccess.into());
        }

        let contract = &contract[contract_offset..];
        let len = contract.len().min(length);

        let code = &contract[..len];

        // perform the code copy
        self.memory.write_slice(memory_offset, code);

        // update frame pointer, if we have a stack frame (e.g. fp > 0)
        if fp > 0 {
            let fp_code_size = add_usize(fp, CallFrame::code_size_offset());

            let length = Word::from_be_bytes(self.mem_read_bytes(fp_code_size)?)
                .checked_add(length as Word)
                .ok_or(PanicReason::MemoryAccess)?;

            self.memory.write_slice(fp_code_size, &length.to_be_bytes());
        }

        self.registers[RegId::SSP] = self.registers[RegId::SP];

        Ok(())
    }

    pub(crate) fn burn(&mut self, a: Word) -> Result<(), RuntimeError> {
        let contract = ContractId::from(self.mem_read_bytes(a)?);
        let asset_id = AssetId::from(*contract);

        let balance = balance(&self.storage, &contract, &asset_id)?;
        let balance = balance.checked_sub(a).ok_or(PanicReason::NotEnoughBalance)?;

        self.storage
            .merkle_contract_asset_id_balance_insert(&contract, &asset_id, balance)
            .map_err(RuntimeError::from_io)?;

        Ok(())
    }

    pub(crate) fn mint(&mut self, a: Word) -> Result<(), RuntimeError> {
        let contract = ContractId::from(self.mem_read_bytes(a)?);
        let asset_id = AssetId::from(*contract);

        let balance = balance(&self.storage, &contract, &asset_id)?;
        let balance = checked_add_word(balance, a)?;

        self.storage
            .merkle_contract_asset_id_balance_insert(&contract, &asset_id, balance)
            .map_err(RuntimeError::from_io)?;

        Ok(())
    }

    pub(crate) fn code_copy(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let contract = ContractId::from(self.mem_read_bytes(b)?);
        let dst_range = MemoryRange::try_new(a, d)?;
        let src_range = MemoryRange::try_new(c, d)?;

        if !self.tx.input_contracts().any(|input| *input == contract) {
            self.panic_context = PanicContext::ContractId(contract);
            return Err(PanicReason::ContractNotInInputs.into());
        }

        let contract = super::contract::contract(&self.storage, &contract)?.into_owned();

        let dst = self.mem_write_range(&dst_range)?;
        dst.fill(0);

        if contract.as_ref().len() >= src_range.end {
            let src_data = &contract.as_ref()[src_range.as_usizes()];
            dst.copy_from_slice(src_data);
        }

        Ok(())
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let height = u32::try_from(b).map_err(|_| PanicReason::ArithmeticOverflow)?.into();
        let hash = self.storage.block_hash(height).map_err(|e| e.into())?;
        self.mem_write_bytes(a, &hash)?;
        Ok(())
    }

    pub(crate) fn block_height(&mut self, ra: RegisterId) -> Result<(), RuntimeError> {
        let (_, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        *result = self
            .context
            .block_height()
            .map(|h| *h as Word)
            .ok_or(PanicReason::TransactionValidity)?;

        Ok(())
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> Result<(), RuntimeError> {
        let cb = self.storage.coinbase().map_err(RuntimeError::from_io)?;
        self.mem_write_bytes(a, &cb)?;
        Ok(())
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> Result<(), RuntimeError> {
        let contract_id = ContractId::from(self.mem_read_bytes(b)?);

        let (_, root) = self
            .storage
            .storage_contract_root(&contract_id)
            .transpose()
            .ok_or(PanicReason::ContractNotFound)?
            .map_err(RuntimeError::from_io)?
            .into_owned();

        self.mem_write_bytes(a, &root)?;

        Ok(())
    }

    pub(crate) fn code_size(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(ra)?;
        let contract_id = ContractId::from(self.mem_read_bytes(b)?);

        let len = contract_size(&self.storage, &contract_id)?;
        self.dependent_gas_charge(self.gas_costs.csiz, len)?;
        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk] = len;

        Ok(())
    }

    pub(crate) fn state_clear_qword(&mut self, a: Word, rb: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(rb)?;
        let contract_id = self.internal_contract()?;

        let start_key = Bytes32::from(self.mem_read_bytes(a)?);
        let num_slots = c;

        let all_previously_set = self
            .storage
            .merkle_contract_state_remove_range(&contract_id, &start_key, num_slots)
            .map_err(RuntimeError::from_io)?
            .is_some();

        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk] = all_previously_set as Word;

        Ok(())
    }

    pub(crate) fn state_read_word(&mut self, ra: RegisterId, rb: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let wrk_a = WriteRegKey::try_from(ra)?;
        let wrk_b = WriteRegKey::try_from(rb)?;

        let key = Bytes32::from(self.mem_read_bytes(c)?);
        let contract = self.internal_contract()?;

        let value = self
            .storage
            .merkle_contract_state(&contract, &key)
            .map_err(RuntimeError::from_io)?
            .map(|state| bytes::from_array(state.as_ref().borrow()))
            .map(Word::from_be_bytes);

        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk_a] = value.unwrap_or(0);
        w[wrk_b] = value.is_some() as Word;

        Ok(())
    }

    pub(crate) fn state_read_qword(&mut self, a: Word, rb: RegisterId, c: Word, d: Word) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(rb)?;
        let contract_id = self.internal_contract()?;
        let src_key = Bytes32::from(self.mem_read_bytes(c)?);

        let mut all_set = true;
        let result: Vec<u8> = self
            .storage
            .merkle_contract_state_range(&contract_id, &src_key, d)
            .map_err(RuntimeError::from_io)?
            .into_iter()
            .flat_map(|bytes| match bytes {
                Some(bytes) => **bytes,
                None => {
                    all_set = false;
                    *Bytes32::zeroed()
                }
            })
            .collect();

        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk] = all_set as Word;

        self.mem_write_slice(a, &result)?;

        Ok(())
    }

    pub(crate) fn state_write_word(&mut self, a: Word, rb: RegisterId, c: Word) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(rb)?;
        let key = Bytes32::from(self.mem_read_bytes(a)?);
        let contract = self.internal_contract()?;

        let mut value = Bytes32::default();

        value[..WORD_SIZE].copy_from_slice(&c.to_be_bytes());

        let result = self
            .storage
            .merkle_contract_state_insert(&contract, &key, &value)
            .map_err(RuntimeError::from_io)?;

        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk] = result.is_some() as Word;

        Ok(())
    }

    pub(crate) fn state_write_qword(&mut self, a: Word, rb: RegisterId, c: Word, d: Word) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(rb)?;
        let contract_id = self.internal_contract();
        let source_addresses = MemoryRange::try_new(c, Bytes32::LEN.saturating_mul(d as usize) as Word)?;

        let contract_id = &contract_id?;
        let destination_key = Bytes32::from(self.mem_read_bytes(a)?);

        // TODO: switch to stdlib array_chunks when it's stable: https://github.com/rust-lang/rust/issues/100450
        let values: Vec<_> =
            itermore::IterArrayChunks::array_chunks(self.mem_read_range(&source_addresses)?.iter().copied())
                .map(Bytes32::from)
                .collect();

        let any_none = self
            .storage
            .merkle_contract_state_insert_range(contract_id, &destination_key, &values)
            .map_err(RuntimeError::from_io)?
            .is_some();

        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk] = any_none as Word;

        Ok(())
    }

    pub(crate) fn timestamp(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(ra)?;
        let block_height = self.get_block_height()?;

        let b = u32::try_from(b).map_err(|_| PanicReason::ArithmeticOverflow)?.into();
        (b <= block_height)
            .then_some(())
            .ok_or(PanicReason::TransactionValidity)?;

        let (_, mut w) = split_registers(&mut self.registers);
        w[wrk] = self.storage.timestamp(b).map_err(|e| e.into())?;

        Ok(())
    }

    pub(crate) fn message_output(
        &mut self,
        recipient_mem_address: Word,
        msg_data_ptr: Word,
        msg_data_len: Word,
        amount_coins_to_send: Word,
    ) -> Result<(), RuntimeError> {
        let recipient = Address::from(self.mem_read_bytes(recipient_mem_address)?);

        if msg_data_len > self.params.max_message_data_length {
            return Err(RuntimeError::Recoverable(PanicReason::MessageDataTooLong));
        }

        let msg_data: Vec<u8> = self.mem_read(msg_data_ptr, msg_data_len)?.to_vec();

        // validations passed, perform the mutations
        if let Some(source_contract) = self.current_contract()? {
            balance_decrease(
                &mut self.storage,
                &source_contract,
                &AssetId::BASE,
                amount_coins_to_send,
            )?;
        } else {
            base_asset_balance_sub(&mut self.balances, &mut self.memory, amount_coins_to_send)?;
        }

        let sender = Address::from(self.mem_read_bytes(self.registers[RegId::FP])?);
        let txid = tx_id(&self.memory);

        let receipt = Receipt::message_out_from_tx_output(
            &txid,
            self.receipts.len() as Word,
            sender,
            recipient,
            amount_coins_to_send,
            msg_data,
        );

        self.append_receipt(receipt);

        Ok(())
    }
}
