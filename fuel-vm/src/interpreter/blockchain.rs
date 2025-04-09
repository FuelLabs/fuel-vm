use crate::{
    call::CallFrame,
    constraints::reg_key::*,
    consts::*,
    context::Context,
    convert,
    error::{
        IoResult,
        RuntimeError,
        SimpleResult,
    },
    interpreter::{
        contract::{
            balance,
            balance_decrease,
            blob_size,
            contract_size,
        },
        gas::{
            dependent_gas_charge_without_base,
            gas_charge,
        },
        internal::{
            base_asset_balance_sub,
            inc_pc,
            internal_contract,
            tx_id,
        },
        memory::{
            copy_from_storage_zero_fill,
            OwnershipRegisters,
        },
        receipts::ReceiptsCtx,
        ExecutableTransaction,
        Interpreter,
        Memory,
        MemoryInstance,
        RuntimeBalances,
    },
    storage::{
        BlobData,
        ContractsAssetsStorage,
        ContractsRawCode,
        ContractsStateData,
        InterpreterStorage,
    },
    verification::Verifier,
};
use alloc::{
    collections::BTreeSet,
    vec::Vec,
};
use fuel_asm::{
    Imm06,
    PanicReason,
    RegId,
};
use fuel_storage::StorageSize;
use fuel_tx::{
    consts::BALANCE_ENTRY_SIZE,
    BlobId,
    ContractIdExt,
    DependentCost,
    Receipt,
};
use fuel_types::{
    bytes::{
        self,
        padded_len_word,
    },
    Address,
    AssetId,
    BlockHeight,
    Bytes32,
    ContractId,
    SubAssetId,
    Word,
};

use super::PanicContext;

