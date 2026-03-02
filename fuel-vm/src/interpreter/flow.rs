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
        ExecutableTransaction,
        Interpreter,
        Memory,
        MemoryInstance,
        PanicContext,
        contract::{
            balance_decrease,
            balance_increase,
            contract_size,
        },
        internal::{
            current_contract,
            external_asset_id_balance_sub,
            inc_pc,
            set_frame_pointer,
        },
        receipts::ReceiptsCtx,
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
use alloc::vec::Vec;
use core::cmp;
use fuel_asm::{
    Instruction,
    PanicInstruction,
    RegId,
};
use fuel_storage::{
    StorageAsRef,
    StorageReadError,
};
use fuel_tx::{
    PanicReason,
    Receipt,
};
use fuel_types::{
    AssetId,
    Bytes32,
    ContractId,
    Word,
    bytes::padded_len_usize,
    canonical::Serialize,
};

#[cfg(test)]
mod jump_tests;
#[cfg(test)]
mod ret_tests;

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
            memory: self.memory.as_mut(),
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
    memory: &'vm mut MemoryInstance,
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

            // Clear storage preload area
            self.memory.as_mut().storage_preload_mut().clear();
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
        let call_params_pointer = self.registers[ra];
        let amount_of_coins_to_forward = self.registers[rb];
        let asset_id_pointer = self.registers[rc];
        let amount_of_gas_to_forward = self.registers[rd];

        let gas_cost = self.gas_costs().call();
        let new_storage_gas_per_byte = self.gas_costs().new_storage_per_byte();
        // Charge only for the `base` execution.
        // We will charge for the frame size in the `prepare_call`.
        self.gas_charge(gas_cost.base())?;
        let current_contract =
            current_contract(&self.context, self.registers.fp(), self.memory.as_ref())?;

        {
            let call_bytes = self.memory().read(call_params_pointer, Call::LEN)?;
            let call = Call::try_from(call_bytes)?;
            let asset_id = AssetId::new(self.memory().read_bytes(asset_id_pointer)?);

            let code_size = contract_size(&self.storage, call.to())? as usize;
            let code_size_padded =
                padded_len_usize(code_size).ok_or(PanicReason::MemoryOverflow)?;

            let total_size_in_stack = CallFrame::serialized_size()
                .checked_add(code_size_padded)
                .ok_or_else(|| Bug::new(BugVariant::CodeSizeOverflow))?;

            self.dependent_gas_charge_without_base(gas_cost, code_size_padded as Word)?;

            let amount = amount_of_coins_to_forward;
            if let Some(source_contract) = current_contract {
                balance_decrease(&mut self.storage, &source_contract, &asset_id, amount)?;
            } else {
                external_asset_id_balance_sub(
                    &mut self.balances,
                    self.memory.as_mut(),
                    &asset_id,
                    amount,
                )?;
            }

            self.verifier.check_contract_in_inputs(
                &mut self.panic_context,
                &self.input_contracts,
                call.to(),
            )?;

            // credit contract asset_id balance
            let created_new_entry = balance_increase(
                &mut self.storage,
                call.to(),
                &asset_id,
                amount_of_coins_to_forward,
            )?;

            if created_new_entry {
                // If a new entry was created, we must charge gas for it
                self.gas_charge(
                    ((Bytes32::LEN + WORD_SIZE) as u64)
                        .saturating_mul(new_storage_gas_per_byte),
                )?;
            }

            let forward_gas_amount =
                cmp::min(self.registers[RegId::CGAS], amount_of_gas_to_forward);

            // subtract gas
            self.registers[RegId::CGAS] = self.registers[RegId::CGAS]
                .checked_sub(forward_gas_amount)
                .ok_or_else(|| Bug::new(BugVariant::ContextGasUnderflow))?;

            // Construct frame
            let mut frame = CallFrame::new(
                *call.to(),
                asset_id,
                self.registers,
                code_size_padded,
                call.a(),
                call.b(),
            )
            .ok_or(PanicReason::MemoryOverflow)?;
            *frame.context_gas_mut() = self.registers[RegId::CGAS];
            *frame.global_gas_mut() = self.registers[RegId::GGAS];

            // Allocate stack memory
            let old_sp = self.registers[RegId::SP];
            let new_sp = old_sp.saturating_add(total_size_in_stack as Word);
            self.memory_mut().grow_stack(new_sp)?;
            self.registers[RegId::SP] = new_sp;
            self.registers[RegId::SSP] = new_sp;

            let id = self.internal_contract().unwrap_or_default();

            set_frame_pointer(&mut self.context, self.registers.fp_mut(), old_sp);

            // Write the frame to memory
            // Ownership checks are disabled because we just allocated the memory above.
            let dst_addr = self.registers[RegId::FP];
            let dst = self
                .memory
                .as_mut()
                .write_noownerchecks(dst_addr, total_size_in_stack)?;
            let (mem_frame, mem_code) = dst.split_at_mut(CallFrame::serialized_size());
            mem_frame.copy_from_slice(&frame.to_bytes());
            let (mem_code, mem_code_padding) = mem_code.split_at_mut(code_size);
            mem_code_padding.fill(0);

            let read_result = self
                .storage
                .storage::<ContractsRawCode>()
                .read_exact(call.to(), 0, mem_code)
                .map_err(RuntimeError::Storage)?;
            match read_result {
                Ok(read_len) => debug_assert_eq!(read_len, code_size),
                Err(StorageReadError::KeyNotFound) => {
                    return Err(RuntimeError::Recoverable(PanicReason::ContractNotFound));
                }
                Err(StorageReadError::OutOfBounds) => {
                    unreachable!("The size is checked above using contract_size")
                }
            }

            #[allow(clippy::arithmetic_side_effects)] // Checked above
            let code_start =
                (self.registers[RegId::FP]) + CallFrame::serialized_size() as Word;

            self.registers[RegId::PC] = code_start;
            self.registers[RegId::BAL] = amount_of_coins_to_forward;
            self.registers[RegId::IS] = self.registers[RegId::PC];
            self.registers[RegId::CGAS] = forward_gas_amount;
            self.registers[RegId::FLAG] = 0;

            // Clear storage preload area
            self.memory.as_mut().storage_preload_mut().clear();

            let receipt = Receipt::call(
                id,
                *call.to(),
                amount_of_coins_to_forward,
                asset_id,
                forward_gas_amount,
                call.a(),
                call.b(),
                self.registers[RegId::PC],
                self.registers[RegId::IS],
            );

            self.receipts.push(receipt)?;

            self.frames.push(frame);

            Ok(())
        }
    }
}
