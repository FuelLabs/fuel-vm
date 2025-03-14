use crate::{
    call::{
        Call,
        CallFrame,
    },
    constraints::reg_key::*,
    consts::*,
    context::Context,
    error::{
        IoResult,
        RuntimeError,
        SimpleResult,
    },
    interpreter::{
        contract::{
            balance_decrease,
            balance_increase,
            contract_size,
        },
        gas::{
            dependent_gas_charge_without_base,
            gas_charge,
        },
        internal::{
            current_contract,
            external_asset_id_balance_sub,
            inc_pc,
            internal_contract,
            set_frame_pointer,
        },
        receipts::ReceiptsCtx,
        ExecutableTransaction,
        Interpreter,
        Memory,
        MemoryInstance,
        PanicContext,
        RuntimeBalances,
    },
    prelude::{
        Bug,
        BugVariant,
    },
    storage::{
        ContractsRawCode,
        InterpreterStorage,
    },
    verification::Verifier,
};
use alloc::{
    collections::BTreeSet,
    vec::Vec,
};
use core::cmp;
use fuel_asm::{
    Instruction,
    PanicInstruction,
    RegId,
};
use fuel_storage::{
    StorageAsRef,
    StorageRead,
    StorageSize,
};
use fuel_tx::{
    DependentCost,
    PanicReason,
    Receipt,
};
use fuel_types::{
    bytes::padded_len_usize,
    canonical::Serialize,
    AssetId,
    Bytes32,
    ContractId,
    Word,
};

#[cfg(test)]
mod jump_tests;
#[cfg(test)]
mod ret_tests;
#[cfg(test)]
mod tests;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    Tx: ExecutableTransaction,
{
    pub(crate) fn jump(&mut self, args: JumpArgs) -> SimpleResult<()> {
        let (SystemRegisters { pc, is, .. }, _) = split_registers(&mut self.registers);
        args.jump(is.as_ref(), pc)
    }

    pub(crate) fn ret(&mut self, a: Word) -> SimpleResult<()> {
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?;
        let input = RetCtx {
            receipts: &mut self.receipts,
            frames: &mut self.frames,
            registers: &mut self.registers,
            memory: self.memory.as_ref(),
            context: &mut self.context,
            current_contract,
        };
        input.ret(a)
    }

    pub(crate) fn ret_data(&mut self, a: Word, b: Word) -> SimpleResult<Bytes32> {
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?;
        let input = RetCtx {
            frames: &mut self.frames,
            registers: &mut self.registers,
            memory: self.memory.as_mut(),
            receipts: &mut self.receipts,
            context: &mut self.context,
            current_contract,
        };
        input.ret_data(a, b)
    }

    pub(crate) fn revert(&mut self, a: Word) -> SimpleResult<()> {
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())
                .unwrap_or(Some(ContractId::zeroed()));
        revert(
            &mut self.receipts,
            current_contract,
            self.registers.pc(),
            self.registers.is(),
            a,
        )
    }

    pub(crate) fn append_panic_receipt(&mut self, result: PanicInstruction) {
        let pc = self.registers[RegId::PC];
        let is = self.registers[RegId::IS];

        let mut receipt =
            Receipt::panic(self.internal_contract().unwrap_or_default(), result, pc, is);

        match self.panic_context {
            PanicContext::None => {}
            PanicContext::ContractId(contract_id) => {
                receipt = receipt.with_panic_contract_id(Some(contract_id));
            }
        };
        self.panic_context = PanicContext::None;

        self.receipts
            .push(receipt)
            .expect("Appending a panic receipt cannot fail");
    }
}

struct RetCtx<'vm> {
    frames: &'vm mut Vec<CallFrame>,
    registers: &'vm mut [Word; VM_REGISTER_COUNT],
    memory: &'vm MemoryInstance,
    receipts: &'vm mut ReceiptsCtx,
    context: &'vm mut Context,
    current_contract: Option<ContractId>,
}