#[cfg(test)]
mod code_tests;
#[cfg(test)]
mod croo_tests;
#[cfg(test)]
mod other_tests;
#[cfg(test)]
mod smo_tests;
#[cfg(test)]
mod test;

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
        let contract_max_size = self.contract_max_size();
        let (
            SystemRegisters {
                cgas,
                ggas,
                ssp,
                sp,
                hp,
                fp,
                pc,
                ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let input = LoadContractCodeCtx {
            memory: self.memory.as_mut(),
            context: &self.context,
            storage: &mut self.storage,
            contract_max_size,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            gas_cost,
            cgas,
            ggas,
            ssp,
            sp,
            hp: hp.as_ref(),
            fp: fp.as_ref(),
            pc,
            verifier: &mut self.verifier,
        };

        match mode.to_u8() {
            0 => input.load_contract_code(addr, offset, length_unpadded),
            1 => input.load_blob_code(addr, offset, length_unpadded),
            2 => input.load_memory_code(addr, offset, length_unpadded),
            _ => Err(PanicReason::InvalidImmediateValue.into()),
        }
    }

    pub(crate) fn burn(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let (SystemRegisters { fp, pc, is, .. }, _) =
            split_registers(&mut self.registers);
        BurnCtx {
            storage: &mut self.storage,
            context: &self.context,
            memory: self.memory.as_ref(),
            receipts: &mut self.receipts,
            fp: fp.as_ref(),
            pc,
            is: is.as_ref(),
        }
        .burn(a, b)
    }

    pub(crate) fn mint(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        let (
            SystemRegisters {
                cgas,
                ggas,
                fp,
                pc,
                is,
                ..
            },
            _,
        ) = split_registers(&mut self.registers);
        MintCtx {
            storage: &mut self.storage,
            context: &self.context,
            memory: self.memory.as_ref(),
            receipts: &mut self.receipts,

            new_storage_gas_per_byte,
            cgas,
            ggas,
            fp: fp.as_ref(),
            pc,
            is: is.as_ref(),
        }
        .mint(a, b)
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
        let (SystemRegisters { cgas, ggas, pc, .. }, _) =
            split_registers(&mut self.registers);
        let input = CodeCopyCtx {
            memory: self.memory.as_mut(),
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            storage: &mut self.storage,
            owner,
            gas_cost,
            cgas,
            ggas,
            pc,
            verifier: &mut self.verifier,
        };
        input.code_copy(a, b, c, d)
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        block_hash(
            &self.storage,
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
        )
    }

    pub(crate) fn block_height(&mut self, ra: RegId) -> IoResult<(), S::DataError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        Ok(block_height(&self.context, pc, result)?)
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        coinbase(
            &self.storage,
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
        )
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().croo();
        self.gas_charge(gas_cost.base())?;
        let owner = self.ownership_registers();
        let (SystemRegisters { cgas, ggas, pc, .. }, _) =
            split_registers(&mut self.registers);
        CodeRootCtx {
            memory: self.memory.as_mut(),
            storage: &mut self.storage,
            gas_cost,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            cgas,
            ggas,
            owner,
            pc,
            verifier: &mut self.verifier,
        }
        .code_root(a, b)
    }

    pub(crate) fn code_size(&mut self, ra: RegId, b: Word) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().csiz();
        // Charge only for the `base` execution.
        // We will charge for the contracts size in the `code_size`.
        self.gas_charge(gas_cost.base())?;
        let (SystemRegisters { cgas, ggas, pc, .. }, mut w) =
            split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = CodeSizeCtx {
            memory: self.memory.as_mut(),
            storage: &mut self.storage,
            gas_cost,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            cgas,
            ggas,
            pc,
            verifier: &mut self.verifier,
        };
        input.code_size(result, b)
    }

    pub(crate) fn state_clear_qword(
        &mut self,
        a: Word,
        rb: RegId,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let contract_id = self.internal_contract();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateClearQWord::new(a, c)?;
        let Self {
            ref mut storage,
            ref memory,
            ..
        } = self;

        state_clear_qword(&contract_id?, storage, memory.as_ref(), pc, result, input)
    }

    pub(crate) fn state_read_word(
        &mut self,
        ra: RegId,
        rb: RegId,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let (SystemRegisters { fp, pc, .. }, mut w) =
            split_registers(&mut self.registers);
        let (result, got_result) = w
            .get_mut_two(WriteRegKey::try_from(ra)?, WriteRegKey::try_from(rb)?)
            .ok_or(RuntimeError::Recoverable(
                PanicReason::ReservedRegisterNotWritable,
            ))?;
        let Self {
            ref mut storage,
            ref memory,
            ref context,
            ..
        } = self;
        state_read_word(
            StateReadWordCtx {
                storage,
                memory: memory.as_ref(),
                context,
                fp: fp.as_ref(),
                pc,
            },
            result,
            got_result,
            c,
        )
    }

    pub(crate) fn state_read_qword(
        &mut self,
        a: Word,
        rb: RegId,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        let (SystemRegisters { pc, fp, .. }, mut w) =
            split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let Self {
            ref storage,
            ref context,
            ref mut memory,
            ..
        } = self;

        state_read_qword(
            storage,
            context,
            memory.as_mut(),
            pc,
            fp.as_ref(),
            owner,
            result,
            StateReadQWordParams {
                destination_pointer: a,
                origin_key_pointer: c,
                num_slots: d,
            },
        )
    }

    pub(crate) fn state_write_word(
        &mut self,
        a: Word,
        rb: RegId,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        let (
            SystemRegisters {
                cgas, ggas, fp, pc, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let exists = &mut w[WriteRegKey::try_from(rb)?];
        let Self {
            ref mut storage,
            ref memory,
            ref context,
            ..
        } = self;
        state_write_word(
            StateWriteWordCtx {
                storage,
                memory: memory.as_ref(),
                context,
                new_storage_gas_per_byte,
                cgas,
                ggas,
                fp: fp.as_ref(),
                pc,
            },
            a,
            exists,
            c,
        )
    }

    pub(crate) fn state_write_qword(
        &mut self,
        a: Word,
        rb: RegId,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_per_byte = self.gas_costs().new_storage_per_byte();
        let contract_id = self.internal_contract();
        let (SystemRegisters { cgas, ggas, pc, .. }, mut w) =
            split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateWriteQWord {
            starting_storage_key_pointer: a,
            source_pointer: c,
            num_slots: d,
        };

        let Self {
            ref mut storage,
            ref mut memory,
            ..
        } = self;

        state_write_qword(
            &contract_id?,
            storage,
            memory.as_ref(),
            new_storage_per_byte,
            cgas,
            ggas,
            pc,
            result,
            input,
        )
    }

    pub(crate) fn timestamp(&mut self, ra: RegId, b: Word) -> IoResult<(), S::DataError> {
        let block_height = self.get_block_height()?;
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        timestamp(&self.storage, block_height, pc, result, b)
    }

    pub(crate) fn message_output(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let base_asset_id = self.interpreter_params.base_asset_id;
        let max_message_data_length = self.max_message_data_length();
        let (SystemRegisters { fp, pc, .. }, _) = split_registers(&mut self.registers);
        let input = MessageOutputCtx {
            base_asset_id,
            max_message_data_length,
            memory: self.memory.as_mut(),
            receipts: &mut self.receipts,
            balances: &mut self.balances,
            storage: &mut self.storage,
            current_contract: self.frames.last().map(|frame| frame.to()).copied(),
            fp: fp.as_ref(),
            pc,
            recipient_mem_address: a,
            msg_data_ptr: b,
            msg_data_len: c,
            amount_coins_to_send: d,
        };
        input.message_output()
    }
}

struct LoadContractCodeCtx<'vm, S, V> {
    contract_max_size: u64,
    memory: &'vm mut MemoryInstance,
    context: &'vm Context,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    storage: &'vm S,
    gas_cost: DependentCost,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    ssp: RegMut<'vm, SSP>,
    sp: RegMut<'vm, SP>,
    hp: Reg<'vm, HP>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    verifier: &'vm mut V,
}

