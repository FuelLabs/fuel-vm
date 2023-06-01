use super::contract::{balance_decrease, balance_increase, contract_size};
use super::internal::{external_asset_id_balance_sub, inc_pc, set_frame_pointer};
use super::{ExecutableTransaction, Interpreter, MemoryRange};
use crate::arith;
use crate::call::{Call, CallFrame};
use crate::constraints::reg_key::*;
use crate::consts::*;
use crate::error::RuntimeError;
use crate::interpreter::PanicContext;
use crate::storage::{ContractsRawCode, InterpreterStorage};

use fuel_asm::{Instruction, PanicInstruction, RegId};
use fuel_crypto::Hasher;
use fuel_storage::{StorageAsRef, StorageInspect, StorageRead, StorageSize};
use fuel_tx::{PanicReason, Receipt};
use fuel_types::bytes::SerializableVec;
use fuel_types::{AssetId, Bytes32, ContractId, Word};
use std::{cmp, io};

#[cfg(test)]
mod jump_tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn jump(&mut self, args: JumpArgs) -> Result<(), RuntimeError> {
        let (SystemRegisters { pc, is, .. }, _) = split_registers(&mut self.registers);
        args.jump(is.as_ref(), pc)
    }

    fn return_from_context(&mut self, receipt: Receipt) -> Result<(), RuntimeError> {
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

        self.append_receipt(receipt);

        inc_pc(self.registers.pc_mut());
        Ok(())
    }

    pub(crate) fn ret(&mut self, a: Word) -> Result<(), RuntimeError> {
        let current_contract = self.current_contract()?;
        let receipt = Receipt::ret(
            current_contract.unwrap_or_else(ContractId::zeroed),
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
        let current_contract = self.current_contract()?;

        let data = self.mem_read(a, b)?;
        let digest = Hasher::hash(data);

        let receipt = Receipt::return_data_with_len(
            current_contract.unwrap_or_default(),
            a,
            b,
            digest,
            data.to_vec(),
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.registers[RegId::RET] = a;
        self.registers[RegId::RETL] = b;

        self.return_from_context(receipt)?;

        Ok(digest)
    }

    pub(crate) fn revert(&mut self, a: Word) {
        let current_contract = self.current_contract().transpose().map(|v| v.unwrap_or_default());
        self.append_receipt(Receipt::revert(
            current_contract.unwrap_or_else(ContractId::zeroed),
            a,
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        ));
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
            inc_pc(pc);
            return Ok(());
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
            JumpMode::RelativeBackwards => pc.checked_sub(offset_bytes).ok_or(PanicReason::MemoryAccess)?,
        };

        if target_addr >= VM_MAX_RAM {
            return Err(PanicReason::MemoryAccess.into());
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
    fn prepare_call_inner(
        &mut self,
        call_params_ptr: Word,
        amount_of_coins_to_forward: Word,
        asset_id_ptr: Word,
        amount_of_gas_to_forward: Word,
    ) -> Result<(), RuntimeError> {
        let current_contract = self.current_contract()?;

        let call = Call::from(self.mem_read_bytes(call_params_ptr)?);
        let asset_id = AssetId::from(self.mem_read_bytes(asset_id_ptr)?);

        let mut frame = call_frame(self.registers, &self.storage, call, asset_id)?;

        self.dependent_gas_charge(self.gas_costs.call, frame.total_code_size())?;

        if let Some(source_contract) = current_contract {
            balance_decrease(
                &mut self.storage,
                &source_contract,
                frame.asset_id(),
                amount_of_coins_to_forward,
            )?;
        } else {
            external_asset_id_balance_sub(
                &mut self.balances,
                &mut self.memory,
                frame.asset_id(),
                amount_of_coins_to_forward,
            )?;
        }

        if !self.tx.input_contracts().any(|contract| call.to() == contract) {
            self.panic_context = PanicContext::ContractId(*call.to());
            return Err(PanicReason::ContractNotInInputs.into());
        }

        // credit contract asset_id balance
        balance_increase(&mut self.storage, call.to(), &asset_id, amount_of_coins_to_forward)?;

        let forward_gas_amount = cmp::min(self.registers[RegId::CGAS], amount_of_gas_to_forward);

        // subtract gas
        self.registers[RegId::CGAS] = arith::sub_word(self.registers[RegId::CGAS], forward_gas_amount)?;

        *frame.context_gas_mut() = self.registers[RegId::CGAS];
        *frame.global_gas_mut() = self.registers[RegId::GGAS];

        let frame_bytes = frame.to_bytes();

        // TODO: do this using reserve_stack

        let len = arith::add_word(frame_bytes.len() as Word, frame.total_code_size())?;

        if len > self.registers[RegId::HP] || self.registers[RegId::SP] > self.registers[RegId::HP] - len {
            return Err(PanicReason::OutOfMemory.into());
        }

        let id = self.internal_contract_or_default();

        let new_fp = self.registers[RegId::SP];
        set_frame_pointer(&mut self.context, self.registers.fp_mut(), new_fp);

        self.registers[RegId::SP] = arith::checked_add_word(self.registers[RegId::SP], len)?;

        self.update_allocations()?;

        let dst_range = MemoryRange::try_new(self.registers[RegId::FP], len)?;
        self.check_mem_owned(&dst_range)?;
        write_call_to_memory(self.memory.write(&dst_range), &frame, frame_bytes, &self.storage)?;

        self.registers[RegId::SSP] = self.registers[RegId::SP];

        self.registers[RegId::BAL] = amount_of_coins_to_forward;
        self.registers[RegId::PC] = self.registers[RegId::FP] + CallFrame::serialized_size() as Word;
        self.registers[RegId::IS] = self.registers[RegId::PC];
        self.registers[RegId::CGAS] = forward_gas_amount;

        let receipt = Receipt::call(
            id,
            *frame.to(),
            amount_of_coins_to_forward,
            *frame.asset_id(),
            amount_of_gas_to_forward,
            frame.a(),
            frame.b(),
            self.registers[RegId::PC],
            self.registers[RegId::IS],
        );

        self.append_receipt(receipt);

        self.frames.push(frame);

        Ok(())
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

        self.prepare_call_inner(a, b, c, d)
    }
}

fn write_call_to_memory<S>(
    dst: &mut [u8],
    frame: &CallFrame,
    frame_bytes: Vec<u8>,
    storage: &S,
) -> Result<(), RuntimeError>
where
    S: StorageSize<ContractsRawCode> + StorageRead<ContractsRawCode> + StorageAsRef,
    <S as StorageInspect<ContractsRawCode>>::Error: Into<std::io::Error>,
{
    debug_assert_eq!(CallFrame::serialized_size(), frame_bytes.len());
    debug_assert_eq!(
        frame.code_size() + frame.code_size_padding() + frame_bytes.len() as Word,
        dst.len() as Word
    );

    dst[..frame_bytes.len()].copy_from_slice(&frame_bytes);

    let bytes_read = storage
        .storage::<ContractsRawCode>()
        .read(
            frame.to(),
            &mut dst[CallFrame::serialized_size()..][..frame.code_size() as usize],
        )
        .map_err(RuntimeError::from_io)?
        .ok_or(PanicReason::ContractNotFound)?;

    if bytes_read as Word != frame.code_size() {
        return Err(PanicReason::ContractMismatch.into());
    }

    if frame.code_size_padding() > 0 {
        let (_, rest) = dst.split_at_mut(CallFrame::serialized_size() + frame.code_size() as usize);
        rest.fill(0);
    }

    Ok(())
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
