use super::contract::{balance_decrease, balance_increase, contract_size};
use super::gas::{dependent_gas_charge, ProfileGas};
use super::internal::{
    append_receipt, external_asset_id_balance_sub, internal_contract, internal_contract_or_default, set_frame_pointer,
};
use super::{ExecutableTransaction, Interpreter, RuntimeBalances};
use crate::arith;
use crate::call::{Call, CallFrame};
use crate::constraints::reg_key::*;
use crate::constraints::*;
use crate::consts::*;
use crate::context::Context;
use crate::error::RuntimeError;
use crate::gas::DependentCost;
use crate::interpreter::PanicContext;
use crate::profiler::Profiler;
use crate::state::ProgramState;
use crate::storage::{ContractsAssets, ContractsAssetsStorage, ContractsRawCode, InterpreterStorage};

use fuel_asm::{Instruction, InstructionResult, RegId};
use fuel_crypto::Hasher;
use fuel_storage::{StorageAsRef, StorageInspect, StorageRead, StorageSize};
use fuel_tx::{ConsensusParameters, PanicReason, Receipt, Script};
use fuel_types::bytes::SerializableVec;
use fuel_types::{AssetId, Bytes32, ContractId, Word};
use std::{cmp, io};

#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn jump(&mut self, j: Word) -> Result<(), RuntimeError> {
        let j = self.registers[RegId::IS].saturating_add(j.saturating_mul(Instruction::SIZE as Word));

        if j > VM_MAX_RAM - 1 {
            Err(PanicReason::MemoryOverflow.into())
        } else {
            self.registers[RegId::PC] = j;

            Ok(())
        }
    }

    pub(crate) fn jump_not_equal(&mut self, a: Word, b: Word, to: Word) -> Result<(), RuntimeError> {
        if a != b {
            self.jump(to)
        } else {
            self.inc_pc()
        }
    }

    pub(crate) fn jump_not_zero(&mut self, a: Word, to: Word) -> Result<(), RuntimeError> {
        if a != self.registers[RegId::ZERO] {
            self.jump(to)
        } else {
            self.inc_pc()
        }
    }

    pub(crate) fn return_from_context(&mut self, receipt: Receipt) -> Result<(), RuntimeError> {
        if let Some(frame) = self.frames.pop() {
            self.registers[RegId::CGAS] = arith::add_word(self.registers[RegId::CGAS], frame.context_gas())?;

            let cgas = self.registers[RegId::CGAS];
            let ggas = self.registers[RegId::GGAS];
            let ret = self.registers[RegId::RET];
            let retl = self.registers[RegId::RETL];

            self.registers.copy_from_slice(frame.registers());

            self.registers[RegId::CGAS] = cgas;
            self.registers[RegId::GGAS] = ggas;
            self.registers[RegId::RET] = ret;
            self.registers[RegId::RETL] = retl;

            self.set_frame_pointer(self.registers[RegId::FP]);
        }

        self.append_receipt(receipt);

        self.inc_pc()
    }

    pub(crate) fn ret(&mut self, a: Word) -> Result<(), RuntimeError> {
        let receipt = Receipt::ret(
            self.internal_contract_or_default(),
            a,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.registers[RegId::RET] = a;
        self.registers[RegId::RETL] = 0;

        // TODO if ret instruction is in memory boundary, inc_pc shouldn't fail
        self.return_from_context(receipt)
    }

    pub(crate) fn ret_data(&mut self, a: Word, b: Word) -> Result<Bytes32, RuntimeError> {
        if b > MEM_MAX_ACCESS_SIZE || a > VM_MAX_RAM - b {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let ab = (a + b) as usize;
        let digest = Hasher::hash(&self.memory[a as usize..ab]);

        let receipt = Receipt::return_data_with_len(
            self.internal_contract_or_default(),
            a,
            b,
            digest,
            self.memory[a as usize..ab].to_vec(),
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.registers[RegId::RET] = a;
        self.registers[RegId::RETL] = b;

        self.return_from_context(receipt)?;

        Ok(digest)
    }

    pub(crate) fn revert(&mut self, a: Word) {
        let receipt = Receipt::revert(
            self.internal_contract_or_default(),
            a,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.append_receipt(receipt);
    }

    pub(crate) fn append_panic_receipt(&mut self, result: InstructionResult) {
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

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    fn _prepare_call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<(), RuntimeError> {
        let params = PrepareCallParams {
            call_params_mem_address: a,
            amount_of_coins_to_forward: b,
            asset_id_mem_address: c,
            amount_of_gas_to_forward: d,
        };
        let current_contract = if self.context.is_internal() {
            Some(*internal_contract(
                &self.context,
                self.registers.fp(),
                self.memory.as_ref(),
            )?)
        } else {
            None
        };
        let memory = PrepareCallMemory::try_from((self.memory.as_mut(), &params))?;
        let input_contracts = self.tx.input_contracts().copied().collect();

        PrepareCallInput {
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

    /// Prepare a call instruction for execution
    pub fn prepare_call(&mut self, ra: RegId, rb: RegId, rc: RegId, rd: RegId) -> Result<(), RuntimeError> {
        const M: &str = "the provided id is not a valid register";

        let a = self
            .registers
            .get(ra.to_u8() as usize)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, M))?;

        let b = self
            .registers
            .get(rb.to_u8() as usize)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, M))?;

        let c = self
            .registers
            .get(rc.to_u8() as usize)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, M))?;

        let d = self
            .registers
            .get(rd.to_u8() as usize)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, M))?;

        self._prepare_call(a, b, c, d)
    }

    pub(crate) fn call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<ProgramState, RuntimeError> {
        self._prepare_call(a, b, c, d)?;
        self.run_call()
    }
}