impl<S, V> LoadContractCodeCtx<'_, S, V> {
    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    pub(crate) fn load_contract_code(
        mut self,
        contract_id_addr: Word,
        contract_offset: Word,
        length_unpadded: Word,
    ) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
        V: Verifier,
    {
        let ssp = *self.ssp;
        let sp = *self.sp;
        let region_start = ssp;

        // only blobs are allowed in predicates
        if self.context.is_predicate() {
            return Err(PanicReason::ContractInstructionNotAllowed.into())
        }

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into())
        }

        let contract_id = ContractId::from(self.memory.read_bytes(contract_id_addr)?);

        let length =
            padded_len_word(length_unpadded).ok_or(PanicReason::MemoryOverflow)?;

        if length > self.contract_max_size {
            return Err(PanicReason::ContractMaxSize.into())
        }

        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            &contract_id,
        )?;

        // Fetch the storage contract
        let contract_len = contract_size(&self.storage, &contract_id)?;
        let charge_len = core::cmp::max(contract_len as u64, length);
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            self.gas_cost,
            charge_len,
        )?;

        let new_sp = ssp.saturating_add(length);
        self.memory.grow_stack(new_sp)?;

        // Set up ownership registers for the copy using old ssp
        let owner =
            OwnershipRegisters::only_allow_stack_write(new_sp, *self.ssp, *self.hp);

        // Mark stack space as allocated
        *self.sp = new_sp;
        *self.ssp = new_sp;

        copy_from_storage_zero_fill::<ContractsRawCode, _>(
            self.memory,
            owner,
            self.storage,
            region_start,
            length,
            &contract_id,
            contract_offset,
            contract_len,
            PanicReason::ContractNotFound,
        )?;

        // Update frame code size, if we have a stack frame (i.e. fp > 0)
        if self.context.is_internal() {
            let code_size_ptr =
                (*self.fp).saturating_add(CallFrame::code_size_offset() as Word);
            let old_code_size =
                Word::from_be_bytes(self.memory.read_bytes(code_size_ptr)?);
            let old_code_size =
                padded_len_word(old_code_size).ok_or(PanicReason::MemoryOverflow)?;
            let new_code_size = old_code_size
                .checked_add(length as Word)
                .ok_or(PanicReason::MemoryOverflow)?;

            self.memory
                .write_bytes_noownerchecks(code_size_ptr, new_code_size.to_be_bytes())?;
        }

        inc_pc(self.pc)?;

        Ok(())
    }

    /// Loads blob ID pointed by `a`, and then for that blob,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// blob_id = mem[$rA, 32]
    /// blob_code = blobs[blob_id]
    /// mem[$ssp, $rC] = blob_code[$rB, $rC]
    /// ```
    pub(crate) fn load_blob_code(
        mut self,
        blob_id_addr: Word,
        blob_offset: Word,
        length_unpadded: Word,
    ) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
    {
        let ssp = *self.ssp;
        let sp = *self.sp;
        let region_start = ssp;

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into())
        }

        let blob_id = BlobId::from(self.memory.read_bytes(blob_id_addr)?);

        let length = bytes::padded_len_word(length_unpadded).unwrap_or(Word::MAX);

        let blob_len = blob_size(self.storage, &blob_id)?;

        // Fetch the storage blob
        let charge_len = core::cmp::max(blob_len as u64, length);
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            self.gas_cost,
            charge_len,
        )?;

        let new_sp = ssp.saturating_add(length);
        self.memory.grow_stack(new_sp)?;

        // Set up ownership registers for the copy using old ssp
        let owner =
            OwnershipRegisters::only_allow_stack_write(new_sp, *self.ssp, *self.hp);

        // Mark stack space as allocated
        *self.sp = new_sp;
        *self.ssp = new_sp;

        // Copy the code.
        copy_from_storage_zero_fill::<BlobData, _>(
            self.memory,
            owner,
            self.storage,
            region_start,
            length,
            &blob_id,
            blob_offset,
            blob_len,
            PanicReason::BlobNotFound,
        )?;

        // Update frame code size, if we have a stack frame (i.e. fp > 0)
        if self.context.is_internal() {
            let code_size_ptr =
                (*self.fp).saturating_add(CallFrame::code_size_offset() as Word);
            let old_code_size =
                Word::from_be_bytes(self.memory.read_bytes(code_size_ptr)?);
            let old_code_size = padded_len_word(old_code_size)
                .expect("Code size cannot overflow with padding");
            let new_code_size = old_code_size
                .checked_add(length as Word)
                .ok_or(PanicReason::MemoryOverflow)?;

            self.memory
                .write_bytes_noownerchecks(code_size_ptr, new_code_size.to_be_bytes())?;
        }

        inc_pc(self.pc)?;

        Ok(())
    }

    /// Copies `c` bytes from starting the memory `a` and offset `b` into the
    /// stack.
    ///
    /// ```txt
    /// mem[$ssp, $rC] = memory[$rA + $rB, $rC]
    /// ```
    pub(crate) fn load_memory_code(
        mut self,
        input_src_addr: Word,
        input_offset: Word,
        length_unpadded: Word,
    ) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
    {
        let ssp = *self.ssp;
        let sp = *self.sp;
        let dst = ssp;

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into())
        }

        if length_unpadded == 0 {
            inc_pc(self.pc)?;
            return Ok(())
        }

        let length = bytes::padded_len_word(length_unpadded).unwrap_or(Word::MAX);
        let length_padding = length.saturating_sub(length_unpadded);

        // Fetch the storage blob
        let charge_len = length;
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            self.gas_cost,
            charge_len,
        )?;

        let new_sp = ssp.saturating_add(length);
        self.memory.grow_stack(new_sp)?;

        // Set up ownership registers for the copy using old ssp
        let owner = OwnershipRegisters::only_allow_stack_write(new_sp, ssp, *self.hp);
        let src = input_src_addr.saturating_add(input_offset);

        // Copy the code
        self.memory.memcopy(dst, src, length_unpadded, owner)?;

        // Write padding
        if length_padding > 0 {
            self.memory
                .write(owner, dst.saturating_add(length_unpadded), length_padding)?
                .fill(0);
        }

        // Mark stack space as allocated
        *self.sp = new_sp;
        *self.ssp = new_sp;

        // Update frame code size, if we have a stack frame (i.e. fp > 0)
        if self.context.is_internal() {
            let code_size_ptr =
                (*self.fp).saturating_add(CallFrame::code_size_offset() as Word);
            let old_code_size =
                Word::from_be_bytes(self.memory.read_bytes(code_size_ptr)?);
            let old_code_size = padded_len_word(old_code_size)
                .expect("Code size cannot overflow with padding");
            let new_code_size = old_code_size
                .checked_add(length as Word)
                .ok_or(PanicReason::MemoryOverflow)?;

            self.memory
                .write_bytes_noownerchecks(code_size_ptr, new_code_size.to_be_bytes())?;
        }

        inc_pc(self.pc)?;

        Ok(())
    }
}

