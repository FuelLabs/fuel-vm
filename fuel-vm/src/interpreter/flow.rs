use super::{ExecutableTransaction, Interpreter};
use crate::arith;
use crate::call::Call;
use crate::consts::*;
use crate::error::RuntimeError;
use crate::interpreter::PanicContext;
use crate::state::ProgramState;
use crate::storage::InterpreterStorage;

use fuel_asm::{Instruction, InstructionResult, RegId};
use fuel_crypto::Hasher;
use fuel_tx::{PanicReason, Receipt};
use fuel_types::bytes::SerializableVec;
use fuel_types::{AssetId, Bytes32, Word};

use std::{cmp, io};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn jump(&mut self, j: Word) -> Result<(), RuntimeError> {
        let j = self.registers[RegId::IS].saturating_add(j.saturating_mul(Instruction::SIZE as Word));

        if j > VM_MAX_RAM - 1 {
            Err(PanicReason::MemoryOverflow.into())
        } else if self.is_predicate() && j <= self.registers[RegId::PC] {
            Err(PanicReason::IllegalJump.into())
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
        let (ax, overflow) = a.overflowing_add(32);
        let (cx, of) = c.overflowing_add(32);
        let overflow = overflow || of;

        if overflow || ax > VM_MAX_RAM || cx > VM_MAX_RAM {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let call = Call::try_from(&self.memory[a as usize..])?;
        let asset_id =
            AssetId::try_from(&self.memory[c as usize..cx as usize]).expect("Unreachable! Checked memory range");

        let mut frame = self.call_frame(call, asset_id)?;

        self.dependent_gas_charge(self.gas_costs.call, frame.total_code_size())?;

        if self.is_external_context() {
            self.external_asset_id_balance_sub(&asset_id, b)?;
        } else {
            let source_contract = *self.internal_contract()?;
            self.balance_decrease(&source_contract, &asset_id, b)?;
        }

        if !self
            .transaction()
            .input_contracts()
            .any(|contract| call.to() == contract)
        {
            self.panic_context = PanicContext::ContractId(*call.to());
            return Err(PanicReason::ContractNotInInputs.into());
        }

        // credit contract asset_id balance
        self.balance_increase(call.to(), &asset_id, b)?;

        let forward_gas_amount = cmp::min(self.registers[RegId::CGAS], d);

        // subtract gas
        self.registers[RegId::CGAS] = arith::sub_word(self.registers[RegId::CGAS], forward_gas_amount)?;

        *frame.set_context_gas() = self.registers[RegId::CGAS];
        *frame.set_global_gas() = self.registers[RegId::GGAS];

        let frame_bytes = frame.to_bytes();
        let len = arith::add_word(frame_bytes.len() as Word, frame.total_code_size())?;

        if len > self.registers[RegId::HP] || self.registers[RegId::SP] > self.registers[RegId::HP] - len {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let id = self.internal_contract_or_default();

        self.set_frame_pointer(self.registers[RegId::SP]);

        self.registers[RegId::SP] += len;
        self.registers[RegId::SSP] = self.registers[RegId::SP];

        let fpx = arith::add_word(self.registers[RegId::FP], frame_bytes.len() as Word)?;
        self.memory[self.registers[RegId::FP] as usize..fpx as usize].copy_from_slice(frame_bytes.as_slice());

        let code_range = (fpx as usize)..arith::add_usize(fpx as usize, frame.code_size() as usize);
        let bytes_read = self
            .storage
            .read(frame.to(), &mut self.memory[code_range.clone()])
            .map_err(RuntimeError::from_io)?
            .ok_or(PanicReason::ContractNotFound)?;
        if bytes_read as Word != frame.code_size() {
            return Err(PanicReason::ContractNotFound.into());
        }
        let pad_len = frame.code_size_padding();
        if pad_len > 0 {
            self.memory[code_range.end..code_range.end + pad_len as usize]
                .copy_from_slice(&[0; 32][..pad_len as usize]);
        }

        self.registers[RegId::BAL] = b;
        self.registers[RegId::PC] = fpx;
        self.registers[RegId::IS] = self.registers[RegId::PC];
        self.registers[RegId::CGAS] = forward_gas_amount;

        let receipt = Receipt::call(
            id,
            *frame.to(),
            b,
            *frame.asset_id(),
            d,
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

        self._prepare_call(a, b, c, d)
    }

    pub(crate) fn call(&mut self, a: Word, b: Word, c: Word, d: Word) -> Result<ProgramState, RuntimeError> {
        self._prepare_call(a, b, c, d)?;
        self.run_call()
    }
}
