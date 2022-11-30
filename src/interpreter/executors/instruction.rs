use crate::consts::*;
use crate::error::{InterpreterError, RuntimeError};
use crate::interpreter::gas::consts::*;
use crate::interpreter::{ExecutableTransaction, Interpreter};
use crate::state::{ExecuteState, ProgramState};
use crate::storage::InterpreterStorage;

use fuel_asm::{Instruction, OpcodeRepr, PanicReason};
use fuel_types::{bytes, Immediate12, Immediate18, Word};

use std::ops::Div;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Execute the current instruction pair located in `$m[$pc]`.
    pub fn execute(&mut self) -> Result<ExecuteState, InterpreterError> {
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
        let state = self.instruction(hi)?;

        // TODO optimize
        // Should execute `lo` only if there is no rupture in the flow - that means
        // either a breakpoint or some instruction that would skip `lo` such as
        // `RET`, `JI` or `CALL`
        if self.registers[REG_PC] == pc && state.should_continue() {
            self.instruction(lo)
        } else {
            Ok(state)
        }
    }

    pub fn reset_heap(&mut self, ssp: Word) {
        self.registers[REG_HP] = VM_MAX_RAM - 1;
        self.registers[REG_SP] = ssp;
        self.registers[REG_SSP] = ssp;
        self.receipts.clear();
    }

    /// Execute a provided instruction
    pub fn instruction(&mut self, instruction: Instruction) -> Result<ExecuteState, InterpreterError> {
        #[cfg(feature = "debug")]
        {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into());
            }
        }

        self._instruction(instruction)
            .map_err(|e| InterpreterError::from_runtime(e, instruction))
    }

    #[tracing::instrument(name = "instruction", skip(self))]
    fn _instruction(&mut self, instruction: Instruction) -> Result<ExecuteState, RuntimeError> {
        let (op, ra, rb, rc, rd, imm) = instruction.into_inner();
        let (a, b, c, d) = (
            self.registers[ra],
            self.registers[rb],
            self.registers[rc],
            self.registers[rd],
        );

        tracing::trace!("Op code: {:?}, Registers: a {}, b, {}, c {}, d {}", op, a, b, c, d);

        // TODO additional branch that might be optimized after
        // https://github.com/FuelLabs/fuel-asm/issues/68
        if self.is_predicate() && !op.is_predicate_allowed() {
            return Err(PanicReason::TransactionValidity.into());
        }

        match op {
            OpcodeRepr::ADD => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_ADD)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_ADD);
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), c.into())?;
            }

            OpcodeRepr::ADDI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_ADDI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_ADDI);
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), imm.into())?;
            }

            OpcodeRepr::AND => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_AND)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_AND);
                self.alu_set(ra, b & c)?;
            }

            OpcodeRepr::ANDI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_ANDI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_ANDI);
                self.alu_set(ra, b & imm)?;
            }

            OpcodeRepr::DIV => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_DIV)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_DIV);
                self.alu_error(ra, Word::div, b, c, c == 0)?;
            }

            OpcodeRepr::DIVI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_DIVI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_DIVI);
                self.alu_error(ra, Word::div, b, imm, imm == 0)?;
            }

            OpcodeRepr::EQ => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_EQ)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_EQ);
                self.alu_set(ra, (b == c) as Word)?;
            }

            OpcodeRepr::EXP => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_EXP)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_EXP);
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, c as u32)?;
            }

            OpcodeRepr::EXPI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_EXPI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_EXPI);
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, imm as u32)?;
            }

            OpcodeRepr::GT => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_GT)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_GT);
                self.alu_set(ra, (b > c) as Word)?;
            }

            OpcodeRepr::LT => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_LT)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_LT);
                self.alu_set(ra, (b < c) as Word)?;
            }

            OpcodeRepr::MLOG => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MLOG)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MLOG);
                self.alu_error(
                    ra,
                    |b, c| checked_ilog(b, c).expect("checked_ilog returned None for valid values") as Word,
                    b,
                    c,
                    b == 0 || c <= 1,
                )?;
            }

            OpcodeRepr::MOD => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MOD)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MOD);
                self.alu_error(ra, Word::wrapping_rem, b, c, c == 0)?;
            }

            OpcodeRepr::MODI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MODI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MODI);
                self.alu_error(ra, Word::wrapping_rem, b, imm, imm == 0)?;
            }

            OpcodeRepr::MOVE => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MOVE)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MOVE);
                self.alu_set(ra, b)?;
            }

            OpcodeRepr::MOVI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MOVI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MOVI);
                self.alu_set(ra, imm)?;
            }

            OpcodeRepr::MROO => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MROO)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MROO);
                self.alu_error(
                    ra,
                    |b, c| checked_nth_root(b, c).expect("checked_nth_root returned None for valid values") as Word,
                    b,
                    c,
                    c == 0,
                )?;
            }

            OpcodeRepr::MUL => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MUL)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MUL);
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), c.into())?;
            }

            OpcodeRepr::MULI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MULI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MULI);
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), imm.into())?;
            }

            OpcodeRepr::NOOP => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_NOOP)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_NOOP);
                self.alu_clear()?;
            }

            OpcodeRepr::NOT => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_NOT)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_NOT);
                self.alu_set(ra, !b)?;
            }

            OpcodeRepr::OR => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_OR)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_OR);
                self.alu_set(ra, b | c)?;
            }

            OpcodeRepr::ORI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_ORI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_ORI);
                self.alu_set(ra, b | imm)?;
            }

            OpcodeRepr::SLL => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SLL)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SLL);
                self.alu_set(ra, b.checked_shl(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SLLI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SLLI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SLLI);
                self.alu_set(ra, b.checked_shl(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRL => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SRL)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SRL);
                self.alu_set(ra, b.checked_shr(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRLI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SRLI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SRLI);
                self.alu_set(ra, b.checked_shr(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SUB => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SUB)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SUB);
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), c.into())?;
            }

            OpcodeRepr::SUBI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SUBI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SUBI);
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), imm.into())?;
            }

            OpcodeRepr::XOR => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_XOR)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_XOR);
                self.alu_set(ra, b ^ c)?;
            }

            OpcodeRepr::XORI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_XORI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_XORI);
                self.alu_set(ra, b ^ imm)?;
            }

            OpcodeRepr::JI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_JI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_JI);
                self.jump(imm)?;
            }

            OpcodeRepr::JNEI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_JNEI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_JNEI);
                self.jump_not_equal(a, b, imm)?;
            }

            OpcodeRepr::JNZI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_JNZI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_JNZI);
                self.jump_not_zero(a, imm)?;
            }

            OpcodeRepr::JMP => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_JMP)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_JMP);
                self.jump(a)?;
            }

            OpcodeRepr::JNE => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_JNE)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_JNE);
                self.jump_not_equal(a, b, c)?;
            }

            OpcodeRepr::RET => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_RET)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_RET);
                self.ret(a)?;

                return Ok(ExecuteState::Return(a));
            }

            OpcodeRepr::RETD => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_RETD)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_RETD);

                return self.ret_data(a, b).map(ExecuteState::ReturnData);
            }

            OpcodeRepr::RVRT => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_RVRT)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_RVRT);
                self.revert(a);

                return Ok(ExecuteState::Revert(a));
            }

            OpcodeRepr::SMO => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SMO)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SMO);
                self.message_output(a, b, c, d)?;
            }

            OpcodeRepr::ALOC => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_ALOC)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_ALOC);
                self.malloc(a)?;
            }

            OpcodeRepr::CFEI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_CFEI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_CFEI);
                self.stack_pointer_overflow(Word::overflowing_add, imm)?;
            }

            OpcodeRepr::CFSI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_CFSI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_CFSI);
                self.stack_pointer_overflow(Word::overflowing_sub, imm)?;
            }

            OpcodeRepr::LB => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_LB)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_LB);
                self.load_byte(ra, b, imm)?;
            }

            OpcodeRepr::LW => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_LW)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_LW);
                self.load_word(ra, b, imm)?;
            }

            OpcodeRepr::MCL => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge_monad(GAS_MCL, b)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge_monad(GAS_MCL, b);
                self.memclear(a, b)?;
            }

            OpcodeRepr::MCLI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge_monad(GAS_MCLI, b)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge_monad(GAS_MCLI, b);
                self.memclear(a, imm)?;
            }

            OpcodeRepr::MCP => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge_monad(GAS_MCP, c)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge_monad(GAS_MCP, c);
                self.memcopy(a, b, c)?;
            }

            OpcodeRepr::MCPI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge_monad(GAS_MCPI, imm)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge_monad(GAS_MCPI, imm);
                self.memcopy(a, b, imm)?;
            }

            OpcodeRepr::MEQ => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge_monad(GAS_MEQ, d)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge_monad(GAS_MEQ, d);
                self.memeq(ra, b, c, d)?;
            }

            OpcodeRepr::SB => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SB)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SB);
                self.store_byte(a, b, imm)?;
            }

            OpcodeRepr::SW => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SW)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SW);
                self.store_word(a, b, imm)?;
            }

            OpcodeRepr::BAL => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_BAL)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_BAL);
                self.contract_balance(ra, b, c)?;
            }

            OpcodeRepr::BHEI => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_BHEI)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_BHEI);
                self.set_block_height(ra)?;
            }

            OpcodeRepr::BHSH => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_BHSH)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_BHSH);
                self.block_hash(a, b)?;
            }

            OpcodeRepr::BURN => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_BURN)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_BURN);
                self.burn(a)?;
            }

            OpcodeRepr::CALL => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_CALL)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_CALL);
                let state = self.call(a, b, c, d)?;
                // raise revert state to halt execution for the callee
                if let ProgramState::Revert(ra) = state {
                    return Ok(ExecuteState::Revert(ra));
                }
            }

            OpcodeRepr::CB => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_CB)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_CB);
                self.block_proposer(a)?;
            }

            OpcodeRepr::CCP => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge_monad(GAS_CCP, d)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge_monad(GAS_CCP, d);
                self.code_copy(a, b, c, d)?;
            }

            OpcodeRepr::CROO => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_CROO)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_CROO);
                self.code_root(a, b)?;
            }

            OpcodeRepr::CSIZ => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_CSIZ)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_CSIZ);
                self.code_size(ra, self.registers[rb])?;
            }

            OpcodeRepr::LDC => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_LDC)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_LDC);
                self.load_contract_code(a, b, c)?;
            }

            OpcodeRepr::LOG => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_LOG)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_LOG);
                self.log(a, b, c, d)?;
            }

            OpcodeRepr::LOGD => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_LOGD)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_LOGD);
                self.log_data(a, b, c, d)?;
            }

            OpcodeRepr::MINT => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_MINT)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_MINT);
                self.mint(a)?;
            }

            OpcodeRepr::SCWQ => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SCWQ)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SCWQ);
                self.state_clear_qword(a, rb, c)?;
            }

            OpcodeRepr::SRW => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SRW)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SRW);
                self.state_read_word(ra, rb, c)?;
            }

            OpcodeRepr::SRWQ => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SRWQ)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SRWQ);
                self.state_read_qword(a, rb, c, d)?;
            }

            OpcodeRepr::SWW => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SWW)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SWW);
                self.state_write_word(a, rb, c)?;
            }

            OpcodeRepr::SWWQ => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_SWWQ)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_SWWQ);
                self.state_write_qword(a, rb, c, d)?;
            }

            OpcodeRepr::TIME => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_TIME)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_TIME);
                self.timestamp(ra, b)?;
            }

            OpcodeRepr::ECR => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_ECR)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_ECR);
                self.ecrecover(a, b, c)?;
            }

            OpcodeRepr::K256 => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_K256)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_K256);
                self.keccak256(a, b, c)?;
            }

            OpcodeRepr::S256 => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_S256)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_S256);
                self.sha256(a, b, c)?;
            }

            OpcodeRepr::FLAG => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_FLAG)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_FLAG);
                self.set_flag(a)?;
            }

            OpcodeRepr::GM => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_GM)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_GM);
                self.metadata(ra, imm as Immediate18)?;
            }

            OpcodeRepr::GTF => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_GTF)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_GTF);
                self.get_transaction_field(ra, b, imm as Immediate12)?;
            }

            OpcodeRepr::TR => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_TR)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_TR);
                self.transfer(a, b, c)?;
            }

            OpcodeRepr::TRO => {
                #[cfg(not(feature = "ignore_gas"))]
                self.gas_charge(GAS_TRO)?;
                #[cfg(feature = "ignore_gas")]
                let _ = self.gas_charge(GAS_TRO);
                self.transfer_output(a, b, c, d)?;
            }

            // list of currently unimplemented opcodes
            _ => {
                return Err(PanicReason::ErrorFlag.into());
            }
        }

        Ok(ExecuteState::Proceed)
    }
}

