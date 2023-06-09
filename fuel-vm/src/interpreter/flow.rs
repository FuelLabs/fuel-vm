use super::contract::{balance_decrease, balance_increase, contract_size};
use super::gas::{dependent_gas_charge, ProfileGas};
use super::internal::{
    append_receipt, current_contract, external_asset_id_balance_sub, inc_pc, internal_contract_or_default,
    set_frame_pointer, AppendReceipt,
};
use super::{ExecutableTransaction, Interpreter, MemoryRange, RuntimeBalances};
use crate::arith;
use crate::call::{Call, CallFrame};
use crate::constraints::reg_key::*;
use crate::constraints::*;
use crate::consts::*;
use crate::context::Context;
use crate::error::RuntimeError;
use crate::gas::DependentCost;
use crate::interpreter::receipts::ReceiptsCtx;
use crate::interpreter::PanicContext;
use crate::profiler::Profiler;
use crate::storage::{ContractsAssets, ContractsAssetsStorage, ContractsRawCode, InterpreterStorage};

use fuel_asm::{Instruction, PanicInstruction, RegId};
use fuel_crypto::Hasher;
use fuel_storage::{StorageAsRef, StorageInspect, StorageRead, StorageSize};
use fuel_tx::{ConsensusParameters, PanicReason, Receipt, Script};
use fuel_types::bytes::SerializableVec;
use fuel_types::{AssetId, Bytes32, ContractId, Word};
use std::cmp;

#[cfg(test)]
mod jump_tests;
#[cfg(test)]
mod ret_tests;
#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn jump(&mut self, args: JumpArgs) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, is, .. }, _) = split_registers(&mut self.registers);
        args.jump(is.as_ref(), pc)
    }

    pub(crate) fn ret(&mut self, a: Word) -> Result<(), RuntimeError> {
        let current_contract = current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?.copied();
        let input = RetCtx {
            append: AppendReceipt {
                receipts: &mut self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset: self.params.tx_offset(),
                memory: &mut self.memory,
            },
            frames: &mut self.frames,
            registers: &mut self.registers,
            context: &mut self.context,
            current_contract,
        };
        input.ret(a)
    }

    pub(crate) fn ret_data(&mut self, a: Word, b: Word) -> Result<Bytes32, RuntimeError> {
        let current_contract = current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?.copied();
        let input = RetCtx {
            append: AppendReceipt {
                receipts: &mut self.receipts,
                script: self.tx.as_script_mut(),
                tx_offset: self.params.tx_offset(),
                memory: &mut self.memory,
            },
            frames: &mut self.frames,
            registers: &mut self.registers,
            context: &mut self.context,
            current_contract,
        };
        input.ret_data(a, b)
    }

    pub(crate) fn revert(&mut self, a: Word) {
        let current_contract = current_contract(&self.context, self.registers.fp(), self.memory.as_ref())
            .map_or_else(|_| Some(ContractId::zeroed()), Option::<&_>::copied);
        let append = AppendReceipt {
            receipts: &mut self.receipts,
            script: self.tx.as_script_mut(),
            tx_offset: self.params.tx_offset(),
            memory: &mut self.memory,
        };
        revert(append, current_contract, self.registers.pc(), self.registers.is(), a)
    }

    pub(crate) fn append_panic_receipt(&mut self, result: PanicInstruction) {
        let pc = self.registers[RegId::PC];
        let is = self.registers[RegId::IS];

        let mut receipt = Receipt::panic(self.internal_contract_or_default(), result, pc, is);

        match self.panic_context {
            PanicContext::None => {}
            PanicContext::ContractId(contract_id) => {
                receipt = receipt.with_panic_contract_id(Some(contract_id));
            }
        };
        self.panic_context = PanicContext::None;

        self.append_receipt(receipt);
    }
}

struct RetCtx<'vm> {
    frames: &'vm mut Vec<CallFrame>,
    registers: &'vm mut [Word; VM_REGISTER_COUNT],
    append: AppendReceipt<'vm>,
    context: &'vm mut Context,
    current_contract: Option<ContractId>,
}