impl RetCtx<'_> {
    pub(crate) fn ret(self, a: Word) -> SimpleResult<()> {
        let receipt = Receipt::ret(
            self.current_contract.unwrap_or_else(ContractId::zeroed),
            a,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.registers[RegId::RET] = a;
        self.registers[RegId::RETL] = 0;

        // TODO if ret instruction is in memory boundary, inc_pc shouldn't fail
        self.return_from_context(receipt)
    }

    pub(crate) fn return_from_context(mut self, receipt: Receipt) -> SimpleResult<()> {
        if let Some(frame) = self.frames.pop() {
            let registers = &mut self.registers;
            let context = &mut self.context;

            registers[RegId::CGAS] = registers[RegId::CGAS]
                .checked_add(frame.context_gas())
                .ok_or_else(|| Bug::new(BugVariant::ContextGasOverflow))?;

            let cgas = registers[RegId::CGAS];
            let ggas = registers[RegId::GGAS];
            let ret = registers[RegId::RET];
            let retl = registers[RegId::RETL];
            let hp = registers[RegId::HP];

            registers.copy_from_slice(frame.registers());

            registers[RegId::CGAS] = cgas;
            registers[RegId::GGAS] = ggas;
            registers[RegId::RET] = ret;
            registers[RegId::RETL] = retl;
            registers[RegId::HP] = hp;

            let fp = registers[RegId::FP];
            set_frame_pointer(context, registers.fp_mut(), fp);
        }

        self.receipts.push(receipt)?;

        Ok(inc_pc(self.registers.pc_mut())?)
    }

    pub(crate) fn ret_data(self, a: Word, b: Word) -> SimpleResult<Bytes32> {
        let data = self.memory.read(a, b)?.to_vec();

        let receipt = Receipt::return_data(
            self.current_contract.unwrap_or_else(ContractId::zeroed),
            a,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
            data,
        );
        let digest = *receipt
            .digest()
            .expect("Receipt is created above and `digest` should exist");

        self.registers[RegId::RET] = a;
        self.registers[RegId::RETL] = b;

        self.return_from_context(receipt)?;

        Ok(digest)
    }
}