/// Computes nth root of target, rounding down to nearest integer.
/// This function uses the floating point operation to get an approximate solution,
/// but corrects the result using exponentation to check for inaccuracy.
fn checked_nth_root(target: u64, nth_root: u64) -> Option<u64> {
    if nth_root == 0 {
        // Zeroth root is not defined
        return None;
    }

    if nth_root == 1 || target <= 1 {
        // Corner cases
        return Some(target);
    }

    if nth_root >= target || nth_root > 64 {
        // For any root >= target, result always 1
        // For any n>1, n**64 can never fit into u64
        return Some(1);
    }

    let nth_root = nth_root as u32; // Never loses bits, checked above

    // Use floating point operation to get an approximation for the starting point.
    // This is at most off by one in either direction.
    let guess = (target as f64).powf((nth_root as f64).recip()) as u64;

    debug_assert!(guess != 0, "This should never occur for {{target, n}} > 1");

    // Check if a value raised to nth_power is below the target value, handling overflow correctly
    let is_nth_power_below_target = |v: u64| match v.checked_pow(nth_root) {
        Some(pow) => target < pow,
        None => true, // v**nth_root >= 2**64 and target < 2**64
    };

    // Compute guess**n to check if the guess is too large.
    // Note that if guess == 1, then g1 == 1 as well, meaning that we will not return here.
    if is_nth_power_below_target(guess) {
        return Some(guess - 1);
    }

    // Check if the initial guess was correct
    if is_nth_power_below_target(guess + 1) {
        return Some(guess);
    }

    // Check if the guess was correct
    Some(guess + 1)
}