impl RetCtx<'_> {
    pub(crate) fn ret(self, a: Word) -> Result<(), RuntimeError> {
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

    pub(crate) fn return_from_context(mut self, receipt: Receipt) -> Result<(), RuntimeError> {
        if let Some(frame) = self.frames.pop() {
            let registers = &mut self.registers;
            let context = &mut self.context;

            registers[RegId::CGAS] = arith::add_word(registers[RegId::CGAS], frame.context_gas())?;

            let cgas = registers[RegId::CGAS];
            let ggas = registers[RegId::GGAS];
            let ret = registers[RegId::RET];
            let retl = registers[RegId::RETL];

            registers.copy_from_slice(frame.registers());

            registers[RegId::CGAS] = cgas;
            registers[RegId::GGAS] = ggas;
            registers[RegId::RET] = ret;
            registers[RegId::RETL] = retl;

            let fp = registers[RegId::FP];
            set_frame_pointer(context, registers.fp_mut(), fp);
        }

        append_receipt(self.append, receipt);

        inc_pc(self.registers.pc_mut())
    }

    pub(crate) fn ret_data(self, a: Word, b: Word) -> Result<Bytes32, RuntimeError> {
        if b > MEM_MAX_ACCESS_SIZE || a > VM_MAX_RAM - b {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let ab = (a + b) as usize;
        let digest = Hasher::hash(&self.append.memory[a as usize..ab]);

        let receipt = Receipt::return_data_with_len(
            self.current_contract.unwrap_or_else(ContractId::zeroed),
            a,
            b,
            digest,
            self.append.memory[a as usize..ab].to_vec(),
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.registers[RegId::RET] = a;
        self.registers[RegId::RETL] = b;

        self.return_from_context(receipt)?;

        Ok(digest)
    }
}

pub(crate) fn revert(append: AppendReceipt, current_contract: Option<ContractId>, pc: Reg<PC>, is: Reg<IS>, a: Word) {
    let receipt = Receipt::revert(current_contract.unwrap_or_else(ContractId::zeroed), a, *pc, *is);

    append_receipt(append, receipt);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JumpMode {
    /// `$pc = $is + address`
    Absolute,
    /// `$pc = $pc + address`
    RelativeForwards,
    /// `$pc = $pc - address`
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

    pub(crate) fn jump(&self, is: Reg<IS>, mut pc: RegMut<PC>) -> Result<(), RuntimeError> {
        if !self.condition {
            return inc_pc(pc);
        }

        let offset_instructions = match self.mode {
            JumpMode::Absolute => self.dynamic.saturating_add(self.fixed),
            // Here +1 is added since jumping to the jump instruction itself doesn't make sense
            JumpMode::RelativeForwards | JumpMode::RelativeBackwards => {
                self.dynamic.saturating_add(self.fixed).saturating_add(1)
            }
        };

        let offset_bytes = offset_instructions.saturating_mul(Instruction::SIZE as Word);

        let target_addr = match self.mode {
            JumpMode::Absolute => is.saturating_add(offset_bytes),
            JumpMode::RelativeForwards => pc.saturating_add(offset_bytes),
            JumpMode::RelativeBackwards => pc
                .checked_sub(offset_bytes)
                .ok_or_else(|| RuntimeError::Recoverable(PanicReason::MemoryOverflow))?,
        };

        if target_addr >= VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        *pc = target_addr;
        Ok(())
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Prepare a call instruction for execution
    pub(crate) fn prepare_call(
        &mut self,
        call_params_mem_address: Word,
        amount_of_coins_to_forward: Word,
        asset_id_mem_address: Word,
        amount_of_gas_to_forward: Word,
    ) -> Result<(), RuntimeError> {
        let params = PrepareCallParams {
            call_params_mem_address,
            amount_of_coins_to_forward,
            asset_id_mem_address,
            amount_of_gas_to_forward,
        };
        let current_contract = current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?.copied();
        let memory = PrepareCallMemory::try_from((self.memory.as_mut(), &params))?;
        let input_contracts = self.tx.input_contracts().copied().collect();

        PrepareCallCtx {
            params,
            registers: (&mut self.registers).into(),
            memory,
            context: &mut self.context,
            gas_cost: self.gas_costs.call,
            runtime_balances: &mut self.balances,
            storage: &mut self.storage,
            input_contracts,
            panic_context: &mut self.panic_context,
            receipts: &mut self.receipts,
            script: self.tx.as_script_mut(),
            consensus: &self.params,
            frames: &mut self.frames,
            current_contract,
            profiler: &mut self.profiler,
        }
        .prepare_call()
    }
}

#[cfg_attr(test, derive(Default))]
struct PrepareCallParams {
    /// Register A of input
    pub call_params_mem_address: Word,
    /// Register B of input
    pub amount_of_coins_to_forward: Word,
    /// Register C of input
    pub asset_id_mem_address: Word,
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
    flag: Reg<'a, FLAG>,
}

impl<'a> PrepareCallRegisters<'a> {
    fn copy_registers(&self) -> [Word; VM_REGISTER_COUNT] {
        copy_registers(&self.into(), &self.program_registers)
    }
}

struct PrepareCallMemory<'a> {
    memory: &'a mut [u8; MEM_SIZE],
    call_params: CheckedMemValue<Call>,
    asset_id: CheckedMemValue<AssetId>,
}

struct PrepareCallCtx<'vm, S> {
    params: PrepareCallParams,
    registers: PrepareCallRegisters<'vm>,
    memory: PrepareCallMemory<'vm>,
    context: &'vm mut Context,
    gas_cost: DependentCost,
    runtime_balances: &'vm mut RuntimeBalances,
    storage: &'vm mut S,
    input_contracts: Vec<fuel_types::ContractId>,
    panic_context: &'vm mut PanicContext,
    receipts: &'vm mut ReceiptsCtx,
    script: Option<&'vm mut Script>,
    consensus: &'vm ConsensusParameters,
    frames: &'vm mut Vec<CallFrame>,
    current_contract: Option<ContractId>,
    profiler: &'vm mut Profiler,
}

impl<'vm, S> PrepareCallCtx<'vm, S> {
    fn prepare_call(mut self) -> Result<(), RuntimeError>
    where
        S: StorageSize<ContractsRawCode> + ContractsAssetsStorage + StorageRead<ContractsRawCode> + StorageAsRef,
        <S as StorageInspect<ContractsRawCode>>::Error: Into<std::io::Error>,
        <S as StorageInspect<ContractsAssets>>::Error: Into<std::io::Error>,
    {
        let call = self.memory.call_params.try_from(self.memory.memory)?;
        let asset_id = self.memory.asset_id.try_from(self.memory.memory)?;

        let mut frame = call_frame(self.registers.copy_registers(), &self.storage, call, asset_id)?;

        let profiler = ProfileGas {
            pc: self.registers.system_registers.pc.as_ref(),
            is: self.registers.system_registers.is.as_ref(),
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge(
            self.registers.system_registers.cgas.as_mut(),
            self.registers.system_registers.ggas.as_mut(),
            profiler,
            self.gas_cost,
            frame.total_code_size(),
        )?;

        if let Some(source_contract) = self.current_contract {
            balance_decrease(
                self.storage,
                &source_contract,
                frame.asset_id(),
                self.params.amount_of_coins_to_forward,
            )?;
        } else {
            let amount = self.params.amount_of_coins_to_forward;
            external_asset_id_balance_sub(self.runtime_balances, self.memory.memory, frame.asset_id(), amount)?;
        }

        if !self.input_contracts.iter().any(|contract| call.to() == contract) {
            *self.panic_context = PanicContext::ContractId(*call.to());
            return Err(PanicReason::ContractNotInInputs.into());
        }

        // credit contract asset_id balance
        balance_increase(
            self.storage,
            call.to(),
            &asset_id,
            self.params.amount_of_coins_to_forward,
        )?;

        let forward_gas_amount = cmp::min(
            *self.registers.system_registers.cgas,
            self.params.amount_of_gas_to_forward,
        );

        // subtract gas
        *self.registers.system_registers.cgas =
            arith::sub_word(*self.registers.system_registers.cgas, forward_gas_amount)?;

        *frame.context_gas_mut() = *self.registers.system_registers.cgas;
        *frame.global_gas_mut() = *self.registers.system_registers.ggas;

        let frame_bytes = frame.to_bytes();
        let len = arith::add_word(frame_bytes.len() as Word, frame.total_code_size())?;

        if len > *self.registers.system_registers.hp
            || *self.registers.system_registers.sp > *self.registers.system_registers.hp - len
        {
            return Err(PanicReason::MemoryOverflow.into());
        }
        let id = internal_contract_or_default(
            self.context,
            self.registers.system_registers.fp.as_ref(),
            self.memory.memory,
        );

        let old_sp = *self.registers.system_registers.sp;
        let new_sp = arith::checked_add_word(old_sp, len)?;

        set_frame_pointer(self.context, self.registers.system_registers.fp.as_mut(), old_sp);
        *self.registers.system_registers.sp = new_sp;
        *self.registers.system_registers.ssp = new_sp;

        let code_frame_mem_range = MemoryRange::new(*self.registers.system_registers.fp, len)?;
        let frame_end = write_call_to_memory(
            &frame,
            frame_bytes,
            code_frame_mem_range,
            self.memory.memory,
            self.storage,
        )?;
        *self.registers.system_registers.bal = self.params.amount_of_coins_to_forward;
        *self.registers.system_registers.pc = frame_end;
        *self.registers.system_registers.is = *self.registers.system_registers.pc;
        *self.registers.system_registers.cgas = forward_gas_amount;

        let receipt = Receipt::call(
            id,
            *frame.to(),
            self.params.amount_of_coins_to_forward,
            *frame.asset_id(),
            self.params.amount_of_gas_to_forward,
            frame.a(),
            frame.b(),
            *self.registers.system_registers.pc,
            *self.registers.system_registers.is,
        );

        append_receipt(
            AppendReceipt {
                receipts: self.receipts,
                script: self.script,
                tx_offset: self.consensus.tx_offset(),
                memory: self.memory.memory,
            },
            receipt,
        );

        self.frames.push(frame);

        Ok(())
    }
}

fn write_call_to_memory<S>(
    frame: &CallFrame,
    frame_bytes: Vec<u8>,
    code_mem_range: MemoryRange,
    memory: &mut [u8; MEM_SIZE],
    storage: &S,
) -> Result<Word, RuntimeError>
where
    S: StorageSize<ContractsRawCode> + StorageRead<ContractsRawCode> + StorageAsRef,
    <S as StorageInspect<ContractsRawCode>>::Error: Into<std::io::Error>,
{
    let mut code_frame_range = code_mem_range.clone();
    // Addition is safe because code size + padding is always less than len
    code_frame_range.shrink_end((frame.code_size() + frame.code_size_padding()) as usize);
    code_frame_range.clone().write(memory).copy_from_slice(&frame_bytes);

    let mut code_range = code_mem_range.clone();
    code_range.grow_start(CallFrame::serialized_size());
    code_range.shrink_end(frame.code_size_padding() as usize);
    let bytes_read = storage
        .storage::<ContractsRawCode>()
        .read(frame.to(), code_range.write(memory))
        .map_err(RuntimeError::from_io)?
        .ok_or(PanicReason::ContractNotFound)?;
    if bytes_read as Word != frame.code_size() {
        return Err(PanicReason::ContractMismatch.into());
    }

    if frame.code_size_padding() > 0 {
        let mut padding_range = code_mem_range;
        padding_range.grow_start(CallFrame::serialized_size() + frame.code_size() as usize);
        padding_range.write(memory).fill(0);
    }
    Ok(code_frame_range.end as Word)
}

fn call_frame<S>(
    registers: [Word; VM_REGISTER_COUNT],
    storage: &S,
    call: Call,
    asset_id: AssetId,
) -> Result<CallFrame, RuntimeError>
where
    S: StorageSize<ContractsRawCode> + ?Sized,
    <S as StorageInspect<ContractsRawCode>>::Error: Into<std::io::Error>,
{
    let (to, a, b) = call.into_inner();

    let code_size = contract_size(storage, &to)?;

    let frame = CallFrame::new(to, asset_id, registers, code_size, a, b);

    Ok(frame)
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
            zero: registers.unused_registers.zero,
            one: registers.unused_registers.one,
            of: registers.unused_registers.of,
            err: registers.unused_registers.err,
            ret: registers.unused_registers.ret,
            retl: registers.unused_registers.retl,
            flag: registers.unused_registers.flag,
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

impl<'reg> From<SystemRegisters<'reg>> for (PrepareCallSystemRegisters<'reg>, PrepareCallUnusedRegisters<'reg>) {
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
                flag: registers.flag.into(),
            },
        )
    }
}

impl<'mem> TryFrom<(&'mem mut [u8; MEM_SIZE], &PrepareCallParams)> for PrepareCallMemory<'mem> {
    type Error = RuntimeError;
    fn try_from((memory, params): (&'mem mut [u8; MEM_SIZE], &PrepareCallParams)) -> Result<Self, Self::Error> {
        Ok(Self {
            memory,
            call_params: CheckedMemValue::new::<{ Call::LEN }>(params.call_params_mem_address)?,
            asset_id: CheckedMemValue::new::<{ AssetId::LEN }>(params.asset_id_mem_address)?,
        })
    }
}
