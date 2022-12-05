use crate::consts::*;
use crate::error::{InterpreterError, RuntimeError};
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
                self.gas_charge(self.gas_costs.add)?;
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), c.into())?;
            }

            OpcodeRepr::ADDI => {
                self.gas_charge(self.gas_costs.addi)?;
                self.alu_capture_overflow(ra, u128::overflowing_add, b.into(), imm.into())?;
            }

            OpcodeRepr::AND => {
                self.gas_charge(self.gas_costs.and)?;
                self.alu_set(ra, b & c)?;
            }

            OpcodeRepr::ANDI => {
                self.gas_charge(self.gas_costs.andi)?;
                self.alu_set(ra, b & imm)?;
            }

            OpcodeRepr::DIV => {
                self.gas_charge(self.gas_costs.div)?;
                self.alu_error(ra, Word::div, b, c, c == 0)?;
            }

            OpcodeRepr::DIVI => {
                self.gas_charge(self.gas_costs.divi)?;
                self.alu_error(ra, Word::div, b, imm, imm == 0)?;
            }

            OpcodeRepr::EQ => {
                self.gas_charge(self.gas_costs.eq)?;
                self.alu_set(ra, (b == c) as Word)?;
            }

            OpcodeRepr::EXP => {
                self.gas_charge(self.gas_costs.exp)?;
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, c as u32)?;
            }

            OpcodeRepr::EXPI => {
                self.gas_charge(self.gas_costs.expi)?;
                self.alu_boolean_overflow(ra, Word::overflowing_pow, b, imm as u32)?;
            }

            OpcodeRepr::GT => {
                self.gas_charge(self.gas_costs.gt)?;
                self.alu_set(ra, (b > c) as Word)?;
            }

            OpcodeRepr::LT => {
                self.gas_charge(self.gas_costs.lt)?;
                self.alu_set(ra, (b < c) as Word)?;
            }

            OpcodeRepr::MLOG => {
                self.gas_charge(self.gas_costs.mlog)?;
                self.alu_error(
                    ra,
                    |b, c| checked_ilog(b, c).expect("checked_ilog returned None for valid values") as Word,
                    b,
                    c,
                    b == 0 || c <= 1,
                )?;
            }

            OpcodeRepr::MOD => {
                self.gas_charge(self.gas_costs.mod_op)?;
                self.alu_error(ra, Word::wrapping_rem, b, c, c == 0)?;
            }

            OpcodeRepr::MODI => {
                self.gas_charge(self.gas_costs.modi)?;
                self.alu_error(ra, Word::wrapping_rem, b, imm, imm == 0)?;
            }

            OpcodeRepr::MOVE => {
                self.gas_charge(self.gas_costs.move_op)?;
                self.alu_set(ra, b)?;
            }

            OpcodeRepr::MOVI => {
                self.gas_charge(self.gas_costs.movi)?;
                self.alu_set(ra, imm)?;
            }

            OpcodeRepr::MROO => {
                self.gas_charge(self.gas_costs.mroo)?;
                self.alu_error(
                    ra,
                    |b, c| checked_nth_root(b, c).expect("checked_nth_root returned None for valid values") as Word,
                    b,
                    c,
                    c == 0,
                )?;
            }

            OpcodeRepr::MUL => {
                self.gas_charge(self.gas_costs.mul)?;
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), c.into())?;
            }

            OpcodeRepr::MULI => {
                self.gas_charge(self.gas_costs.muli)?;
                self.alu_capture_overflow(ra, u128::overflowing_mul, b.into(), imm.into())?;
            }

            OpcodeRepr::NOOP => {
                self.gas_charge(self.gas_costs.noop)?;
                self.alu_clear()?;
            }

            OpcodeRepr::NOT => {
                self.gas_charge(self.gas_costs.not)?;
                self.alu_set(ra, !b)?;
            }

            OpcodeRepr::OR => {
                self.gas_charge(self.gas_costs.or)?;
                self.alu_set(ra, b | c)?;
            }

            OpcodeRepr::ORI => {
                self.gas_charge(self.gas_costs.ori)?;
                self.alu_set(ra, b | imm)?;
            }

            OpcodeRepr::SLL => {
                self.gas_charge(self.gas_costs.sll)?;
                self.alu_set(ra, b.checked_shl(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SLLI => {
                self.gas_charge(self.gas_costs.slli)?;
                self.alu_set(ra, b.checked_shl(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRL => {
                self.gas_charge(self.gas_costs.srl)?;
                self.alu_set(ra, b.checked_shr(c as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SRLI => {
                self.gas_charge(self.gas_costs.srli)?;
                self.alu_set(ra, b.checked_shr(imm as u32).unwrap_or_default())?;
            }

            OpcodeRepr::SUB => {
                self.gas_charge(self.gas_costs.sub)?;
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), c.into())?;
            }

            OpcodeRepr::SUBI => {
                self.gas_charge(self.gas_costs.subi)?;
                self.alu_capture_overflow(ra, u128::overflowing_sub, b.into(), imm.into())?;
            }

            OpcodeRepr::XOR => {
                self.gas_charge(self.gas_costs.xor)?;
                self.alu_set(ra, b ^ c)?;
            }

            OpcodeRepr::XORI => {
                self.gas_charge(self.gas_costs.xori)?;
                self.alu_set(ra, b ^ imm)?;
            }

            OpcodeRepr::JI => {
                self.gas_charge(self.gas_costs.ji)?;
                self.jump(imm)?;
            }

            OpcodeRepr::JNEI => {
                self.gas_charge(self.gas_costs.jnei)?;
                self.jump_not_equal(a, b, imm)?;
            }

            OpcodeRepr::JNZI => {
                self.gas_charge(self.gas_costs.jnzi)?;
                self.jump_not_zero(a, imm)?;
            }

            OpcodeRepr::JMP => {
                self.gas_charge(self.gas_costs.jmp)?;
                self.jump(a)?;
            }

            OpcodeRepr::JNE => {
                self.gas_charge(self.gas_costs.jne)?;
                self.jump_not_equal(a, b, c)?;
            }

            OpcodeRepr::RET => {
                self.gas_charge(self.gas_costs.ret)?;
                self.ret(a)?;

                return Ok(ExecuteState::Return(a));
            }

            OpcodeRepr::RETD => {
                self.gas_charge(self.gas_costs.retd)?;

                return self.ret_data(a, b).map(ExecuteState::ReturnData);
            }

            OpcodeRepr::RVRT => {
                self.gas_charge(self.gas_costs.rvrt)?;
                self.revert(a);

                return Ok(ExecuteState::Revert(a));
            }

            OpcodeRepr::SMO => {
                self.gas_charge(self.gas_costs.smo)?;
                self.message_output(a, b, c, d)?;
            }

            OpcodeRepr::ALOC => {
                self.gas_charge(self.gas_costs.aloc)?;
                self.malloc(a)?;
            }

            OpcodeRepr::CFEI => {
                self.gas_charge(self.gas_costs.cfei)?;
                self.stack_pointer_overflow(Word::overflowing_add, imm)?;
            }

            OpcodeRepr::CFSI => {
                self.gas_charge(self.gas_costs.cfsi)?;
                self.stack_pointer_overflow(Word::overflowing_sub, imm)?;
            }

            OpcodeRepr::LB => {
                self.gas_charge(self.gas_costs.lb)?;
                self.load_byte(ra, b, imm)?;
            }

            OpcodeRepr::LW => {
                self.gas_charge(self.gas_costs.lw)?;
                self.load_word(ra, b, imm)?;
            }

            OpcodeRepr::MCL => {
                self.dependant_gas_charge(self.gas_costs.mcl, b)?;
                self.memclear(a, b)?;
            }

            OpcodeRepr::MCLI => {
                self.dependant_gas_charge(self.gas_costs.mcli, b)?;
                self.memclear(a, imm)?;
            }

            OpcodeRepr::MCP => {
                self.dependant_gas_charge(self.gas_costs.mcp, c)?;
                self.memcopy(a, b, c)?;
            }

            OpcodeRepr::MCPI => {
                self.dependant_gas_charge(self.gas_costs.mcpi, imm)?;
                self.memcopy(a, b, imm)?;
            }

            OpcodeRepr::MEQ => {
                self.dependant_gas_charge(self.gas_costs.meq, d)?;
                self.memeq(ra, b, c, d)?;
            }

            OpcodeRepr::SB => {
                self.gas_charge(self.gas_costs.sb)?;
                self.store_byte(a, b, imm)?;
            }

            OpcodeRepr::SW => {
                self.gas_charge(self.gas_costs.sw)?;
                self.store_word(a, b, imm)?;
            }

            OpcodeRepr::BAL => {
                self.gas_charge(self.gas_costs.bal)?;
                self.contract_balance(ra, b, c)?;
            }

            OpcodeRepr::BHEI => {
                self.gas_charge(self.gas_costs.bhei)?;
                self.set_block_height(ra)?;
            }

            OpcodeRepr::BHSH => {
                self.gas_charge(self.gas_costs.bhsh)?;
                self.block_hash(a, b)?;
            }

            OpcodeRepr::BURN => {
                self.gas_charge(self.gas_costs.burn)?;
                self.burn(a)?;
            }

            OpcodeRepr::CALL => {
                self.dependant_gas_charge(self.gas_costs.call, todo!("Figure out how to get contract size here"))?;
                let state = self.call(a, b, c, d)?;
                // raise revert state to halt execution for the callee
                if let ProgramState::Revert(ra) = state {
                    return Ok(ExecuteState::Revert(ra));
                }
            }

            OpcodeRepr::CB => {
                self.gas_charge(self.gas_costs.cb)?;
                self.block_proposer(a)?;
            }

            OpcodeRepr::CCP => {
                self.dependant_gas_charge(self.gas_costs.ccp, d)?;
                self.code_copy(a, b, c, d)?;
            }

            OpcodeRepr::CROO => {
                self.gas_charge(self.gas_costs.croo)?;
                self.code_root(a, b)?;
            }

            OpcodeRepr::CSIZ => {
                self.gas_charge(self.gas_costs.csiz)?;
                self.code_size(ra, self.registers[rb])?;
            }

            OpcodeRepr::LDC => {
                self.gas_charge(self.gas_costs.ldc)?;
                self.load_contract_code(a, b, c)?;
            }

            OpcodeRepr::LOG => {
                self.gas_charge(self.gas_costs.log)?;
                self.log(a, b, c, d)?;
            }

            OpcodeRepr::LOGD => {
                self.gas_charge(self.gas_costs.logd)?;
                self.log_data(a, b, c, d)?;
            }

            OpcodeRepr::MINT => {
                self.gas_charge(self.gas_costs.mint)?;
                self.mint(a)?;
            }

            OpcodeRepr::SCWQ => {
                self.gas_charge(self.gas_costs.scwq)?;
                self.state_clear_qword(a, rb, c)?;
            }

            OpcodeRepr::SRW => {
                self.gas_charge(self.gas_costs.srw)?;
                self.state_read_word(ra, rb, c)?;
            }

            OpcodeRepr::SRWQ => {
                self.gas_charge(self.gas_costs.srwq)?;
                self.state_read_qword(a, rb, c, d)?;
            }

            OpcodeRepr::SWW => {
                self.gas_charge(self.gas_costs.sww)?;
                self.state_write_word(a, rb, c)?;
            }

            OpcodeRepr::SWWQ => {
                self.gas_charge(self.gas_costs.swwq)?;
                self.state_write_qword(a, rb, c, d)?;
            }

            OpcodeRepr::TIME => {
                self.gas_charge(self.gas_costs.time)?;
                self.timestamp(ra, b)?;
            }

            OpcodeRepr::ECR => {
                self.gas_charge(self.gas_costs.ecr)?;
                self.ecrecover(a, b, c)?;
            }

            OpcodeRepr::K256 => {
                self.gas_charge(self.gas_costs.k256)?;
                self.keccak256(a, b, c)?;
            }

            OpcodeRepr::S256 => {
                self.gas_charge(self.gas_costs.s256)?;
                self.sha256(a, b, c)?;
            }

            OpcodeRepr::FLAG => {
                self.gas_charge(self.gas_costs.flag)?;
                self.set_flag(a)?;
            }

            OpcodeRepr::GM => {
                self.gas_charge(self.gas_costs.gm)?;
                self.metadata(ra, imm as Immediate18)?;
            }

            OpcodeRepr::GTF => {
                self.gas_charge(self.gas_costs.gtf)?;
                self.get_transaction_field(ra, b, imm as Immediate12)?;
            }

            OpcodeRepr::TR => {
                self.gas_charge(self.gas_costs.tr)?;
                self.transfer(a, b, c)?;
            }

            OpcodeRepr::TRO => {
                self.gas_charge(self.gas_costs.tro)?;
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