#[cfg_attr(test, derive(Default))]
struct PrepareCallParams {
    /// A
    pub call_params_mem_address: Word,
    /// B
    pub amount_of_coins_to_forward: Word,
    /// C
    pub asset_id_mem_address: Word,
    /// D
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
    read_registers: PrepareCallSystemRegisters<'a>,
    write_registers: ProgramRegistersRef<'a>,
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
        copy_registers(&self.into(), &self.write_registers)
    }
}

struct PrepareCallMemory<'a> {
    memory: &'a mut [u8; MEM_SIZE],
    call_params: CheckedMemValue<Call>,
    asset_id: CheckedMemValue<AssetId>,
}

struct PrepareCallInput<'vm, S> {
    params: PrepareCallParams,
    registers: PrepareCallRegisters<'vm>,
    memory: PrepareCallMemory<'vm>,
    context: &'vm mut Context,
    gas_cost: DependentCost,
    runtime_balances: &'vm mut RuntimeBalances,
    storage: &'vm mut S,
    input_contracts: Vec<fuel_types::ContractId>,
    panic_context: &'vm mut PanicContext,
    receipts: &'vm mut Vec<Receipt>,
    script: Option<&'vm mut Script>,
    consensus: &'vm ConsensusParameters,
    frames: &'vm mut Vec<CallFrame>,
    current_contract: Option<ContractId>,
    profiler: &'vm mut Profiler,
}

impl<'vm, S> PrepareCallInput<'vm, S> {
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
            pc: self.registers.read_registers.pc.as_ref(),
            is: self.registers.read_registers.is.as_ref(),
            current_contract: self.current_contract,
            profiler: self.profiler,
        };
        dependent_gas_charge(
            self.registers.read_registers.cgas.as_mut(),
            self.registers.read_registers.ggas.as_mut(),
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
            *self.registers.read_registers.cgas,
            self.params.amount_of_gas_to_forward,
        );

        // subtract gas
        *self.registers.read_registers.cgas = arith::sub_word(*self.registers.read_registers.cgas, forward_gas_amount)?;

        *frame.context_gas_mut() = *self.registers.read_registers.cgas;
        *frame.global_gas_mut() = *self.registers.read_registers.ggas;

        let frame_bytes = frame.to_bytes();
        let len = arith::add_word(frame_bytes.len() as Word, frame.total_code_size())?;

        if len > *self.registers.read_registers.hp
            || *self.registers.read_registers.sp > *self.registers.read_registers.hp - len
        {
            return Err(PanicReason::MemoryOverflow.into());
        }
        let id = internal_contract_or_default(
            self.context,
            self.registers.read_registers.fp.as_ref(),
            self.memory.memory,
        );

        let sp = *self.registers.read_registers.sp;
        set_frame_pointer(self.context, self.registers.read_registers.fp.as_mut(), sp);

        *self.registers.read_registers.sp = arith::checked_add_word(*self.registers.read_registers.sp, len)?;
        *self.registers.read_registers.ssp = *self.registers.read_registers.sp;

        let code_frame_mem_range = CheckedMemRange::new(*self.registers.read_registers.fp, len as usize)?;
        let frame_end = write_call_to_memory(
            &frame,
            frame_bytes,
            code_frame_mem_range,
            self.memory.memory,
            self.storage,
        )?;
        *self.registers.read_registers.bal = self.params.amount_of_coins_to_forward;
        *self.registers.read_registers.pc = frame_end;
        *self.registers.read_registers.is = *self.registers.read_registers.pc;
        *self.registers.read_registers.cgas = forward_gas_amount;

        let receipt = Receipt::call(
            id,
            *frame.to(),
            self.params.amount_of_coins_to_forward,
            *frame.asset_id(),
            self.params.amount_of_gas_to_forward,
            frame.a(),
            frame.b(),
            *self.registers.read_registers.pc,
            *self.registers.read_registers.is,
        );

        append_receipt(self.receipts, self.script, self.consensus, self.memory.memory, receipt);

        self.frames.push(frame);

        Ok(())
    }
}

fn write_call_to_memory<S>(
    frame: &CallFrame,
    frame_bytes: Vec<u8>,
    code_mem_range: CheckedMemRange,
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
    Ok(code_frame_range.end() as Word)
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
            hp: registers.read_registers.hp,
            sp: registers.read_registers.sp.as_ref(),
            ssp: registers.read_registers.ssp.as_ref(),
            fp: registers.read_registers.fp.as_ref(),
            pc: registers.read_registers.pc.as_ref(),
            is: registers.read_registers.is.as_ref(),
            bal: registers.read_registers.bal.as_ref(),
            cgas: registers.read_registers.cgas.as_ref(),
            ggas: registers.read_registers.ggas.as_ref(),
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
            read_registers: r,
            write_registers: w.into(),
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