struct BurnCtx<'vm, S> {
    storage: &'vm mut S,
    context: &'vm Context,
    memory: &'vm MemoryInstance,
    receipts: &'vm mut ReceiptsCtx,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<S> BurnCtx<'_, S>
where
    S: ContractsAssetsStorage,
{
    pub(crate) fn burn(self, a: Word, b: Word) -> IoResult<(), S::Error> {
        let contract_id = internal_contract(self.context, self.fp, self.memory)?;
        let sub_id = SubAssetId::new(self.memory.read_bytes(b)?);
        let asset_id = contract_id.asset_id(&sub_id);

        let balance = balance(self.storage, &contract_id, &asset_id)?;
        let balance = balance
            .checked_sub(a)
            .ok_or(PanicReason::NotEnoughBalance)?;

        self.storage
            .contract_asset_id_balance_insert(&contract_id, &asset_id, balance)
            .map_err(RuntimeError::Storage)?;

        let receipt = Receipt::burn(sub_id, contract_id, a, *self.pc, *self.is);

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct MintCtx<'vm, S> {
    storage: &'vm mut S,
    context: &'vm Context,
    memory: &'vm MemoryInstance,

    receipts: &'vm mut ReceiptsCtx,
    new_storage_gas_per_byte: Word,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<S> MintCtx<'_, S>
where
    S: ContractsAssetsStorage,
{
    pub(crate) fn mint(self, a: Word, b: Word) -> Result<(), RuntimeError<S::Error>> {
        let contract_id = internal_contract(self.context, self.fp, self.memory)?;
        let sub_id = SubAssetId::new(self.memory.read_bytes(b)?);
        let asset_id = contract_id.asset_id(&sub_id);

        let balance = balance(self.storage, &contract_id, &asset_id)?;
        let balance = balance.checked_add(a).ok_or(PanicReason::BalanceOverflow)?;

        let old_value = self
            .storage
            .contract_asset_id_balance_replace(&contract_id, &asset_id, balance)
            .map_err(RuntimeError::Storage)?;

        if old_value.is_none() {
            // New data was written, charge gas for it
            gas_charge(
                self.cgas,
                self.ggas,
                (BALANCE_ENTRY_SIZE as u64).saturating_mul(self.new_storage_gas_per_byte),
            )?;
        }

        let receipt = Receipt::mint(sub_id, contract_id, a, *self.pc, *self.is);

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct CodeCopyCtx<'vm, S, V> {
    memory: &'vm mut MemoryInstance,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    storage: &'vm S,
    owner: OwnershipRegisters,
    gas_cost: DependentCost,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    pc: RegMut<'vm, PC>,
    verifier: &'vm mut V,
}

impl<S, V> CodeCopyCtx<'_, S, V> {
    pub(crate) fn code_copy(
        self,
        dst_addr: Word,
        contract_id_addr: Word,
        contract_offset: Word,
        length: Word,
    ) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
        V: Verifier,
    {
        let contract_id = ContractId::from(self.memory.read_bytes(contract_id_addr)?);

        self.memory.write(self.owner, dst_addr, length)?;
        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            &contract_id,
        )?;

        let contract_len = contract_size(&self.storage, &contract_id)?;
        let charge_len = core::cmp::max(contract_len as u64, length);
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            self.gas_cost,
            charge_len,
        )?;

        copy_from_storage_zero_fill::<ContractsRawCode, _>(
            self.memory,
            self.owner,
            self.storage,
            dst_addr,
            length,
            &contract_id,
            contract_offset,
            contract_len,
            PanicReason::ContractNotFound,
        )?;

        Ok(inc_pc(self.pc)?)
    }
}