pub(crate) fn revert(
    receipts: &mut ReceiptsCtx,
    current_contract: Option<ContractId>,
    pc: Reg<PC>,
    is: Reg<IS>,
    a: Word,
) -> SimpleResult<()> {
    let receipt = Receipt::revert(
        current_contract.unwrap_or_else(ContractId::zeroed),
        a,
        *pc,
        *is,
    );

    receipts.push(receipt)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JumpMode {
    /// `$pc = reg + (imm * instruction_size)`
    Assign,
    /// `$pc = $is + (reg + imm) * instruction_size)`
    RelativeIS,
    /// `$pc = $pc + (reg + imm + 1) * instruction_size`
    RelativeForwards,
    /// `$pc = $pc - (reg + imm + 1) * instruction_size`
    RelativeBackwards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JumpArgs {
    /// Condition. The jump is performed only if this is true.
    condition: bool,
    /// The kind of jump performed
    mode: JumpMode,
    /// Dynamic part of the jump target, i.e. register value
    dynamic: Word,
    /// Fixed part of the jump target, i.e. immediate value
    fixed: Word,
}

impl JumpArgs {
    pub(crate) fn new(mode: JumpMode) -> Self {
        Self {
            condition: true,
            mode,
            dynamic: 0,
            fixed: 0,
        }
    }

    pub(crate) fn with_condition(mut self, condition: bool) -> Self {
        self.condition = condition;
        self
    }

    pub(crate) fn to_address(mut self, addr: Word) -> Self {
        self.dynamic = addr;
        self
    }

    pub(crate) fn plus_fixed(mut self, addr: Word) -> Self {
        self.fixed = addr;
        self
    }

    pub(crate) fn jump(&self, is: Reg<IS>, mut pc: RegMut<PC>) -> SimpleResult<()> {
        if !self.condition {
            return Ok(inc_pc(pc)?)
        }

        let target_addr = match self.mode {
            JumpMode::Assign => self
                .dynamic
                .saturating_add(self.fixed.saturating_mul(Instruction::SIZE as Word)),
            JumpMode::RelativeIS => {
                let offset_instructions = self.dynamic.saturating_add(self.fixed);
                let offset_bytes =
                    offset_instructions.saturating_mul(Instruction::SIZE as Word);
                is.saturating_add(offset_bytes)
            }
            // In relative jumps, +1 is added since jumping to the jump instruction itself
            // is not useful
            JumpMode::RelativeForwards => {
                let offset_instructions =
                    self.dynamic.saturating_add(self.fixed).saturating_add(1);
                let offset_bytes =
                    offset_instructions.saturating_mul(Instruction::SIZE as Word);
                pc.saturating_add(offset_bytes)
            }
            JumpMode::RelativeBackwards => {
                let offset_instructions =
                    self.dynamic.saturating_add(self.fixed).saturating_add(1);
                let offset_bytes =
                    offset_instructions.saturating_mul(Instruction::SIZE as Word);
                pc.checked_sub(offset_bytes)
                    .ok_or(PanicReason::MemoryOverflow)?
            }
        };

        if target_addr >= VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into())
        }

        *pc = target_addr;
        Ok(())
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    V: Verifier,
{
    /// Prepare a call instruction for execution
    pub fn prepare_call(
        &mut self,
        ra: RegId,
        rb: RegId,
        rc: RegId,
        rd: RegId,
    ) -> IoResult<(), S::DataError> {
        self.prepare_call_inner(
            self.registers[ra],
            self.registers[rb],
            self.registers[rc],
            self.registers[rd],
        )
    }

    /// Prepare a call instruction for execution
    fn prepare_call_inner(
        &mut self,
        call_params_pointer: Word,
        amount_of_coins_to_forward: Word,
        asset_id_pointer: Word,
        amount_of_gas_to_forward: Word,
    ) -> IoResult<(), S::DataError> {
        let params = PrepareCallParams {
            call_params_pointer,
            asset_id_pointer,
            amount_of_coins_to_forward,
            amount_of_gas_to_forward,
        };
        let gas_cost = self.gas_costs().call();
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        // Charge only for the `base` execution.
        // We will charge for the frame size in the `prepare_call`.
        self.gas_charge(gas_cost.base())?;
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?;

        PrepareCallCtx {
            params,
            registers: (&mut self.registers).into(),
            memory: self.memory.as_mut(),
            context: &mut self.context,
            gas_cost,
            runtime_balances: &mut self.balances,
            storage: &mut self.storage,
            input_contracts: &self.input_contracts,
            panic_context: &mut self.panic_context,
            new_storage_gas_per_byte,
            receipts: &mut self.receipts,
            frames: &mut self.frames,
            current_contract,
            verifier: &mut self.verifier,
        }
        .prepare_call()
    }
}

#[cfg_attr(test, derive(Default))]
struct PrepareCallParams {
    /// Register A of input
    pub call_params_pointer: Word,
    /// Register B of input
    pub amount_of_coins_to_forward: Word,
    /// Register C of input
    pub asset_id_pointer: Word,
    /// Register D of input
    pub amount_of_gas_to_forward: Word,
}

struct PrepareCallSystemRegisters<'a> {
    hp: Reg<'a, HP>,
    sp: RegMut<'a, SP>,
    ssp: RegMut<'a, SSP>,
    fp: RegMut<'a, FP>,
    pc: RegMut<'a, PC>,
    is: RegMut<'a, IS>,
    bal: RegMut<'a, BAL>,
    cgas: RegMut<'a, CGAS>,
    ggas: RegMut<'a, GGAS>,
    flag: RegMut<'a, FLAG>,
}

struct PrepareCallRegisters<'a> {
    system_registers: PrepareCallSystemRegisters<'a>,
    program_registers: ProgramRegistersRef<'a>,
    unused_registers: PrepareCallUnusedRegisters<'a>,
}

struct PrepareCallUnusedRegisters<'a> {
    zero: Reg<'a, ZERO>,
    one: Reg<'a, ONE>,
    of: Reg<'a, OF>,
    err: Reg<'a, ERR>,
    ret: Reg<'a, RET>,
    retl: Reg<'a, RETL>,
}

impl PrepareCallRegisters<'_> {
    fn copy_registers(&self) -> [Word; VM_REGISTER_COUNT] {
        copy_registers(&self.into(), &self.program_registers)
    }
}

