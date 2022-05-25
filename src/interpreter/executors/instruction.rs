use crate::consts::*;
use crate::error::{InterpreterError, RuntimeError};
use crate::interpreter::gas::consts::*;
use crate::interpreter::Interpreter;
use crate::state::{ExecuteState, ProgramState};
use crate::storage::InterpreterStorage;

use fuel_asm::{Instruction, OpcodeRepr, PanicReason};
use fuel_types::{bytes, Immediate18, Word};

use std::mem;
use std::ops::Div;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S> {
    /// Execute the current instruction pair located in `$m[$pc]`.
    pub fn execute<F>(&mut self, instruction_executor: F) -> Result<ExecuteState, InterpreterError>
    where
        F: Fn(&mut Self, Instruction) -> Result<ExecuteState, RuntimeError>,
    {
        // Safety: `chunks_exact` is guaranteed to return a well-formed slice
        let (hi, lo) = self.memory[self.registers[REG_PC] as usize..]
            .chunks_exact(WORD_SIZE)
            .next()
            .map(|b| unsafe { bytes::from_slice_unchecked(b) })
            .map(Word::from_be_bytes)
            .map(Instruction::parse_word)
            .ok_or(InterpreterError::Panic(PanicReason::MemoryOverflow))?;

        // Store the expected `$pc` after executing `hi`
        let pc = self.registers[REG_PC] + Instruction::LEN as Word;
        let state = self.instruction(&instruction_executor, hi)?;

        // TODO optimize
        // Should execute `lo` only if there is no rupture in the flow - that means
        // either a breakpoint or some instruction that would skip `lo` such as
        // `RET`, `JI` or `CALL`
        if self.registers[REG_PC] == pc && state.should_continue() {
            self.instruction(&instruction_executor, lo)
        } else {
            Ok(state)
        }
    }

    /// Execute a provided instruction
    pub fn instruction<F>(
        &mut self,
        instruction_executor: F,
        instruction: Instruction,
    ) -> Result<ExecuteState, InterpreterError>
    where
        F: FnOnce(&mut Self, Instruction) -> Result<ExecuteState, RuntimeError>,
    {
        #[cfg(feature = "debug")]
        {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into());
            }
        }

        instruction_executor(self, instruction).map_err(|e| InterpreterError::from_runtime(e, instruction))
    }

    /// executes valid predicate instructions
    pub fn instruction_predicate(&mut self, instruction: Instruction) -> Result<ExecuteState, RuntimeError> {
        let (op, ra, rb, rc, rd, imm) = instruction.into_inner();
        let (a, b, c, d) = (
            self.registers[ra],
            self.registers[rb],
            self.registers[rc],
            self.registers[rd],
        );

        match op {
            OpcodeRepr::ADD => {
                self.gas_charge(GAS_ADD)?;
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), c.into())?;
            }

            OpcodeRepr::ADDI => {
                self.gas_charge(GAS_ADDI)?;
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), imm.into())?;
            }

            OpcodeRepr::AND => {
                self.gas_charge(GAS_AND)?;
                self.alu_set(ra, b & c)?;
            }

            OpcodeRepr::ANDI => {
                self.gas_charge(GAS_ANDI)?;
                self.alu_set(ra, b & imm)?;
            }

            OpcodeRepr::DIV => {
                self.gas_charge(GAS_DIV)?;
                self.alu_error(ra, Word::div, b, c, c == 0)?;
            }

            OpcodeRepr::DIVI => {
                self.gas_charge(GAS_DIVI)?;
                self.alu_error(ra, Word::div, b, imm, imm == 0)?;
            }

            OpcodeRepr::EQ => {
                self.gas_charge(GAS_EQ)?;
                self.alu_set(ra, (b == c) as Word)?;
            }

            OpcodeRepr::EXP => {
                self.gas_charge(GAS_EXP)?;
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, c as u32)?;
            }

            OpcodeRepr::EXPI => {
                self.gas_charge(GAS_EXPI)?;
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, imm as u32)?;
            }

            OpcodeRepr::GT => {
                self.gas_charge(GAS_GT)?;
                self.alu_set(ra, (b > c) as Word)?;
            }

            OpcodeRepr::LT => {
                self.gas_charge(GAS_LT)?;
                self.alu_set(ra, (b < c) as Word)?;
            }

            OpcodeRepr::MLOG => {
                self.gas_charge(GAS_MLOG)?;
                self.alu_error(
                    ra,
                    |b, c| (b as f64).log(c as f64).trunc() as Word,
                    b,
                    c,
                    b == 0 || c <= 1,
                )?;
            }

            OpcodeRepr::MOD => {
                self.gas_charge(GAS_MOD)?;
                self.alu_error(ra, Word::wrapping_rem, b, c, c == 0)?;
            }

            OpcodeRepr::MODI => {
                self.gas_charge(GAS_MODI)?;
                self.alu_error(ra, Word::wrapping_rem, b, imm, imm == 0)?;
            }

            OpcodeRepr::MOVE => {
                self.gas_charge(GAS_MOVE)?;
                self.alu_set(ra, b)?;
            }

            OpcodeRepr::MOVI => {
                self.gas_charge(GAS_MOVI)?;
                self.alu_set(ra, imm)?;
            }

            OpcodeRepr::MROO => {
                self.gas_charge(GAS_MROO)?;
                self.alu_error(
                    ra,
                    |b, c| (b as f64).powf((c as f64).recip()).trunc() as Word,
                    b,
                    c,
                    c == 0,
                )?;
            }

            OpcodeRepr::MUL => {
                self.gas_charge(GAS_MUL)?;
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), c.into())?;
            }

            OpcodeRepr::MULI => {
                self.gas_charge(GAS_MULI)?;
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), imm.into())?;
            }

            OpcodeRepr::NOOP => {
                self.gas_charge(GAS_NOOP)?;
                self.alu_clear()?;
            }

            OpcodeRepr::NOT => {
                self.gas_charge(GAS_NOT)?;
                self.alu_set(ra, !b)?;
            }

            OpcodeRepr::OR => {
                self.gas_charge(GAS_OR)?;
                self.alu_set(ra, b | c)?;
            }

            OpcodeRepr::ORI => {
                self.gas_charge(GAS_ORI)?;
                self.alu_set(ra, b | imm)?;
            }

            OpcodeRepr::SLL => {
                self.gas_charge(GAS_SLL)?;
                self.alu_set(ra, b.checked_shl(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SLLI => {
                self.gas_charge(GAS_SLLI)?;
                self.alu_set(ra, b.checked_shl(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRL => {
                self.gas_charge(GAS_SRL)?;
                self.alu_set(ra, b.checked_shr(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRLI => {
                self.gas_charge(GAS_SRLI)?;
                self.alu_set(ra, b.checked_shr(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SUB => {
                self.gas_charge(GAS_SUB)?;
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), c.into())?;
            }

            OpcodeRepr::SUBI => {
                self.gas_charge(GAS_SUBI)?;
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), imm.into())?;
            }

            OpcodeRepr::XOR => {
                self.gas_charge(GAS_XOR)?;
                self.alu_set(ra, b ^ c)?;
            }

            OpcodeRepr::XORI => {
                self.gas_charge(GAS_XORI)?;
                self.alu_set(ra, b ^ imm)?;
            }

            OpcodeRepr::CIMV => {
                self.gas_charge(GAS_CIMV)?;
                self.check_input_maturity(ra, b, c)?;
            }

            OpcodeRepr::CTMV => {
                self.gas_charge(GAS_CTMV)?;
                self.check_tx_maturity(ra, b)?;
            }

            OpcodeRepr::JI => {
                self.gas_charge(GAS_JI)?;
                self.jump(imm)?;
            }

            OpcodeRepr::JNEI => {
                self.gas_charge(GAS_JNEI)?;
                self.jump_not_equal_imm(a, b, imm)?;
            }

            OpcodeRepr::JNZI => {
                self.gas_charge(GAS_JNZI)?;
                self.jump_not_zero_imm(a, imm)?;
            }

            OpcodeRepr::RET => {
                self.gas_charge(GAS_RET)?;
                self.ret(a)?;

                return Ok(ExecuteState::Return(a));
            }

            OpcodeRepr::ALOC => {
                self.gas_charge(GAS_ALOC)?;
                self.malloc(a)?;
            }

            OpcodeRepr::CFEI => {
                self.gas_charge(GAS_CFEI)?;
                self.stack_pointer_overflow(Word::overflowing_add, imm)?;
            }

            OpcodeRepr::CFSI => {
                self.gas_charge(GAS_CFSI)?;
                self.stack_pointer_overflow(Word::overflowing_sub, imm)?;
            }

            OpcodeRepr::LB => {
                self.gas_charge(GAS_LB)?;
                self.load_byte(ra, rb, imm)?;
            }

            OpcodeRepr::LW => {
                self.gas_charge(GAS_LW)?;
                self.load_word(ra, b, imm)?;
            }

            OpcodeRepr::MCL => {
                self.gas_charge_monad(GAS_MCL, b)?;
                self.memclear(a, b)?;
            }

            OpcodeRepr::MCLI => {
                self.gas_charge_monad(GAS_MCLI, b)?;
                self.memclear(a, imm)?;
            }

            OpcodeRepr::MCP => {
                self.gas_charge_monad(GAS_MCP, c)?;
                self.memcopy(a, b, c)?;
            }

            OpcodeRepr::MCPI => {
                self.gas_charge_monad(GAS_MCPI, imm)?;
                self.memcopy(a, b, imm)?;
            }

            OpcodeRepr::MEQ => {
                self.gas_charge(GAS_MEQ)?;
                self.memeq(ra, b, c, d)?;
            }

            OpcodeRepr::SB => {
                self.gas_charge(GAS_SB)?;
                self.store_byte(a, b, imm)?;
            }

            OpcodeRepr::SW => {
                self.gas_charge(GAS_SW)?;
                self.store_word(a, b, imm)?;
            }

            OpcodeRepr::ECR => {
                self.gas_charge(GAS_ECR)?;
                self.ecrecover(a, b, c)?;
            }

            OpcodeRepr::K256 => {
                self.gas_charge(GAS_K256)?;
                self.keccak256(a, b, c)?;
            }

            OpcodeRepr::S256 => {
                self.gas_charge(GAS_S256)?;
                self.sha256(a, b, c)?;
            }

            OpcodeRepr::XIL => {
                self.gas_charge(GAS_XIL)?;
                self.transaction_input_length(ra, b)?;
            }

            OpcodeRepr::XIS => {
                self.gas_charge(GAS_XIS)?;
                self.transaction_input_start(ra, b)?;
            }

            OpcodeRepr::XOL => {
                self.gas_charge(GAS_XOL)?;
                self.transaction_output_length(ra, b)?;
            }

            OpcodeRepr::XOS => {
                self.gas_charge(GAS_XOS)?;
                self.transaction_output_start(ra, b)?;
            }

            OpcodeRepr::XWL => {
                self.gas_charge(GAS_XWL)?;
                self.transaction_witness_length(ra, b)?;
            }

            OpcodeRepr::XWS => {
                self.gas_charge(GAS_XWS)?;
                self.transaction_witness_start(ra, b)?;
            }

            OpcodeRepr::FLAG => {
                self.gas_charge(GAS_FLAG)?;
                self.set_flag(a)?;
            }

            OpcodeRepr::GM => {
                self.gas_charge(GAS_GM)?;
                self.metadata(ra, imm as Immediate18)?;
            }

            _ => {
                // TODO use dedicated panic reason variant
                // https://github.com/FuelLabs/fuel-asm/issues/69
                return Err(PanicReason::TransactionValidity.into());
            }
        }

        Ok(ExecuteState::Proceed)
    }
}

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    /// executes stateful script instructions
    pub fn instruction_script(&mut self, instruction: Instruction) -> Result<ExecuteState, RuntimeError> {
        let (op, ra, rb, rc, rd, imm) = instruction.into_inner();
        let (a, b, c, d) = (
            self.registers[ra],
            self.registers[rb],
            self.registers[rc],
            self.registers[rd],
        );

        match op {
            OpcodeRepr::ADD => {
                self.gas_charge(GAS_ADD)?;
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), c.into())?;
            }

            OpcodeRepr::ADDI => {
                self.gas_charge(GAS_ADDI)?;
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), imm.into())?;
            }

            OpcodeRepr::AND => {
                self.gas_charge(GAS_AND)?;
                self.alu_set(ra, b & c)?;
            }

            OpcodeRepr::ANDI => {
                self.gas_charge(GAS_ANDI)?;
                self.alu_set(ra, b & imm)?;
            }

            OpcodeRepr::DIV => {
                self.gas_charge(GAS_DIV)?;
                self.alu_error(ra, Word::div, b, c, c == 0)?;
            }

            OpcodeRepr::DIVI => {
                self.gas_charge(GAS_DIVI)?;
                self.alu_error(ra, Word::div, b, imm, imm == 0)?;
            }

            OpcodeRepr::EQ => {
                self.gas_charge(GAS_EQ)?;
                self.alu_set(ra, (b == c) as Word)?;
            }

            OpcodeRepr::EXP => {
                self.gas_charge(GAS_EXP)?;
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, c as u32)?;
            }

            OpcodeRepr::EXPI => {
                self.gas_charge(GAS_EXPI)?;
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, imm as u32)?;
            }

            OpcodeRepr::GT => {
                self.gas_charge(GAS_GT)?;
                self.alu_set(ra, (b > c) as Word)?;
            }

            OpcodeRepr::LT => {
                self.gas_charge(GAS_LT)?;
                self.alu_set(ra, (b < c) as Word)?;
            }

            OpcodeRepr::MLOG => {
                self.gas_charge(GAS_MLOG)?;
                self.alu_error(
                    ra,
                    |b, c| (b as f64).log(c as f64).trunc() as Word,
                    b,
                    c,
                    b == 0 || c <= 1,
                )?;
            }

            OpcodeRepr::MOD => {
                self.gas_charge(GAS_MOD)?;
                self.alu_error(ra, Word::wrapping_rem, b, c, c == 0)?;
            }

            OpcodeRepr::MODI => {
                self.gas_charge(GAS_MODI)?;
                self.alu_error(ra, Word::wrapping_rem, b, imm, imm == 0)?;
            }

            OpcodeRepr::MOVE => {
                self.gas_charge(GAS_MOVE)?;
                self.alu_set(ra, b)?;
            }

            OpcodeRepr::MOVI => {
                self.gas_charge(GAS_MOVI)?;
                self.alu_set(ra, imm)?;
            }

            OpcodeRepr::MROO => {
                self.gas_charge(GAS_MROO)?;
                self.alu_error(
                    ra,
                    |b, c| (b as f64).powf((c as f64).recip()).trunc() as Word,
                    b,
                    c,
                    c == 0,
                )?;
            }

            OpcodeRepr::MUL => {
                self.gas_charge(GAS_MUL)?;
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), c.into())?;
            }

            OpcodeRepr::MULI => {
                self.gas_charge(GAS_MULI)?;
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), imm.into())?;
            }

            OpcodeRepr::NOOP => {
                self.gas_charge(GAS_NOOP)?;
                self.alu_clear()?;
            }

            OpcodeRepr::NOT => {
                self.gas_charge(GAS_NOT)?;
                self.alu_set(ra, !b)?;
            }

            OpcodeRepr::OR => {
                self.gas_charge(GAS_OR)?;
                self.alu_set(ra, b | c)?;
            }

            OpcodeRepr::ORI => {
                self.gas_charge(GAS_ORI)?;
                self.alu_set(ra, b | imm)?;
            }

            OpcodeRepr::SLL => {
                self.gas_charge(GAS_SLL)?;
                self.alu_set(ra, b.checked_shl(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SLLI => {
                self.gas_charge(GAS_SLLI)?;
                self.alu_set(ra, b.checked_shl(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRL => {
                self.gas_charge(GAS_SRL)?;
                self.alu_set(ra, b.checked_shr(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRLI => {
                self.gas_charge(GAS_SRLI)?;
                self.alu_set(ra, b.checked_shr(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SUB => {
                self.gas_charge(GAS_SUB)?;
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), c.into())?;
            }

            OpcodeRepr::SUBI => {
                self.gas_charge(GAS_SUBI)?;
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), imm.into())?;
            }

            OpcodeRepr::XOR => {
                self.gas_charge(GAS_XOR)?;
                self.alu_set(ra, b ^ c)?;
            }

            OpcodeRepr::XORI => {
                self.gas_charge(GAS_XORI)?;
                self.alu_set(ra, b ^ imm)?;
            }

            OpcodeRepr::CIMV => {
                self.gas_charge(GAS_CIMV)?;
                self.check_input_maturity(ra, b, c)?;
            }

            OpcodeRepr::CTMV => {
                self.gas_charge(GAS_CTMV)?;
                self.check_tx_maturity(ra, b)?;
            }

            OpcodeRepr::JI => {
                self.gas_charge(GAS_JI)?;
                self.jump(imm)?;
            }

            OpcodeRepr::JNEI => {
                self.gas_charge(GAS_JNEI)?;
                self.jump_not_equal_imm(a, b, imm)?;
            }

            OpcodeRepr::JNZI => {
                self.gas_charge(GAS_JNZI)?;
                self.jump_not_zero_imm(a, imm)?;
            }

            OpcodeRepr::RET => {
                self.gas_charge(GAS_RET)?;
                self.ret(a)?;

                return Ok(ExecuteState::Return(a));
            }

            OpcodeRepr::RETD => {
                self.gas_charge(GAS_RETD)?;

                return self.ret_data(a, b).map(ExecuteState::ReturnData);
            }

            OpcodeRepr::RVRT => {
                self.gas_charge(GAS_RVRT)?;
                self.revert(a);

                return Ok(ExecuteState::Revert(a));
            }

            OpcodeRepr::ALOC => {
                self.gas_charge(GAS_ALOC)?;
                self.malloc(a)?;
            }

            OpcodeRepr::CFEI => {
                self.gas_charge(GAS_CFEI)?;
                self.stack_pointer_overflow(Word::overflowing_add, imm)?;
            }

            OpcodeRepr::CFSI => {
                self.gas_charge(GAS_CFSI)?;
                self.stack_pointer_overflow(Word::overflowing_sub, imm)?;
            }

            OpcodeRepr::LB => {
                self.gas_charge(GAS_LB)?;
                self.load_byte(ra, rb, imm)?;
            }

            OpcodeRepr::LW => {
                self.gas_charge(GAS_LW)?;
                self.load_word(ra, b, imm)?;
            }

            OpcodeRepr::MCL => {
                self.gas_charge_monad(GAS_MCL, b)?;
                self.memclear(a, b)?;
            }

            OpcodeRepr::MCLI => {
                self.gas_charge_monad(GAS_MCLI, b)?;
                self.memclear(a, imm)?;
            }

            OpcodeRepr::MCP => {
                self.gas_charge_monad(GAS_MCP, c)?;
                self.memcopy(a, b, c)?;
            }

            OpcodeRepr::MCPI => {
                self.gas_charge_monad(GAS_MCPI, imm)?;
                self.memcopy(a, b, imm)?;
            }

            OpcodeRepr::MEQ => {
                self.gas_charge(GAS_MEQ)?;
                self.memeq(ra, b, c, d)?;
            }

            OpcodeRepr::SB => {
                self.gas_charge(GAS_SB)?;
                self.store_byte(a, b, imm)?;
            }

            OpcodeRepr::SW => {
                self.gas_charge(GAS_SW)?;
                self.store_word(a, b, imm)?;
            }

            OpcodeRepr::BAL => {
                self.gas_charge(GAS_BAL)?;
                self.contract_balance(ra, b, c)?;
            }

            OpcodeRepr::BHEI => {
                self.gas_charge(GAS_BHEI)?;
                self.alu_set(ra, self.block_height() as Word)?;
            }

            OpcodeRepr::BHSH => {
                self.gas_charge(GAS_BHSH)?;
                self.block_hash(a, b)?;
            }

            OpcodeRepr::BURN => {
                self.gas_charge(GAS_BURN)?;
                self.burn(a)?;
            }

            OpcodeRepr::CALL => {
                self.gas_charge(GAS_CALL)?;
                let state = self.call(a, b, c, d)?;
                // raise revert state to halt execution for the callee
                if let ProgramState::Revert(ra) = state {
                    return Ok(ExecuteState::Revert(ra));
                }
            }

            OpcodeRepr::CB => {
                self.gas_charge(GAS_CB)?;
                self.block_proposer(a)?;
            }

            OpcodeRepr::CCP => {
                self.gas_charge(GAS_CCP)?;
                self.code_copy(a, b, c, d)?;
            }

            OpcodeRepr::CROO => {
                self.gas_charge(GAS_CROO)?;
                self.code_root(a, b)?;
            }

            OpcodeRepr::CSIZ => {
                self.gas_charge(GAS_CSIZ)?;
                self.code_size(ra, self.registers[rb])?;
            }

            OpcodeRepr::LDC => {
                self.gas_charge(GAS_LDC)?;
                self.load_contract_code(a, b, c)?;
            }

            OpcodeRepr::LOG => {
                self.gas_charge(GAS_LOG)?;
                self.log(a, b, c, d)?;
            }

            OpcodeRepr::LOGD => {
                self.gas_charge(GAS_LOGD)?;
                self.log_data(a, b, c, d)?;
            }

            OpcodeRepr::MINT => {
                self.gas_charge(GAS_MINT)?;
                self.mint(a)?;
            }

            OpcodeRepr::SRW => {
                self.gas_charge(GAS_SRW)?;
                self.state_read_word(ra, b)?;
            }

            OpcodeRepr::SRWQ => {
                self.gas_charge(GAS_SRWQ)?;
                self.state_read_qword(a, b)?;
            }

            OpcodeRepr::SWW => {
                self.gas_charge(GAS_SWW)?;
                self.state_write_word(a, b)?;
            }

            OpcodeRepr::SWWQ => {
                self.gas_charge(GAS_SWWQ)?;
                self.state_write_qword(a, b)?;
            }

            OpcodeRepr::ECR => {
                self.gas_charge(GAS_ECR)?;
                self.ecrecover(a, b, c)?;
            }

            OpcodeRepr::K256 => {
                self.gas_charge(GAS_K256)?;
                self.keccak256(a, b, c)?;
            }

            OpcodeRepr::S256 => {
                self.gas_charge(GAS_S256)?;
                self.sha256(a, b, c)?;
            }

            OpcodeRepr::XIL => {
                self.gas_charge(GAS_XIL)?;
                self.transaction_input_length(ra, b)?;
            }

            OpcodeRepr::XIS => {
                self.gas_charge(GAS_XIS)?;
                self.transaction_input_start(ra, b)?;
            }

            OpcodeRepr::XOL => {
                self.gas_charge(GAS_XOL)?;
                self.transaction_output_length(ra, b)?;
            }

            OpcodeRepr::XOS => {
                self.gas_charge(GAS_XOS)?;
                self.transaction_output_start(ra, b)?;
            }

            OpcodeRepr::XWL => {
                self.gas_charge(GAS_XWL)?;
                self.transaction_witness_length(ra, b)?;
            }

            OpcodeRepr::XWS => {
                self.gas_charge(GAS_XWS)?;
                self.transaction_witness_start(ra, b)?;
            }

            OpcodeRepr::FLAG => {
                self.gas_charge(GAS_FLAG)?;
                self.set_flag(a)?;
            }

            OpcodeRepr::GM => {
                self.gas_charge(GAS_GM)?;
                self.metadata(ra, imm as Immediate18)?;
            }

            OpcodeRepr::TR => {
                self.gas_charge(GAS_TR)?;
                self.transfer(a, b, c)?;
            }

            OpcodeRepr::TRO => {
                self.gas_charge(GAS_TRO)?;
                self.transfer_output(a, b, c, d)?;
            }

            // list of currently unimplemented opcodes
            OpcodeRepr::SLDC | _ => {
                return Err(PanicReason::ErrorFlag.into());
            }
        }

        Ok(ExecuteState::Proceed)
    }
}