pub(crate) fn block_hash<S: InterpreterStorage>(
    storage: &S,
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> IoResult<(), S::DataError> {
    let height = u32::try_from(b)
        .map_err(|_| PanicReason::InvalidBlockHeight)?
        .into();
    let hash = storage.block_hash(height).map_err(RuntimeError::Storage)?;

    memory.write_bytes(owner, a, *hash)?;

    inc_pc(pc)?;
    Ok(())
}

pub(crate) fn block_height(
    context: &Context,
    pc: RegMut<PC>,
    result: &mut Word,
) -> SimpleResult<()> {
    context
        .block_height()
        .map(|h| *h as Word)
        .map(|h| *result = h)
        .ok_or(PanicReason::TransactionValidity)?;

    inc_pc(pc)?;
    Ok(())
}

pub(crate) fn coinbase<S: InterpreterStorage>(
    storage: &S,
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
) -> IoResult<(), S::DataError> {
    let coinbase = storage.coinbase().map_err(RuntimeError::Storage)?;
    memory.write_bytes(owner, a, *coinbase)?;
    inc_pc(pc)?;
    Ok(())
}

struct CodeRootCtx<'vm, S, V> {
    storage: &'vm S,
    memory: &'vm mut MemoryInstance,
    gas_cost: DependentCost,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    owner: OwnershipRegisters,
    pc: RegMut<'vm, PC>,
    verifier: &'vm mut V,
}