struct PrepareCallCtx<'vm, S, V> {
    params: PrepareCallParams,
    registers: PrepareCallRegisters<'vm>,
    memory: &'vm mut MemoryInstance,
    context: &'vm mut Context,
    gas_cost: DependentCost,
    runtime_balances: &'vm mut RuntimeBalances,
    new_storage_gas_per_byte: Word,
    storage: &'vm mut S,
    input_contracts: &'vm BTreeSet<ContractId>,
    panic_context: &'vm mut PanicContext,
    receipts: &'vm mut ReceiptsCtx,
    frames: &'vm mut Vec<CallFrame>,
    current_contract: Option<ContractId>,
    verifier: &'vm mut V,
}

impl<S, V> PrepareCallCtx<'_, S, V> {
    fn prepare_call(mut self) -> IoResult<(), S::DataError>
    where
        S: InterpreterStorage,
        V: Verifier,
    {
        let call_bytes = self
            .memory
            .read(self.params.call_params_pointer, Call::LEN)?;
        let call = Call::try_from(call_bytes)?;
        let asset_id =
            AssetId::new(self.memory.read_bytes(self.params.asset_id_pointer)?);

        let code_size = contract_size(&self.storage, call.to())? as usize;
        let code_size_padded =
            padded_len_usize(code_size).ok_or(PanicReason::MemoryOverflow)?;

        let total_size_in_stack = CallFrame::serialized_size()
            .checked_add(code_size_padded)
            .ok_or_else(|| Bug::new(BugVariant::CodeSizeOverflow))?;

        dependent_gas_charge_without_base(
            self.registers.system_registers.cgas.as_mut(),
            self.registers.system_registers.ggas.as_mut(),
            self.gas_cost,
            code_size_padded as Word,
        )?;

        if let Some(source_contract) = self.current_contract {
            balance_decrease(
                self.storage,
                &source_contract,
                &asset_id,
                self.params.amount_of_coins_to_forward,
            )?;
        } else {
            let amount = self.params.amount_of_coins_to_forward;
            external_asset_id_balance_sub(
                self.runtime_balances,
                self.memory,
                &asset_id,
                amount,
            )?;
        }

        self.verifier.check_contract_in_inputs(
            self.panic_context,
            self.input_contracts,
            call.to(),
        )?;

        // credit contract asset_id balance
        let (_, created_new_entry) = balance_increase(
            self.storage,
            call.to(),
            &asset_id,
            self.params.amount_of_coins_to_forward,
        )?;

        if created_new_entry {
            // If a new entry was created, we must charge gas for it
            gas_charge(
                self.registers.system_registers.cgas.as_mut(),
                self.registers.system_registers.ggas.as_mut(),
                ((Bytes32::LEN + WORD_SIZE) as u64)
                    .saturating_mul(self.new_storage_gas_per_byte),
            )?;
        }

        let forward_gas_amount = cmp::min(
            *self.registers.system_registers.cgas,
            self.params.amount_of_gas_to_forward,
        );

        // subtract gas
        *self.registers.system_registers.cgas = (*self.registers.system_registers.cgas)
            .checked_sub(forward_gas_amount)
            .ok_or_else(|| Bug::new(BugVariant::ContextGasUnderflow))?;

        // Construct frame
        let mut frame = CallFrame::new(
            *call.to(),
            asset_id,
            self.registers.copy_registers(),
            code_size_padded,
            call.a(),
            call.b(),
        )
        .ok_or(PanicReason::MemoryOverflow)?;
        *frame.context_gas_mut() = *self.registers.system_registers.cgas;
        *frame.global_gas_mut() = *self.registers.system_registers.ggas;

        // Allocate stack memory
        let old_sp = *self.registers.system_registers.sp;
        let new_sp = old_sp.saturating_add(total_size_in_stack as Word);
        self.memory.grow_stack(new_sp)?;
        *self.registers.system_registers.sp = new_sp;
        *self.registers.system_registers.ssp = new_sp;

        let id = internal_contract(
            self.context,
            self.registers.system_registers.fp.as_ref(),
            self.memory,
        )
        .unwrap_or_default();

        set_frame_pointer(
            self.context,
            self.registers.system_registers.fp.as_mut(),
            old_sp,
        );

        // Write the frame to memory
        // Ownership checks are disabled because we just allocated the memory above.
        let dst = self.memory.write_noownerchecks(
            *self.registers.system_registers.fp,
            total_size_in_stack,
        )?;
        let (mem_frame, mem_code) = dst.split_at_mut(CallFrame::serialized_size());
        mem_frame.copy_from_slice(&frame.to_bytes());
        let (mem_code, mem_code_padding) = mem_code.split_at_mut(code_size);
        read_contract(call.to(), self.storage, mem_code)?;
        mem_code_padding.fill(0);

        #[allow(clippy::arithmetic_side_effects)] // Checked above
        let code_start =
            (*self.registers.system_registers.fp) + CallFrame::serialized_size() as Word;

        *self.registers.system_registers.pc = code_start;
        *self.registers.system_registers.bal = self.params.amount_of_coins_to_forward;
        *self.registers.system_registers.is = *self.registers.system_registers.pc;
        *self.registers.system_registers.cgas = forward_gas_amount;
        *self.registers.system_registers.flag = 0;

        let receipt = Receipt::call(
            id,
            *call.to(),
            self.params.amount_of_coins_to_forward,
            asset_id,
            forward_gas_amount,
            call.a(),
            call.b(),
            *self.registers.system_registers.pc,
            *self.registers.system_registers.is,
        );

        self.receipts.push(receipt)?;

        self.frames.push(frame);

        Ok(())
    }
}

