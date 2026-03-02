use crate::{
    call::CallFrame,
    constraints::reg_key::*,
    consts::*,
    convert,
    error::{
        IoResult,
        RuntimeError,
    },
    interpreter::{
        ExecutableTransaction,
        Interpreter,
        Memory,
        contract::{
            balance,
            balance_decrease,
            blob_size,
            contract_size,
        },
        internal::{
            base_asset_balance_sub,
            inc_pc,
            tx_id,
        },
        memory::{
            OwnershipRegisters,
            copy_from_storage_zero_fill,
        },
    },
    storage::{
        BlobData,
        ContractsRawCode,
        InterpreterStorage,
    },
    verification::Verifier,
};
use alloc::vec::Vec;
use fuel_asm::{
    Imm06,
    PanicReason,
    RegId,
};
use fuel_tx::{
    BlobId,
    ContractIdExt,
    Receipt,
    consts::BALANCE_ENTRY_SIZE,
};
use fuel_types::{
    Address,
    Bytes32,
    ContractId,
    SubAssetId,
    Word,
    bytes::{
        self,
        padded_len_word,
    },
};

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
    V: Verifier,
{
    /// Loads contract ID pointed by `contract_id_addr`, and then for that contract,
    /// copies `length_unpadded` bytes from it starting from offset `contract_offset` into
    /// the stack.
    ///
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(
        &mut self,
        addr: Word,
        offset: Word,
        length_unpadded: Word,
        mode: Imm06,
    ) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().ldc();
        // Charge only for the `base` execution.
        // We will charge for the contracts size in the `load_contract_code`.
        self.gas_charge(gas_cost.base())?;

        let ssp = self.registers[RegId::SSP];
        let sp = self.registers[RegId::SP];
        let region_start = ssp;

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into())
        }

        match mode.to_u8() {
            0 => {
                // only blobs are allowed in predicates
                if self.context.is_predicate() {
                    return Err(PanicReason::ContractInstructionNotAllowed.into())
                }

                let contract_id = ContractId::from(self.memory().read_bytes(addr)?);

                let length = padded_len_word(length_unpadded)
                    .ok_or(PanicReason::MemoryOverflow)?;

                if length > self.contract_max_size() {
                    return Err(PanicReason::ContractMaxSize.into())
                }

                self.verifier.check_contract_in_inputs(
                    &mut self.panic_context,
                    &self.input_contracts,
                    &contract_id,
                )?;

                // Fetch the storage contract
                let contract_len = contract_size(&self.storage, &contract_id)?;
                let charge_len = core::cmp::max(contract_len as u64, length);
                self.dependent_gas_charge_without_base(gas_cost, charge_len)?;

                let new_sp = ssp.saturating_add(length);
                self.memory_mut().grow_stack(new_sp)?;

                // Set up ownership registers for the copy using old ssp
                let owner = OwnershipRegisters::only_allow_stack_write(
                    new_sp,
                    self.registers[RegId::SSP],
                    self.registers[RegId::HP],
                );

                // Mark stack space as allocated
                self.registers[RegId::SP] = new_sp;
                self.registers[RegId::SSP] = new_sp;

                copy_from_storage_zero_fill::<ContractsRawCode, _>(
                    self.memory.as_mut(),
                    owner,
                    &self.storage,
                    region_start,
                    length,
                    &contract_id,
                    offset,
                    contract_len,
                    PanicReason::ContractNotFound,
                )?;

                // Update frame code size, if we have a stack frame (i.e. fp > 0)
                if self.context.is_internal() {
                    let code_size_ptr = (self.registers[RegId::FP])
                        .saturating_add(CallFrame::code_size_offset() as Word);
                    let old_code_size =
                        Word::from_be_bytes(self.memory().read_bytes(code_size_ptr)?);
                    let old_code_size = padded_len_word(old_code_size)
                        .ok_or(PanicReason::MemoryOverflow)?;
                    let new_code_size = old_code_size
                        .checked_add(length as Word)
                        .ok_or(PanicReason::MemoryOverflow)?;

                    self.memory_mut().write_bytes_noownerchecks(
                        code_size_ptr,
                        new_code_size.to_be_bytes(),
                    )?;
                }
            }
            1 => {
                let blob_id = BlobId::from(self.memory().read_bytes(addr)?);

                let length = bytes::padded_len_word(length_unpadded).unwrap_or(Word::MAX);

                let blob_len = blob_size(&self.storage, &blob_id)?;

                // Fetch the storage blob
                let charge_len = core::cmp::max(blob_len as u64, length);
                self.dependent_gas_charge_without_base(gas_cost, charge_len)?;

                let new_sp = ssp.saturating_add(length);
                self.memory_mut().grow_stack(new_sp)?;

                // Set up ownership registers for the copy using old ssp
                let owner = OwnershipRegisters::only_allow_stack_write(
                    new_sp,
                    self.registers[RegId::SSP],
                    self.registers[RegId::HP],
                );

                // Mark stack space as allocated
                self.registers[RegId::SP] = new_sp;
                self.registers[RegId::SSP] = new_sp;

                // Copy the code.
                copy_from_storage_zero_fill::<BlobData, _>(
                    self.memory.as_mut(),
                    owner,
                    &self.storage,
                    region_start,
                    length,
                    &blob_id,
                    offset,
                    blob_len,
                    PanicReason::BlobNotFound,
                )?;

                // Update frame code size, if we have a stack frame (i.e. fp > 0)
                if self.context.is_internal() {
                    let code_size_ptr = self.registers[RegId::FP]
                        .saturating_add(CallFrame::code_size_offset() as Word);
                    let old_code_size =
                        Word::from_be_bytes(self.memory().read_bytes(code_size_ptr)?);
                    let old_code_size = padded_len_word(old_code_size)
                        .expect("Code size cannot overflow with padding");
                    let new_code_size = old_code_size
                        .checked_add(length as Word)
                        .ok_or(PanicReason::MemoryOverflow)?;

                    self.memory_mut().write_bytes_noownerchecks(
                        code_size_ptr,
                        new_code_size.to_be_bytes(),
                    )?;
                }
            }
            2 => {
                let dst = ssp;

                if length_unpadded == 0 {
                    inc_pc(self.registers.pc_mut())?;
                    return Ok(())
                }

                let length = bytes::padded_len_word(length_unpadded).unwrap_or(Word::MAX);
                let length_padding = length.saturating_sub(length_unpadded);

                // Fetch the storage blob
                let charge_len = length;
                self.dependent_gas_charge_without_base(gas_cost, charge_len)?;

                let new_sp = ssp.saturating_add(length);
                self.memory_mut().grow_stack(new_sp)?;

                // Set up ownership registers for the copy using old ssp
                let owner = OwnershipRegisters::only_allow_stack_write(
                    new_sp,
                    ssp,
                    self.registers[RegId::HP],
                );
                let src = addr.saturating_add(offset);

                // Copy the code
                self.memory_mut()
                    .memcopy(dst, src, length_unpadded, owner)?;

                // Write padding
                if length_padding > 0 {
                    self.memory_mut()
                        .write(
                            owner,
                            dst.saturating_add(length_unpadded),
                            length_padding,
                        )?
                        .fill(0);
                }

                // Mark stack space as allocated
                self.registers[RegId::SP] = new_sp;
                self.registers[RegId::SSP] = new_sp;

                // Update frame code size, if we have a stack frame (i.e. fp > 0)
                if self.context.is_internal() {
                    let code_size_ptr = self.registers[RegId::FP]
                        .saturating_add(CallFrame::code_size_offset() as Word);
                    let old_code_size =
                        Word::from_be_bytes(self.memory().read_bytes(code_size_ptr)?);
                    let old_code_size = padded_len_word(old_code_size)
                        .expect("Code size cannot overflow with padding");
                    let new_code_size = old_code_size
                        .checked_add(length as Word)
                        .ok_or(PanicReason::MemoryOverflow)?;

                    self.memory_mut().write_bytes_noownerchecks(
                        code_size_ptr,
                        new_code_size.to_be_bytes(),
                    )?;
                }
            }
            _ => return Err(PanicReason::InvalidImmediateValue.into()),
        }

        inc_pc(self.registers.pc_mut())?;
        Ok(())
    }

    pub(crate) fn burn(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let contract_id = self.internal_contract()?;
        let sub_id = SubAssetId::new(self.memory().read_bytes(b)?);
        let asset_id = contract_id.asset_id(&sub_id);

        let balance = balance(&self.storage, &contract_id, &asset_id)?;
        let balance = balance
            .checked_sub(a)
            .ok_or(PanicReason::NotEnoughBalance)?;

        self.storage
            .contract_asset_id_balance_insert(&contract_id, &asset_id, balance)
            .map_err(RuntimeError::Storage)?;

        let receipt = Receipt::burn(
            sub_id,
            contract_id,
            a,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );
        self.receipts.push(receipt)?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }

    pub(crate) fn mint(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        {
            let contract_id = self.internal_contract()?;
            let sub_id = SubAssetId::new(self.memory().read_bytes(b)?);
            let asset_id = contract_id.asset_id(&sub_id);

            let balance = balance(&self.storage, &contract_id, &asset_id)?;
            let balance = balance.checked_add(a).ok_or(PanicReason::BalanceOverflow)?;

            let old_value = self
                .storage
                .contract_asset_id_balance_replace(&contract_id, &asset_id, balance)
                .map_err(RuntimeError::Storage)?;

            if old_value.is_none() {
                // New data was written, charge gas for it
                self.gas_charge(
                    (BALANCE_ENTRY_SIZE as u64).saturating_mul(new_storage_gas_per_byte),
                )?;
            }

            let receipt = Receipt::mint(
                sub_id,
                contract_id,
                a,
                self.registers[RegId::PC],
                self.registers[RegId::IS],
            );

            self.receipts.push(receipt)?;

            Ok(inc_pc(self.registers.pc_mut())?)
        }
    }

    pub(crate) fn code_copy(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().ccp();
        // Charge only for the `base` execution.
        // We will charge for the contract's size in the `code_copy`.
        self.gas_charge(gas_cost.base())?;
        let owner = self.ownership_registers();
        {
            let contract_id = ContractId::from(self.memory().read_bytes(b)?);

            self.memory_mut().write(owner, a, d)?;
            self.verifier.check_contract_in_inputs(
                &mut self.panic_context,
                &self.input_contracts,
                &contract_id,
            )?;

            let contract_len = contract_size(&self.storage, &contract_id)?;
            let charge_len = core::cmp::max(contract_len as u64, d);
            self.dependent_gas_charge_without_base(gas_cost, charge_len)?;

            copy_from_storage_zero_fill::<ContractsRawCode, _>(
                self.memory.as_mut(),
                owner,
                &self.storage,
                a,
                d,
                &contract_id,
                c,
                contract_len,
                PanicReason::ContractNotFound,
            )?;

            Ok(inc_pc(self.registers.pc_mut())?)
        }
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let height = u32::try_from(b)
            .map_err(|_| PanicReason::InvalidBlockHeight)?
            .into();
        let hash = self
            .storage
            .block_hash(height)
            .map_err(RuntimeError::Storage)?;
        let owner = self.ownership_registers();
        self.memory_mut().write_bytes(owner, a, *hash)?;
        inc_pc(self.registers.pc_mut())?;
        Ok(())
    }

    pub(crate) fn block_height(&mut self, ra: RegId) -> IoResult<(), S::DataError> {
        let height = self
            .context
            .block_height()
            .ok_or(PanicReason::TransactionValidity)?;
        self.write_user_register_legacy(ra, (*height) as Word)?;
        inc_pc(self.registers.pc_mut())?;
        Ok(())
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        let coinbase = self.storage.coinbase().map_err(RuntimeError::Storage)?;
        self.memory_mut().write_bytes(owner, a, *coinbase)?;
        inc_pc(self.registers.pc_mut())?;
        Ok(())
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().croo();
        self.gas_charge(gas_cost.base())?;
        let owner = self.ownership_registers();
        {
            self.memory_mut().write_noownerchecks(a, Bytes32::LEN)?;

            let contract_id = ContractId::new(self.memory().read_bytes(b)?);

            self.verifier.check_contract_in_inputs(
                &mut self.panic_context,
                &self.input_contracts,
                &contract_id,
            )?;

            let len = contract_size(&self.storage, &contract_id)?;
            self.dependent_gas_charge_without_base(gas_cost, len as u64)?;
            let root = self
                .storage
                .storage_contract(&contract_id)
                .transpose()
                .ok_or(PanicReason::ContractNotFound)?
                .map_err(RuntimeError::Storage)?
                .root();

            self.memory_mut().write_bytes(owner, a, *root)?;

            Ok(inc_pc(self.registers.pc_mut())?)
        }
    }

    pub(crate) fn code_size(&mut self, ra: RegId, b: Word) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().csiz();
        // Charge only for the `base` execution.
        self.gas_charge(gas_cost.base())?;
        let contract_id = ContractId::new(self.memory().read_bytes(b)?);

        self.verifier.check_contract_in_inputs(
            &mut self.panic_context,
            &self.input_contracts,
            &contract_id,
        )?;

        let len = contract_size(&self.storage, &contract_id)?;
        self.dependent_gas_charge_without_base(gas_cost, len as u64)?;
        self.write_user_register_legacy(ra, len as u64)?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }

    pub(crate) fn state_clear_qword(
        &mut self,
        start_storage_key_pointer: Word,
        r_result: RegId,
        num_slots: Word,
    ) -> IoResult<(), S::DataError> {
        let contract_id = self.internal_contract()?;
        let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;
        let start_key =
            Bytes32::new(self.memory().read_bytes(start_storage_key_pointer)?);

        let all_previously_set = self
            .storage
            .contract_state_remove_range(&contract_id, &start_key, num_slots)
            .map_err(RuntimeError::Storage)?
            .is_some();

        self.write_user_register_legacy(r_result, all_previously_set as Word)?;
        inc_pc(self.registers.pc_mut())?;
        Ok(())
    }

    pub(crate) fn state_read_word(
        &mut self,
        r_result: RegId,
        r_got_result: RegId,
        key_ptr: Word,
        offset: Imm06,
    ) -> IoResult<(), S::DataError> {
        let key = Bytes32::new(self.memory().read_bytes(key_ptr)?);
        let contract = self.internal_contract()?;
        let value: Option<Word> = self
            .storage
            .contract_state(&contract, &key)
            .map_err(RuntimeError::Storage)?
            .map(|bytes| {
                let offset = offset.to_u8() as usize;
                let offset_bytes = offset.saturating_mul(WORD_SIZE);
                let end_bytes = offset_bytes.saturating_add(WORD_SIZE);

                let data = bytes.as_ref().as_ref();
                if (data.len() as u64) < (end_bytes as u64) {
                    return Err(PanicReason::StorageOutOfBounds);
                }

                let mut buf = [0u8; WORD_SIZE];
                buf.copy_from_slice(&data[offset_bytes..end_bytes]);
                Ok(Word::from_be_bytes(buf))
            })
            .transpose()?;

        self.write_user_register_legacy(r_result, value.unwrap_or(0))?;
        self.write_user_register_legacy(r_got_result, value.is_some() as Word)?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }

    pub(crate) fn state_read_qword(
        &mut self,
        destination_pointer: Word,
        r_result: RegId,
        origin_key_pointer: Word,
        num_slots: Word,
    ) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        let contract_id = self.internal_contract()?;
        let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;
        let slots_len = Bytes32::LEN.saturating_mul(num_slots);
        let origin_key = Bytes32::new(self.memory().read_bytes(origin_key_pointer)?);
        let dst = self
            .memory
            .as_mut()
            .write(owner, destination_pointer, slots_len)?;

        let mut all_set = true;
        let mut result: Vec<u8> = Vec::with_capacity(slots_len);

        for slot in self
            .storage
            .contract_state_range(&contract_id, &origin_key, num_slots)
            .map_err(RuntimeError::Storage)?
        {
            if let Some(bytes) = slot {
                if bytes.0.len() != Bytes32::LEN {
                    return Err(PanicReason::StorageOutOfBounds.into());
                }
                result.extend(bytes.into_owned());
            } else {
                all_set = false;
                result.extend([0; 32]);
            }
        }

        dst.copy_from_slice(&result);
        self.write_user_register_legacy(r_result, all_set as Word)?;

        inc_pc(self.registers.pc_mut())?;

        Ok(())
    }

    pub(crate) fn state_write_word(
        &mut self,
        a: Word,
        rb: RegId,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        {
            let key = Bytes32::new(self.memory().read_bytes(a)?);
            let contract = self.internal_contract()?;

            let mut value = Bytes32::zeroed();
            value.as_mut()[..WORD_SIZE].copy_from_slice(&c.to_be_bytes());

            let prev = self
                .storage
                .contract_state_replace(&contract, &key, value.as_ref())
                .map_err(RuntimeError::Storage)?;

            self.write_user_register_legacy(rb, prev.is_none() as Word)?;

            if prev.is_none() {
                // New data was written, charge gas for it
                self.gas_charge(
                    (Bytes32::LEN as u64)
                        .saturating_mul(2)
                        .saturating_mul(new_storage_gas_per_byte),
                )?;
            }

            Ok(inc_pc(self.registers.pc_mut())?)
        }
    }

    pub(crate) fn state_write_qword(
        &mut self,
        starting_storage_key_pointer: Word,
        r_result: RegId,
        source_pointer: Word,
        num_slots: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_per_byte = self.gas_costs().new_storage_per_byte();
        let contract_id = self.internal_contract()?;
        let memory = self.memory.as_ref();
        let destination_key =
            Bytes32::new(memory.read_bytes(starting_storage_key_pointer)?);

        let values = memory
            .read(
                source_pointer,
                (Bytes32::LEN as Word).saturating_mul(num_slots),
            )?
            .chunks_exact(Bytes32::LEN);

        let unset_count = self
            .storage
            .contract_state_insert_range(&contract_id, &destination_key, values)
            .map_err(RuntimeError::Storage)?;
        self.write_user_register_legacy(r_result, unset_count as Word)?;

        if unset_count > 0 {
            // New data was written, charge gas for it
            self.gas_charge(
                (unset_count as u64)
                    .saturating_mul(2)
                    .saturating_mul(Bytes32::LEN as u64)
                    .saturating_mul(new_storage_per_byte),
            )?;
        }

        inc_pc(self.registers.pc_mut())?;
        Ok(())
    }

    pub(crate) fn timestamp(&mut self, ra: RegId, b: Word) -> IoResult<(), S::DataError> {
        let block_height = self.get_block_height()?;
        let b = u32::try_from(b)
            .map_err(|_| PanicReason::InvalidBlockHeight)?
            .into();
        (b <= block_height)
            .then_some(())
            .ok_or(PanicReason::TransactionValidity)?;

        let result = self.storage.timestamp(b).map_err(RuntimeError::Storage)?;
        self.write_user_register_legacy(ra, result)?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }

    pub(crate) fn message_output(
        &mut self,
        recipient_mem_address: Word,
        msg_data_ptr: Word,
        msg_data_len: Word,
        amount_coins_to_send: Word,
    ) -> IoResult<(), S::DataError> {
        let base_asset_id = self.interpreter_params.base_asset_id;
        if msg_data_len > self.max_message_data_length() {
            return Err(RuntimeError::Recoverable(PanicReason::MessageDataTooLong));
        }

        let msg_data = self.memory().read(msg_data_ptr, msg_data_len)?.to_vec();
        let recipient = Address::new(self.memory().read_bytes(recipient_mem_address)?);
        let sender = Address::new(self.memory().read_bytes(self.registers[RegId::FP])?);

        // validations passed, perform the mutations

        if let Some(source_contract) = self.frames.last().map(|frame| frame.to()).copied()
        {
            balance_decrease(
                &mut self.storage,
                &source_contract,
                &base_asset_id,
                amount_coins_to_send,
            )?;
        } else {
            base_asset_balance_sub(
                &base_asset_id,
                &mut self.balances,
                self.memory.as_mut(),
                amount_coins_to_send,
            )?;
        }

        let txid = tx_id(self.memory());
        let receipt = Receipt::message_out(
            &txid,
            self.receipts.len() as Word,
            sender,
            recipient,
            amount_coins_to_send,
            msg_data,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }
}