impl<S, V> CodeRootCtx<'_, S, V> {
    pub(crate) fn code_root(self, a: Word, b: Word) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
        V: Verifier,
    {
        self.memory.write_noownerchecks(a, Bytes32::LEN)?;

        let contract_id = ContractId::new(self.memory.read_bytes(b)?);

        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            &contract_id,
        )?;

        let len = contract_size(self.storage, &contract_id)?;
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            self.gas_cost,
            len as u64,
        )?;
        let root = self
            .storage
            .storage_contract(&contract_id)
            .transpose()
            .ok_or(PanicReason::ContractNotFound)?
            .map_err(RuntimeError::Storage)?
            .root();

        self.memory.write_bytes(self.owner, a, *root)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct CodeSizeCtx<'vm, S, V> {
    storage: &'vm S,
    memory: &'vm mut MemoryInstance,
    gas_cost: DependentCost,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    pc: RegMut<'vm, PC>,
    verifier: &'vm mut V,
}

impl<S, V> CodeSizeCtx<'_, S, V> {
    pub(crate) fn code_size(
        self,
        result: &mut Word,
        b: Word,
    ) -> Result<(), RuntimeError<S::Error>>
    where
        S: StorageSize<ContractsRawCode>,
        V: Verifier,
    {
        let contract_id = ContractId::new(self.memory.read_bytes(b)?);

        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            &contract_id,
        )?;

        let len = contract_size(self.storage, &contract_id)?;
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            self.gas_cost,
            len as u64,
        )?;
        *result = len as u64;

        Ok(inc_pc(self.pc)?)
    }
}

pub(crate) struct StateReadWordCtx<'vm, S> {
    pub storage: &'vm mut S,
    pub memory: &'vm MemoryInstance,
    pub context: &'vm Context,
    pub fp: Reg<'vm, FP>,
    pub pc: RegMut<'vm, PC>,
}

pub(crate) fn state_read_word<S: InterpreterStorage>(
    StateReadWordCtx {
        storage,
        memory,
        context,
        fp,
        pc,
        ..
    }: StateReadWordCtx<S>,
    result: &mut Word,
    got_result: &mut Word,
    c: Word,
) -> IoResult<(), S::DataError> {
    let key = Bytes32::new(memory.read_bytes(c)?);
    let contract = internal_contract(context, fp, memory)?;

    let value = storage
        .contract_state(&contract, &key)
        .map_err(RuntimeError::Storage)?
        .map(|bytes| {
            Word::from_be_bytes(
                bytes.as_ref().as_ref()[..8]
                    .try_into()
                    .expect("8 bytes can be converted to a Word"),
            )
        });

    *result = value.unwrap_or(0);
    *got_result = value.is_some() as Word;

    Ok(inc_pc(pc)?)
}