fn read_contract<S>(
    contract: &ContractId,
    storage: &S,
    dst: &mut [u8],
) -> IoResult<(), S::Error>
where
    S: StorageSize<ContractsRawCode> + StorageRead<ContractsRawCode> + StorageAsRef,
{
    if !storage
        .storage::<ContractsRawCode>()
        .read(contract, 0, dst)
        .map_err(RuntimeError::Storage)?
    {
        return Err(PanicReason::ContractNotFound.into());
    }
    Ok(())
}

impl<'a> From<&'a PrepareCallRegisters<'_>> for SystemRegistersRef<'a> {
    fn from(registers: &'a PrepareCallRegisters) -> Self {
        Self {
            hp: registers.system_registers.hp,
            sp: registers.system_registers.sp.as_ref(),
            ssp: registers.system_registers.ssp.as_ref(),
            fp: registers.system_registers.fp.as_ref(),
            pc: registers.system_registers.pc.as_ref(),
            is: registers.system_registers.is.as_ref(),
            bal: registers.system_registers.bal.as_ref(),
            cgas: registers.system_registers.cgas.as_ref(),
            ggas: registers.system_registers.ggas.as_ref(),
            flag: registers.system_registers.flag.as_ref(),
            zero: registers.unused_registers.zero,
            one: registers.unused_registers.one,
            of: registers.unused_registers.of,
            err: registers.unused_registers.err,
            ret: registers.unused_registers.ret,
            retl: registers.unused_registers.retl,
        }
    }
}

impl<'reg> From<&'reg mut [Word; VM_REGISTER_COUNT]> for PrepareCallRegisters<'reg> {
    fn from(registers: &'reg mut [Word; VM_REGISTER_COUNT]) -> Self {
        let (r, w) = split_registers(registers);
        let (r, u) = r.into();
        Self {
            system_registers: r,
            program_registers: w.into(),
            unused_registers: u,
        }
    }
}

impl<'reg> From<SystemRegisters<'reg>>
    for (
        PrepareCallSystemRegisters<'reg>,
        PrepareCallUnusedRegisters<'reg>,
    )
{
    fn from(registers: SystemRegisters<'reg>) -> Self {
        let read = PrepareCallSystemRegisters {
            hp: registers.hp.into(),
            sp: registers.sp,
            ssp: registers.ssp,
            fp: registers.fp,
            pc: registers.pc,
            is: registers.is,
            bal: registers.bal,
            cgas: registers.cgas,
            ggas: registers.ggas,
            flag: registers.flag,
        };

        (
            read,
            PrepareCallUnusedRegisters {
                zero: registers.zero.into(),
                one: registers.one.into(),
                of: registers.of.into(),
                err: registers.err.into(),
                ret: registers.ret.into(),
                retl: registers.retl.into(),
            },
        )
    }
}