/// Computes logarithm for given exponent and base.
/// Diverges when exp == 0 or base <= 1.
///
/// This code is originally from [rust corelib][rust-corelib-impl],
/// but with all additional clutter removed.
///
/// [rust-corelib-impl]: https://github.com/rust-lang/rust/blob/415d8fcc3e17f8c1324a81cf2aa7127b4fcfa32e/library/core/src/num/uint_macros.rs#L774
#[inline(always)] // Force copy of each invocation for optimization, see comments below
const fn _unchecked_ilog_inner(exp: Word, base: Word) -> u32 {
    let mut n = 0;
    let mut r = exp;
    while r >= base {
        r /= base;
        n += 1;
    }

    n
}

/// Logarithm for given exponent and an arbitrary base, rounded
/// rounded down to nearest integer value.
///
/// Returns `None` if the exponent == 0, or if the base <= 1.
///
/// TODO: when <https://github.com/rust-lang/rust/issues/70887> is stabilized,
/// consider using that instead.
const fn checked_ilog(exp: Word, base: Word) -> Option<u32> {
    if exp == 0 || base <= 1 {
        return None;
    }

    // Generate separately optimized paths for some common and/or expensive bases.
    // See <https://github.com/FuelLabs/fuel-vm/issues/150#issuecomment-1288797787> for benchmark.
    Some(match base {
        2 => _unchecked_ilog_inner(exp, 2),
        3 => _unchecked_ilog_inner(exp, 3),
        4 => _unchecked_ilog_inner(exp, 4),
        5 => _unchecked_ilog_inner(exp, 5),
        10 => _unchecked_ilog_inner(exp, 10),
        n => _unchecked_ilog_inner(exp, n),
    })
}

#[cfg(test)]
mod tests;