pub(crate) struct StateWriteWordCtx<'vm, S> {
    pub storage: &'vm mut S,
    pub memory: &'vm MemoryInstance,
    pub context: &'vm Context,
    pub new_storage_gas_per_byte: Word,
    pub cgas: RegMut<'vm, CGAS>,
    pub ggas: RegMut<'vm, GGAS>,
    pub fp: Reg<'vm, FP>,
    pub pc: RegMut<'vm, PC>,
}

pub(crate) fn state_write_word<S: InterpreterStorage>(
    StateWriteWordCtx {
        storage,
        memory,
        context,
        new_storage_gas_per_byte,
        cgas,
        ggas,
        fp,
        pc,
    }: StateWriteWordCtx<S>,
    a: Word,
    created_new: &mut Word,
    c: Word,
) -> IoResult<(), S::DataError> {
    let key = Bytes32::new(memory.read_bytes(a)?);
    let contract = internal_contract(context, fp, memory)?;

    let mut value = Bytes32::zeroed();
    value.as_mut()[..WORD_SIZE].copy_from_slice(&c.to_be_bytes());

    let prev = storage
        .contract_state_replace(&contract, &key, value.as_ref())
        .map_err(RuntimeError::Storage)?;

    *created_new = prev.is_none() as Word;

    if prev.is_none() {
        // New data was written, charge gas for it
        gas_charge(
            cgas,
            ggas,
            (Bytes32::LEN as u64)
                .saturating_mul(2)
                .saturating_mul(new_storage_gas_per_byte),
        )?;
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn timestamp<S: InterpreterStorage>(
    storage: &S,
    block_height: BlockHeight,
    pc: RegMut<PC>,
    result: &mut Word,
    b: Word,
) -> IoResult<(), S::DataError> {
    let b = u32::try_from(b)
        .map_err(|_| PanicReason::InvalidBlockHeight)?
        .into();
    (b <= block_height)
        .then_some(())
        .ok_or(PanicReason::TransactionValidity)?;

    *result = storage.timestamp(b).map_err(RuntimeError::Storage)?;

    Ok(inc_pc(pc)?)
}
struct MessageOutputCtx<'vm, S>
where
    S: ContractsAssetsStorage + ?Sized,
{
    base_asset_id: AssetId,
    max_message_data_length: u64,
    memory: &'vm mut MemoryInstance,
    receipts: &'vm mut ReceiptsCtx,
    balances: &'vm mut RuntimeBalances,
    storage: &'vm mut S,
    current_contract: Option<ContractId>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    /// A
    recipient_mem_address: Word,
    /// B
    msg_data_ptr: Word,
    /// C
    msg_data_len: Word,
    /// D
    amount_coins_to_send: Word,
}

impl<S> MessageOutputCtx<'_, S>
where
    S: ContractsAssetsStorage + ?Sized,
{
    pub(crate) fn message_output(self) -> Result<(), RuntimeError<S::Error>> {
        if self.msg_data_len > self.max_message_data_length {
            return Err(RuntimeError::Recoverable(PanicReason::MessageDataTooLong));
        }

        let msg_data = self
            .memory
            .read(self.msg_data_ptr, self.msg_data_len)?
            .to_vec();
        let recipient = Address::new(self.memory.read_bytes(self.recipient_mem_address)?);
        let sender = Address::new(self.memory.read_bytes(*self.fp)?);

        // validations passed, perform the mutations

        if let Some(source_contract) = self.current_contract {
            balance_decrease(
                self.storage,
                &source_contract,
                &self.base_asset_id,
                self.amount_coins_to_send,
            )?;
        } else {
            base_asset_balance_sub(
                &self.base_asset_id,
                self.balances,
                self.memory,
                self.amount_coins_to_send,
            )?;
        }

        let txid = tx_id(self.memory);
        let receipt = Receipt::message_out(
            &txid,
            self.receipts.len() as Word,
            sender,
            recipient,
            self.amount_coins_to_send,
            msg_data,
        );

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct StateReadQWordParams {
    destination_pointer: Word,
    origin_key_pointer: Word,
    num_slots: Word,
}

#[allow(clippy::too_many_arguments)]
fn state_read_qword<S: InterpreterStorage>(
    storage: &S,
    context: &Context,
    memory: &mut MemoryInstance,
    pc: RegMut<PC>,
    fp: Reg<FP>,
    ownership_registers: OwnershipRegisters,
    result_register: &mut Word,
    params: StateReadQWordParams,
) -> IoResult<(), S::DataError> {
    let StateReadQWordParams {
        destination_pointer,
        origin_key_pointer,
        num_slots,
    } = params;

    let contract_id = internal_contract(context, fp, memory)?;
    let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;
    let slots_len = Bytes32::LEN.saturating_mul(num_slots);
    let origin_key = Bytes32::new(memory.read_bytes(origin_key_pointer)?);
    let dst = memory.write(ownership_registers, destination_pointer, slots_len)?;

    let mut all_set = true;
    let result: Vec<u8> = storage
        .contract_state_range(&contract_id, &origin_key, num_slots)
        .map_err(RuntimeError::Storage)?
        .into_iter()
        .flat_map(|bytes| match bytes {
            Some(bytes) => bytes.into_owned(),
            None => {
                all_set = false;
                ContractsStateData::from(Bytes32::zeroed().as_ref())
            }
        })
        .collect();

    *result_register = all_set as Word;

    dst.copy_from_slice(&result);

    inc_pc(pc)?;

    Ok(())
}

struct StateWriteQWord {
    /// The starting storage key location is stored in this range of memory.
    starting_storage_key_pointer: Word,
    /// The source data memory address is stored in this range of memory.
    source_pointer: Word,
    /// How many slots to write.
    num_slots: Word,
}

#[allow(clippy::too_many_arguments)]
fn state_write_qword<'vm, S: InterpreterStorage>(
    contract_id: &ContractId,
    storage: &mut S,
    memory: &MemoryInstance,
    new_storage_gas_per_byte: Word,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateWriteQWord,
) -> IoResult<(), S::DataError> {
    let destination_key =
        Bytes32::new(memory.read_bytes(input.starting_storage_key_pointer)?);

    let values = memory
        .read(
            input.source_pointer,
            (Bytes32::LEN as Word).saturating_mul(input.num_slots),
        )?
        .chunks_exact(Bytes32::LEN);

    let unset_count = storage
        .contract_state_insert_range(contract_id, &destination_key, values)
        .map_err(RuntimeError::Storage)?;
    *result_register = unset_count as Word;

    if unset_count > 0 {
        // New data was written, charge gas for it
        gas_charge(
            cgas,
            ggas,
            (unset_count as u64)
                .saturating_mul(2)
                .saturating_mul(Bytes32::LEN as u64)
                .saturating_mul(new_storage_gas_per_byte),
        )?;
    }

    inc_pc(pc)?;

    Ok(())
}

struct StateClearQWord {
    /// The starting storage key location is stored in this address.
    start_storage_key_pointer: Word,
    /// Number of slots to read.
    num_slots: usize,
}

impl StateClearQWord {
    fn new(start_storage_key_pointer: Word, num_slots: Word) -> SimpleResult<Self> {
        let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;
        Ok(Self {
            start_storage_key_pointer,
            num_slots,
        })
    }
}

fn state_clear_qword<S: InterpreterStorage>(
    contract_id: &ContractId,
    storage: &mut S,
    memory: &MemoryInstance,
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateClearQWord,
) -> IoResult<(), S::DataError> {
    let start_key = Bytes32::new(memory.read_bytes(input.start_storage_key_pointer)?);

    let all_previously_set = storage
        .contract_state_remove_range(contract_id, &start_key, input.num_slots)
        .map_err(RuntimeError::Storage)?
        .is_some();

    *result_register = all_previously_set as Word;

    inc_pc(pc)?;

    Ok(())
}
