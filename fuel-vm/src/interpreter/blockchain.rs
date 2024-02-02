use crate::{
    call::CallFrame,
    constraints::{
        reg_key::*,
        CheckedMemConstLen,
        CheckedMemValue,
    },
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
            append_receipt,
            base_asset_balance_sub,
            current_contract,
            inc_pc,
            internal_contract,
            internal_contract_bounds,
            tx_id,
            AppendReceipt,
        },
        memory::{
            copy_from_slice_zero_fill_noownerchecks,
            read_bytes,
            try_mem_write,
            OwnershipRegisters,
        },
        receipts::ReceiptsCtx,
        ExecutableTransaction,
        InputContracts,
        Interpreter,
        MemoryRange,
        RuntimeBalances,
    },
    prelude::Profiler,
    storage::{
        ContractsAssetsStorage,
        ContractsRawCode,
        InterpreterStorage,
    },
};
use alloc::vec::Vec;
use fuel_asm::PanicReason;
use fuel_storage::StorageSize;
use fuel_tx::{
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
        let gas_cost = self.gas_costs().ldc;
        // Charge only for the `base` execution.
        // We will charge for the contracts size in the `load_contract_code`.
        self.gas_charge(gas_cost.base())?;
        let contract_max_size = self.contract_max_size();
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?
                .copied();
        let (
            SystemRegisters {
                cgas,
                ggas,
                ssp,
                sp,
                hp,
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
            hp: hp.as_ref(),
            pc,
            is: is.as_ref(),
        };
        input.load_contract_code(contract_id_addr, contract_offset, length_unpadded)
    }

    pub(crate) fn burn(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let tx_offset = self.tx_offset();
        let (SystemRegisters { fp, pc, is, .. }, _) =
            split_registers(&mut self.registers);
        BurnCtx {
            storage: &mut self.storage,
            context: &self.context,
            append: AppendReceipt {
                receipts: &mut self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset,
                memory: &mut self.memory,
            },
            fp: fp.as_ref(),
            pc,
            is: is.as_ref(),
        }
        .burn(a, b)
    }

    pub(crate) fn mint(&mut self, a: Word, b: Word) -> IoResult<(), S::DataError> {
        let tx_offset = self.tx_offset();
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte;
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
            append: AppendReceipt {
                receipts: &mut self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset,
                memory: &mut self.memory,
            },
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
        let gas_cost = self.gas_costs().ccp;
        // Charge only for the `base` execution.
        // We will charge for the contract's size in the `code_copy`.
        self.gas_charge(gas_cost.base())?;

        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?
                .copied();
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
        let owner = self.ownership_registers();
        CodeRootCtx {
            memory: &mut self.memory,
            input_contracts: InputContracts::new(
                self.tx.input_contracts(),
                &mut self.panic_context,
            ),
            storage: &self.storage,
            owner,
            pc: self.registers.pc_mut(),
        }
        .code_root(a, b)
    }

    pub(crate) fn code_size(
        &mut self,
        ra: RegisterId,
        b: Word,
    ) -> IoResult<(), S::DataError> {
        let gas_cost = self.gas_costs().csiz;
        // Charge only for the `base` execution.
        // We will charge for the contracts size in the `code_size`.
        self.gas_charge(gas_cost.base())?;
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?
                .copied();
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
        let contract_id = self.internal_contract().copied();
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
        let contract_id = self.internal_contract().copied();
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateReadQWord::new(a, c, d, owner)?;
        let Self {
            ref storage,
            ref mut memory,
            ..
        } = self;

        state_read_qword(&contract_id?, storage, memory, pc, result, input)
    }

    pub(crate) fn state_write_word(
        &mut self,
        a: Word,
        rb: RegisterId,
        c: Word,
    ) -> IoResult<(), S::DataError> {
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte;
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
        let new_storage_per_byte = self.gas_costs().new_storage_per_byte;
        let contract_id = self.internal_contract().copied();
        let (
            SystemRegisters {
                is, cgas, ggas, pc, ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let result = &mut w[WriteRegKey::try_from(rb)?];

        let input = StateWriteQWord::new(a, c, d)?;
        let Self {
            ref mut storage,
            ref mut memory,
            ..
        } = self;

        state_write_qword(
            &contract_id?,
            storage,
            memory.as_mut(),
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
        let tx_offset = self.tx_offset();
        let (SystemRegisters { fp, pc, .. }, _) = split_registers(&mut self.registers);
        let input = MessageOutputCtx {
            base_asset_id,
            max_message_data_length,
            memory: &mut self.memory,
            tx_offset,
            receipts: &mut self.receipts,
            tx: &mut self.tx,
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
    memory: &'vm mut [u8; MEM_SIZE],
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
    hp: Reg<'vm, HP>,
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

        if ssp != sp {
            return Err(PanicReason::ExpectedUnallocatedStack.into())
        }

        let contract_id = ContractId::from(read_bytes(self.memory, contract_id_addr)?);
        let contract_offset: usize = contract_offset
            .try_into()
            .map_err(|_| PanicReason::MemoryOverflow)?;

        let length = bytes::padded_len_word(length_unpadded);
        let dst_range = MemoryRange::new(ssp, length)?;

        if dst_range.end as Word >= *self.hp {
            // Would make stack and heap overlap
            return Err(PanicReason::MemoryOverflow.into())
        }

        if length > self.contract_max_size {
            return Err(PanicReason::MemoryOverflow.into())
        }

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
        let new_stack = dst_range.words().end;
        *self.sp = new_stack;
        *self.ssp = new_stack;

        // Copy the code. Ownership checks are not used as the stack is adjusted above.
        copy_from_slice_zero_fill_noownerchecks(
            self.memory,
            contract_bytes,
            dst_range.start,
            contract_offset,
            length,
        )?;

        // Update frame pointer, if we have a stack frame (e.g. fp > 0)
        if fp > 0 {
            let fp_code_size = MemoryRange::new_overflowing_op(
                u64::overflowing_add,
                fp,
                CallFrame::code_size_offset() as Word,
                WORD_SIZE,
            )?;

            let old_code_size = Word::from_be_bytes(
                self.memory[fp_code_size.usizes()]
                    .try_into()
                    .expect("`fp_code_size_end` is `WORD_SIZE`"),
            );

            let new_code_size = old_code_size
                .checked_add(length as Word)
                .ok_or(PanicReason::MemoryOverflow)?;

            self.memory[fp_code_size.usizes()]
                .copy_from_slice(&new_code_size.to_be_bytes());
        }

        inc_pc(self.pc)?;

        Ok(())
    }
}

struct BurnCtx<'vm, S> {
    storage: &'vm mut S,
    context: &'vm Context,
    append: AppendReceipt<'vm>,
    fp: Reg<'vm, FP>,
    pc: RegMut<'vm, PC>,
    is: Reg<'vm, IS>,
}

impl<'vm, S> BurnCtx<'vm, S>
where
    S: ContractsAssetsStorage,
{
    pub(crate) fn burn(self, a: Word, b: Word) -> IoResult<(), S::Error> {
        let range = internal_contract_bounds(self.context, self.fp)?;
        let sub_id_range = CheckedMemConstLen::<{ Bytes32::LEN }>::new(b)?;
        let memory = &*self.append.memory;

        let sub_id = Bytes32::from_bytes_ref(sub_id_range.read(memory));

        let contract_id = ContractId::from_bytes_ref(range.read(memory));
        let asset_id = contract_id.asset_id(sub_id);

        let balance = balance(self.storage, contract_id, &asset_id)?;
        let balance = balance
            .checked_sub(a)
            .ok_or(PanicReason::NotEnoughBalance)?;

        let _ = self
            .storage
            .merkle_contract_asset_id_balance_insert(contract_id, &asset_id, balance)
            .map_err(RuntimeError::Storage)?;

        let receipt = Receipt::burn(*sub_id, *contract_id, a, *self.pc, *self.is);

        append_receipt(self.append, receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct MintCtx<'vm, S> {
    storage: &'vm mut S,
    context: &'vm Context,
    profiler: &'vm mut Profiler,
    new_storage_gas_per_byte: Word,
    append: AppendReceipt<'vm>,
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
        let range = internal_contract_bounds(self.context, self.fp)?;
        let sub_id_range = CheckedMemConstLen::<{ Bytes32::LEN }>::new(b)?;
        let memory = &*self.append.memory;

        let sub_id = Bytes32::from_bytes_ref(sub_id_range.read(memory));

        let contract_id = ContractId::from_bytes_ref(range.read(memory));
        let asset_id = contract_id.asset_id(sub_id);

        let balance = balance(self.storage, contract_id, &asset_id)?;
        let balance = balance.checked_add(a).ok_or(PanicReason::BalanceOverflow)?;

        let old_value = self
            .storage
            .merkle_contract_asset_id_balance_insert(contract_id, &asset_id, balance)
            .map_err(RuntimeError::Storage)?;

        if old_value.is_none() {
            // New data was written, charge gas for it
            let profiler = ProfileGas {
                pc: self.pc.as_ref(),
                is: self.is,
                current_contract: Some(*contract_id),
                profiler: self.profiler,
            };
            gas_charge(
                self.cgas,
                self.ggas,
                profiler,
                ((AssetId::LEN + WORD_SIZE) as u64) * self.new_storage_gas_per_byte,
            )?;
        }

        let receipt = Receipt::mint(*sub_id, *contract_id, a, *self.pc, *self.is);

        append_receipt(self.append, receipt)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct CodeCopyCtx<'vm, S, I> {
    memory: &'vm mut [u8; MEM_SIZE],
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
        let contract_id = ContractId::from(read_bytes(self.memory, contract_id_addr)?);
        let offset: usize = contract_offset
            .try_into()
            .map_err(|_| PanicReason::MemoryOverflow)?;

        // Check target memory range ownership
        if !self
            .owner
            .has_ownership_range(&MemoryRange::new(dst_addr, length)?)
        {
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
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
) -> IoResult<(), S::DataError> {
    let height = u32::try_from(b)
        .map_err(|_| PanicReason::InvalidBlockHeight)?
        .into();
    let hash = storage.block_hash(height).map_err(RuntimeError::Storage)?;

    try_mem_write(a, hash.as_ref(), owner, memory)?;

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
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
) -> IoResult<(), S::DataError> {
    let coinbase = storage.coinbase().map_err(RuntimeError::Storage)?;
    try_mem_write(a, coinbase.as_ref(), owner, memory)?;
    inc_pc(pc)?;
    Ok(())
}

struct CodeRootCtx<'vm, S, I> {
    memory: &'vm mut [u8; MEM_SIZE],
    input_contracts: InputContracts<'vm, I>,
    storage: &'vm S,
    owner: OwnershipRegisters,
    pc: RegMut<'vm, PC>,
}

impl<'vm, S, I: Iterator<Item = &'vm ContractId>> CodeRootCtx<'vm, S, I> {
    pub(crate) fn code_root(mut self, a: Word, b: Word) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
    {
        MemoryRange::new(a, Bytes32::LEN)?;
        let contract_id = CheckedMemConstLen::<{ ContractId::LEN }>::new(b)?;

        let contract_id = ContractId::from_bytes_ref(contract_id.read(self.memory));

        self.input_contracts.check(contract_id)?;

        let (_, root) = self
            .storage
            .storage_contract_root(contract_id)
            .transpose()
            .ok_or(PanicReason::ContractNotFound)?
            .map_err(RuntimeError::Storage)?
            .into_owned();

        try_mem_write(a, root.as_ref(), self.owner, self.memory)?;

        Ok(inc_pc(self.pc)?)
    }
}

struct CodeSizeCtx<'vm, S, I> {
    storage: &'vm S,
    memory: &'vm mut [u8; MEM_SIZE],
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
        let contract_id = CheckedMemConstLen::<{ ContractId::LEN }>::new(b)?;

        let contract_id = ContractId::from_bytes_ref(contract_id.read(self.memory));

        self.input_contracts.check(contract_id)?;

        let len = contract_size(self.storage, contract_id)? as Word;
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
    pub memory: &'vm [u8; MEM_SIZE],
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
    let key = CheckedMemConstLen::<{ Bytes32::LEN }>::new(c)?;

    let contract = internal_contract(context, fp, memory)?;

    let key = Bytes32::from_bytes_ref(key.read(memory));

    let value = storage
        .merkle_contract_state(contract, key)
        .map_err(RuntimeError::Storage)?
        .map(|bytes| {
            Word::from_be_bytes(
                bytes[..8]
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
    pub memory: &'vm [u8; MEM_SIZE],
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
    let key = CheckedMemConstLen::<{ Bytes32::LEN }>::new(a)?;

    let contract = internal_contract_bounds(context, fp)?;

    // Safety: Memory bounds logically verified by the interpreter
    let contract = ContractId::from_bytes_ref(contract.read(memory));
    let key = Bytes32::from_bytes_ref(key.read(memory));

    let mut value = Vec::<u8>::default();

    value[..WORD_SIZE].copy_from_slice(&c.to_be_bytes());

    let result = storage
        .merkle_contract_state_insert(contract, key, &value)
        .map_err(RuntimeError::Storage)?;

    *created_new = result.is_none() as Word;

    if result.is_none() {
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
            (2 * Bytes32::LEN as u64) * new_storage_gas_per_byte,
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
struct MessageOutputCtx<'vm, Tx, S>
where
    S: ContractsAssetsStorage + ?Sized,
{
    base_asset_id: AssetId,
    max_message_data_length: u64,
    memory: &'vm mut [u8; MEM_SIZE],
    tx_offset: usize,
    receipts: &'vm mut ReceiptsCtx,
    tx: &'vm mut Tx,
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

impl<Tx, S> MessageOutputCtx<'_, Tx, S>
where
    S: ContractsAssetsStorage + ?Sized,
{
    pub(crate) fn message_output(self) -> Result<(), RuntimeError<S::Error>>
    where
        Tx: ExecutableTransaction,
    {
        let recipient_address = CheckedMemValue::<Address>::new::<{ Address::LEN }>(
            self.recipient_mem_address,
        )?;

        if self.msg_data_len > self.max_message_data_length {
            return Err(RuntimeError::Recoverable(PanicReason::MessageDataTooLong))
        }

        let msg_data_range = MemoryRange::new(self.msg_data_ptr, self.msg_data_len)?;

        let recipient = recipient_address.try_from(self.memory)?;

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

        let sender = CheckedMemConstLen::<{ Address::LEN }>::new(*self.fp)?;
        let txid = tx_id(self.memory);
        let msg_data = msg_data_range.read(self.memory).to_vec();
        let sender = Address::from_bytes_ref(sender.read(self.memory));

        let receipt = Receipt::message_out(
            txid,
            self.receipts.len() as Word,
            *sender,
            recipient,
            self.amount_coins_to_send,
            msg_data,
        );

        append_receipt(
            AppendReceipt {
                receipts: self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset: self.tx_offset,
                memory: self.memory,
            },
            receipt,
        )?;

        Ok(inc_pc(self.pc)?)
    }
}

struct StateReadQWord {
    /// The destination memory address is stored in this range of memory.
    destination_address_memory_range: MemoryRange,
    /// The starting storage key location is stored in this range of memory.
    origin_key_memory_range: CheckedMemConstLen<{ Bytes32::LEN }>,
    /// Number of slots to read.
    num_slots: usize,
}

impl StateReadQWord {
    fn new(
        destination_memory_address: Word,
        origin_key_memory_address: Word,
        num_slots: Word,
        ownership_registers: OwnershipRegisters,
    ) -> SimpleResult<Self> {
        let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;
        let destination_address_memory_range = MemoryRange::new(
            destination_memory_address,
            Bytes32::LEN.saturating_mul(num_slots),
        )?;
        ownership_registers.verify_ownership(&destination_address_memory_range)?;
        ownership_registers.verify_internal_context()?;
        let origin_key_memory_range =
            CheckedMemConstLen::<{ Bytes32::LEN }>::new(origin_key_memory_address)?;
        Ok(Self {
            destination_address_memory_range,
            origin_key_memory_range,
            num_slots,
        })
    }
}

fn state_read_qword<S: InterpreterStorage>(
    contract_id: &ContractId,
    storage: &S,
    memory: &mut [u8; MEM_SIZE],
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateReadQWord,
) -> IoResult<(), S::DataError> {
    let origin_key = Bytes32::from_bytes_ref(input.origin_key_memory_range.read(memory));

    let mut all_set = true;
    let result: Vec<u8> = storage
        .merkle_contract_state_range(contract_id, origin_key, input.num_slots)
        .map_err(RuntimeError::Storage)?
        .into_iter()
        .flat_map(|bytes| match bytes {
            Some(bytes) => **bytes,
            None => {
                all_set = false;
                Default::default()
            }
        })
        .collect();

    *result_register = all_set as Word;

    memory[input.destination_address_memory_range.usizes()].copy_from_slice(&result);

    inc_pc(pc)?;

    Ok(())
}

struct StateWriteQWord {
    /// The starting storage key location is stored in this range of memory.
    starting_storage_key_memory_range: CheckedMemConstLen<{ Bytes32::LEN }>,
    /// The source data memory address is stored in this range of memory.
    source_address_memory_range: MemoryRange,
}

impl StateWriteQWord {
    fn new(
        starting_storage_key_memory_address: Word,
        source_memory_address: Word,
        num_slots: Word,
    ) -> SimpleResult<Self> {
        let source_address_memory_range = MemoryRange::new(
            source_memory_address,
            (Bytes32::LEN as Word).saturating_mul(num_slots),
        )?;

        let starting_storage_key_memory_range =
            CheckedMemConstLen::<{ Bytes32::LEN }>::new(
                starting_storage_key_memory_address,
            )?;

        Ok(Self {
            source_address_memory_range,
            starting_storage_key_memory_range,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn state_write_qword<'vm, S: InterpreterStorage>(
    contract_id: &ContractId,
    storage: &mut S,
    memory: &[u8; MEM_SIZE],
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
        Bytes32::from_bytes_ref(input.starting_storage_key_memory_range.read(memory));

    let values: Vec<_> = memory[input.source_address_memory_range.usizes()]
        .chunks_exact(Bytes32::LEN)
        .flat_map(|chunk| Some(Bytes32::from(<[u8; 32]>::try_from(chunk).ok()?)))
        .collect();

    let unset_count = storage
        .merkle_contract_state_insert_range(contract_id, destination_key, &values)
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
            // Overflow safety: unset_count * 32 can be at most VM_MAX_RAM
            (unset_count as u64) * (2 * Bytes32::LEN as u64) * new_storage_gas_per_byte,
        )?;
    }

    inc_pc(pc)?;

    Ok(())
}

struct StateClearQWord {
    /// The starting storage key location is stored
    /// in this range of memory.
    start_storage_key_memory_range: CheckedMemConstLen<{ Bytes32::LEN }>,
    /// Number of slots to read.
    num_slots: usize,
}

impl StateClearQWord {
    fn new(
        start_storage_key_memory_address: Word,
        num_slots: Word,
    ) -> SimpleResult<Self> {
        let start_storage_key_memory_range = CheckedMemConstLen::<{ Bytes32::LEN }>::new(
            start_storage_key_memory_address,
        )?;

        let num_slots = convert::to_usize(num_slots).ok_or(PanicReason::TooManySlots)?;

        Ok(Self {
            start_storage_key_memory_range,
            num_slots,
        })
    }
}

fn state_clear_qword<S: InterpreterStorage>(
    contract_id: &ContractId,
    storage: &mut S,
    memory: &[u8; MEM_SIZE],
    pc: RegMut<PC>,
    result_register: &mut Word,
    input: StateClearQWord,
) -> IoResult<(), S::DataError> {
    let start_key =
        Bytes32::from_bytes_ref(input.start_storage_key_memory_range.read(memory));

    let all_previously_set = storage
        .merkle_contract_state_remove_range(contract_id, start_key, input.num_slots)
        .map_err(RuntimeError::Storage)?
        .is_some();

    *result_register = all_previously_set as Word;

    inc_pc(pc)?;

    Ok(())
}
