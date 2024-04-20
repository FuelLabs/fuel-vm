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
            contract_size,
        },
        gas::{
            dependent_gas_charge_without_base,
            gas_charge,
            ProfileGas,
        },
        internal::{
            base_asset_balance_sub,
            current_contract,
            inc_pc,
            internal_contract,
            tx_id,
        },
        memory::{
            copy_from_slice_zero_fill_noownerchecks,
            OwnershipRegisters,
        },
        receipts::ReceiptsCtx,
        ExecutableTransaction,
        InputContracts,
        Interpreter,
        Memory,
        RuntimeBalances,
    },
    prelude::Profiler,
    storage::{
        ContractsAssetsStorage,
        ContractsRawCode,
        ContractsStateData,
        InterpreterStorage,
    },
};
use alloc::vec::Vec;
use fuel_asm::PanicReason;
use fuel_storage::StorageSize;
use fuel_tx::{
    consts::BALANCE_ENTRY_SIZE,
    ContractIdExt,
    DependentCost,
    Receipt,
};
use fuel_types::{
    bytes,
    Address,
    AssetId,
    BlockHeight,
    Bytes32,
    ContractId,
    RegisterId,
    Word,
};

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

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
    S: InterpreterStorage,
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
        contract_id_addr: Word,
        contract_offset: Word,
        length_unpadded: Word,
    ) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().ldc();
        // Charge only for the `base` execution.
        // We will charge for the contracts size in the `load_contract_code`.
        self.gas_charge(gas_cost.base())?;
        let contract_max_size = self.contract_max_size();
        let current_contract =
            current_contract(&self.context, self.registers.fp(), &self.memory)?;
        let (
            SystemRegisters {
                cgas,
                ggas,
                ssp,
                sp,
                fp,
                pc,
                is,
                ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let input = LoadContractCodeCtx {
            memory: &mut self.memory,
            profiler: &mut self.profiler,
            storage: &mut self.storage,
            contract_max_size,
            input_contracts: InputContracts::new(
                self.tx.input_contracts(),
                &mut self.panic_context,
            ),
            gas_cost,
            current_contract,
            cgas,
            ggas,
            ssp,
            sp,
            fp: fp.as_ref(),
            pc,
            is: is.as_ref(),
        };
        input.load_contract_code(contract_id_addr, contract_offset, length_unpadded)
    }

    pub(crate) fn burn(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let (SystemRegisters { fp, pc, is, .. }, _) =
            split_registers(&mut self.registers);
        BurnCtx {
            storage: &mut self.storage,
            context: &self.context,
            memory: &self.memory,
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
            memory: &self.memory,
            receipts: &mut self.receipts,
            profiler: &mut self.profiler,
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

        let current_contract =
            current_contract(&self.context, self.registers.fp(), &self.memory)?;
        let owner = self.ownership_registers();
        let (
            SystemRegisters {
                cgas, ggas, pc, is, ..
            },
            _,
        ) = split_registers(&mut self.registers);
        let input = CodeCopyCtx {
            memory: &mut self.memory,
            input_contracts: InputContracts::new(
                self.tx.input_contracts(),
                &mut self.panic_context,
            ),
            storage: &mut self.storage,
            profiler: &mut self.profiler,
            current_contract,
            owner,
            gas_cost,
            cgas,
            ggas,
            pc,
            is: is.as_ref(),
        };
        input.code_copy(a, b, c, d)
    }

    pub(crate) fn block_hash(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        block_hash(
            &self.storage,
            &mut self.memory,
            owner,
            self.registers.pc_mut(),
            a,
            b,
        )
    }

    pub(crate) fn block_height(&mut self, ra: RegisterId) -> IoResult<(), S::DataError> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        Ok(block_height(&self.context, pc, result)?)
    }

    pub(crate) fn block_proposer(&mut self, a: Word) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        coinbase(
            &self.storage,
            &mut self.memory,
            owner,
            self.registers.pc_mut(),
            a,
        )
    }

    pub(crate) fn code_root(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().croo();
        self.gas_charge(gas_cost.base())?;
        let current_contract =
            current_contract(&self.context, self.registers.fp(), &self.memory)?;
        let owner = self.ownership_registers();
        let (
            SystemRegisters {
                cgas, ggas, pc, is, ..
            },
            _,
        ) = split_registers(&mut self.registers);
        CodeRootCtx {
            memory: &mut self.memory,
            storage: &mut self.storage,
            gas_cost,
            profiler: &mut self.profiler,
            input_contracts: InputContracts::new(
                self.tx.input_contracts(),
                &mut self.panic_context,
            ),
            current_contract,
            cgas,
            ggas,
            owner,
            pc,
            is: is.as_ref(),
        }
        .code_root(a, b)
    }

    pub(crate) fn code_size(
        &mut self,
        ra: RegisterId,
        b: Word,
    ) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().csiz();
        // Charge only for the `base` execution.
        // We will charge for the contracts size in the `code_size`.
        self.gas_charge(gas_cost.base())?;
        let current_contract =
            current_contract(&self.context, self.registers.fp(), &self.memory)?;
        let (
            SystemRegisters {
                cgas, ggas, pc, is, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(ra)?];
        let input = CodeSizeCtx {
            memory: &mut self.memory,
            storage: &mut self.storage,
            gas_cost,
            profiler: &mut self.profiler,
            input_contracts: InputContracts::new(
                self.tx.input_contracts(),
                &mut self.panic_context,
            ),
            current_contract,
            cgas,
            ggas,
            pc,
            is: is.as_ref(),
        };
        input.code_size(result, b)
    }

    pub(crate) fn state_clear_qword(
        &mut self,
        a: Word,
        rb: RegisterId,
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

        state_clear_qword(&contract_id?, storage, memory, pc, result, input)
    }

    pub(crate) fn state_read_word(
        &mut self,
        ra: RegisterId,
        rb: RegisterId,
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
                memory,
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
        rb: RegisterId,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let owner = self.ownership_registers();
        let contract_id = self.internal_contract();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let Self {
            ref storage,
            ref mut memory,
            ..
        } = self;

        state_read_qword(
            &contract_id?,
            storage,
            memory,
            pc,
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
        rb: RegisterId,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        let (
            SystemRegisters {
                cgas,
                ggas,
                is,
                fp,
                pc,
                ..
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
                memory,
                context,
                profiler: &mut self.profiler,
                new_storage_gas_per_byte,
                current_contract: self.frames.last().map(|frame| frame.to()).copied(),
                cgas,
                ggas,
                is: is.as_ref(),
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
        rb: RegisterId,
        c: Word,
        d: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_per_byte = self.gas_costs().new_storage_per_byte();
        let contract_id = self.internal_contract();
        let (
            SystemRegisters {
                is, cgas, ggas, pc, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
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
            memory,
            &mut self.profiler,
            new_storage_per_byte,
            self.frames.last().map(|frame| frame.to()).copied(),
            cgas,
            ggas,
            is.as_ref(),
            pc,
            result,
            input,
        )
    }

    pub(crate) fn timestamp(
        &mut self,
        ra: RegisterId,
        b: Word,
    ) -> IoResult<(), S::DataError> {
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
            memory: &mut self.memory,
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

struct LoadContractCodeCtx<'vm, S, I> {
    contract_max_size: u64,
    memory: &'vm mut Memory,
    profiler: &'vm mut Profiler,
    input_contracts: InputContracts<'vm, I>,
    storage: &'vm S,
    current_contract: Option<ContractId>,
    gas_cost: DependentCost,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    ssp: RegMut<'vm, SSP>,
    sp: RegMut<'vm, SP>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S, I> LoadContractCodeCtx<'vm, S, I>
where
    S: InterpreterStorage,
{
    /// Loads contract ID pointed by `a`, and then for that contract,
    /// copies `c` bytes from it starting from offset `b` into the stack.
    /// ```txt
    /// contract_id = mem[$rA, 32]
    /// contract_code = contracts[contract_id]
    /// mem[$ssp, $rC] = contract_code[$rB, $rC]
    /// ```
    /// Returns the total length of the contract code that was loaded from storage.
    pub(crate) fn load_contract_code(
        mut self,
        contract_id_addr: Word,
        contract_offset: Word,
        length_unpadded: Word,
    ) -> IoResult<(), S::DataError>
    where
        I: Iterator<Item = &'vm ContractId>,
        S: InterpreterStorage,
    {
        let ssp = *self.ssp;
        let sp = *self.sp;
        let fp = *self.fp;
        let region_start = ssp;

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into())
        }

        let contract_id = ContractId::from(self.memory.read_bytes(contract_id_addr)?);
        let contract_offset: usize = contract_offset
            .try_into()
            .map_err(|_| PanicReason::MemoryOverflow)?;

        let length = bytes::padded_len_usize(
            length_unpadded
                .try_into()
                .map_err(|_| PanicReason::MemoryOverflow)?,
        )
        .map(|len| len as Word)
        .unwrap_or(Word::MAX);

        if length > self.contract_max_size {
            return Err(PanicReason::ContractMaxSize.into())
        }

        let new_sp = ssp.saturating_add(length);
        self.memory.grow_stack(new_sp)?;

        self.input_contracts.check(&contract_id)?;

        // Fetch the storage contract
        let contract = super::contract::contract(self.storage, &contract_id)?;
        let contract_bytes = contract.as_ref().as_ref();
        let contract_len = contract_bytes.len();
        let profiler = ProfileGas {
            pc: self.pc.as_ref(),
            is: self.is,
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            profiler,
            self.gas_cost,
            contract_len as u64,
        )?;

        // Mark stack space as allocated
        *self.sp = new_sp;
        *self.ssp = new_sp;

        // Copy the code. Ownership checks are not used as the stack is adjusted above.
        copy_from_slice_zero_fill_noownerchecks(
            self.memory,
            contract_bytes,
            region_start,
            contract_offset,
            length,
        )?;

        // Update frame pointer, if we have a stack frame (e.g. fp > 0)
        if fp > 0 {
            let size = CallFrame::code_size_offset().saturating_add(WORD_SIZE);

            let old_code_size = Word::from_be_bytes(self.memory.read_bytes(fp)?);

            let new_code_size = old_code_size
                .checked_add(length as Word)
                .ok_or(PanicReason::MemoryOverflow)?;

            self.memory
                .write_noownerchecks(fp, size)?
                .copy_from_slice(&new_code_size.to_be_bytes());
        }

        inc_pc(self.pc)?;

        Ok(())
    }
}

struct BurnCtx<'vm, S> {
    storage: &'vm mut S,
    context: &'vm Context,
    memory: &'vm Memory,
    receipts: &'vm mut ReceiptsCtx,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S> BurnCtx<'vm, S>
where
    S: ContractsAssetsStorage,
{
    pub(crate) fn burn(self, a: Word, b: Word) -> IoResult<(), S::Error> {
        let contract_id = internal_contract(self.context, self.fp, self.memory)?;
        let sub_id = Bytes32::new(self.memory.read_bytes(b)?);
        let asset_id = contract_id.asset_id(&sub_id);

        let balance = balance(self.storage, &contract_id, &asset_id)?;
        let balance = balance
            .checked_sub(a)
            .ok_or(PanicReason::NotEnoughBalance)?;

        let _ = self
            .storage
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
    memory: &'vm Memory,
    profiler: &'vm mut Profiler,
    receipts: &'vm mut ReceiptsCtx,
    new_storage_gas_per_byte: Word,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S> MintCtx<'vm, S>
where
    S: ContractsAssetsStorage,
{
    pub(crate) fn mint(self, a: Word, b: Word) -> Result<(), RuntimeError<S::Error>> {
        let contract_id = internal_contract(self.context, self.fp, self.memory)?;
        let sub_id = Bytes32::new(self.memory.read_bytes(b)?);
        let asset_id = contract_id.asset_id(&sub_id);

        let balance = balance(self.storage, &contract_id, &asset_id)?;
        let balance = balance.checked_add(a).ok_or(PanicReason::BalanceOverflow)?;

        let old_value = self
            .storage
            .contract_asset_id_balance_insert(&contract_id, &asset_id, balance)
            .map_err(RuntimeError::Storage)?;

        if old_value.is_none() {
            // New data was written, charge gas for it
            let profiler = ProfileGas {
                pc: self.pc.as_ref(),
                is: self.is,
                current_contract: Some(contract_id),
                profiler: self.profiler,
            };
            gas_charge(
                self.cgas,
                self.ggas,
                profiler,
                (BALANCE_ENTRY_SIZE as u64).saturating_mul(self.new_storage_gas_per_byte),
            )?;
        }

        let receipt = Receipt::mint(sub_id, contract_id, a, *self.pc, *self.is);

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct CodeCopyCtx<'vm, S, I> {
    memory: &'vm mut Memory,
    input_contracts: InputContracts<'vm, I>,
    storage: &'vm S,
    profiler: &'vm mut Profiler,
    current_contract: Option<ContractId>,
    owner: OwnershipRegisters,
    gas_cost: DependentCost,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S, I> CodeCopyCtx<'vm, S, I>
where
    S: InterpreterStorage,
{
    pub(crate) fn code_copy(
        mut self,
        dst_addr: Word,
        contract_id_addr: Word,
        contract_offset: Word,
        length: Word,
    ) -> IoResult<(), S::DataError>
    where
        I: Iterator<Item = &'vm ContractId>,
        S: InterpreterStorage,
    {
        let contract_id = ContractId::from(self.memory.read_bytes(contract_id_addr)?);
        let offset: usize = contract_offset
            .try_into()
            .map_err(|_| PanicReason::MemoryOverflow)?;

        // Check target memory range ownership
        if !self.owner.has_ownership_range(&(dst_addr..length)) {
            return Err(PanicReason::MemoryOverflow.into())
        }

        self.input_contracts.check(&contract_id)?;

        let contract = super::contract::contract(self.storage, &contract_id)?;
        let contract_bytes = contract.as_ref().as_ref();
        let contract_len = contract_bytes.len();
        let profiler = ProfileGas {
            pc: self.pc.as_ref(),
            is: self.is,
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            profiler,
            self.gas_cost,
            contract_len as u64,
        )?;

        // Owner checks already performed above
        copy_from_slice_zero_fill_noownerchecks(
            self.memory,
            contract.as_ref().as_ref(),
            dst_addr,
            offset,
            length,
        )?;

        Ok(inc_pc(self.pc)?)
    }
}

pub(crate) fn block_hash<S: InterpreterStorage>(
    storage: &S,
    memory: &mut Memory,
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
    memory: &mut Memory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
) -> IoResult<(), S::DataError> {
    let coinbase = storage.coinbase().map_err(RuntimeError::Storage)?;
    memory.write_bytes(owner, a, *coinbase)?;
    inc_pc(pc)?;
    Ok(())
}

struct CodeRootCtx<'vm, S, I> {
    storage: &'vm S,
    memory: &'vm mut Memory,
    gas_cost: DependentCost,
    profiler: &'vm mut Profiler,
    input_contracts: InputContracts<'vm, I>,
    current_contract: Option<ContractId>,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    owner: OwnershipRegisters,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S, I: Iterator<Item = &'vm ContractId>> CodeRootCtx<'vm, S, I> {
    pub(crate) fn code_root(mut self, a: Word, b: Word) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
    {
        self.memory.write_noownerchecks(a, Bytes32::LEN)?;

        let contract_id = ContractId::new(self.memory.read_bytes(b)?);

        self.input_contracts.check(&contract_id)?;

        let len = contract_size(self.storage, &contract_id)? as Word;
        let profiler = ProfileGas {
            pc: self.pc.as_ref(),
            is: self.is,
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            profiler,
            self.gas_cost,
            len,
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

struct CodeSizeCtx<'vm, S, I> {
    storage: &'vm S,
    memory: &'vm mut Memory,
    gas_cost: DependentCost,
    profiler: &'vm mut Profiler,
    input_contracts: InputContracts<'vm, I>,
    current_contract: Option<ContractId>,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S, I: Iterator<Item = &'vm ContractId>> CodeSizeCtx<'vm, S, I> {
    pub(crate) fn code_size(
        mut self,
        result: &mut Word,
        b: Word,
    ) -> Result<(), RuntimeError<S::Error>>
    where
        S: StorageSize<ContractsRawCode>,
    {
        let contract_id = ContractId::new(self.memory.read_bytes(b)?);

        self.input_contracts.check(&contract_id)?;

        let len = contract_size(self.storage, &contract_id)? as Word;
        let profiler = ProfileGas {
            pc: self.pc.as_ref(),
            is: self.is,
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge_without_base(
            self.cgas,
            self.ggas,
            profiler,
            self.gas_cost,
            len,
        )?;
        *result = len;

        Ok(inc_pc(self.pc)?)
    }
}

pub(crate) struct StateReadWordCtx<'vm, S> {
    pub storage: &'vm mut S,
    pub memory: &'vm Memory,
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
    pub memory: &'vm Memory,
    pub context: &'vm Context,
    pub profiler: &'vm mut Profiler,
    pub new_storage_gas_per_byte: Word,
    pub current_contract: Option<ContractId>,
    pub cgas: RegMut<'vm, CGAS>,
    pub ggas: RegMut<'vm, GGAS>,
    pub is: Reg<'vm, IS>,
    pub fp: Reg<'vm, FP>,
    pub pc: RegMut<'vm, PC>,
}

pub(crate) fn state_write_word<S: InterpreterStorage>(
    StateWriteWordCtx {
        storage,
        memory,
        context,
        profiler,
        new_storage_gas_per_byte,
        current_contract,
        cgas,
        ggas,
        is,
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

    let (_size, prev) = storage
        .contract_state_insert(&contract, &key, value.as_ref())
        .map_err(RuntimeError::Storage)?;

    *created_new = prev.is_none() as Word;

    if prev.is_none() {
        // New data was written, charge gas for it
        let profiler = ProfileGas {
            pc: pc.as_ref(),
            is,
            current_contract,
            profiler,
        };
        gas_charge(
            cgas,
            ggas,
            profiler,
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
    memory: &'vm mut Memory,
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

fn state_read_qword<S: InterpreterStorage>(
    contract_id: &ContractId,
    storage: &S,
    memory: &mut Memory,
    pc: RegMut<PC>,
    ownership_registers: OwnershipRegisters,
    result_register: &mut Word,
    params: StateReadQWordParams,
) -> IoResult<(), S::DataError> {
    let StateReadQWordParams {
        destination_pointer,
        origin_key_pointer,
        num_slots,
    } = params;

    let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;
    let slots_len = Bytes32::LEN.saturating_mul(num_slots);
    let target_range = memory.verify(destination_pointer, slots_len)?;
    ownership_registers.verify_ownership(&target_range.words())?;
    ownership_registers.verify_internal_context()?;

    let origin_key = Bytes32::new(memory.read_bytes(origin_key_pointer)?);

    let mut all_set = true;
    let result: Vec<u8> = storage
        .contract_state_range(contract_id, &origin_key, num_slots)
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

    memory
        .write_noownerchecks(destination_pointer, result.len())?
        .copy_from_slice(&result);

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
    memory: &Memory,
    profiler: &'vm mut Profiler,
    new_storage_gas_per_byte: Word,
    current_contract: Option<ContractId>,
    cgas: RegMut<'vm, CGAS>,
    ggas: RegMut<'vm, GGAS>,
    is: Reg<'vm, IS>,
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
        let profiler = ProfileGas {
            pc: pc.as_ref(),
            is,
            current_contract,
            profiler,
        };
        gas_charge(
            cgas,
            ggas,
            profiler,
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
    memory: &Memory,
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
