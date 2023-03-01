use crate::consts::*;
use crate::error::{InterpreterError, RuntimeError};
use crate::interpreter::{alu, ExecutableTransaction, Interpreter};
use crate::state::{ExecuteState, ProgramState};
use crate::storage::InterpreterStorage;

use fuel_asm::{Instruction, PanicReason, RawInstruction, RegId};
use fuel_types::{bytes, Word};

use std::ops::Div;

impl<S, Tx> Interpreter<S, Tx>
where
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
{
    /// Execute the current instruction pair located in `$m[$pc]`.
    pub fn execute(&mut self) -> Result<ExecuteState, InterpreterError> {
        // Safety: `chunks_exact` is guaranteed to return a well-formed slice
        let [hi, lo] = self.memory[self.registers[RegId::PC] as usize..]
            .chunks_exact(WORD_SIZE)
            .next()
            .map(|b| unsafe { bytes::from_slice_unchecked(b) })
            .map(Word::from_be_bytes)
            .map(fuel_asm::raw_instructions_from_word)
            .ok_or(InterpreterError::Panic(PanicReason::MemoryOverflow))?;

        // Store the expected `$pc` after executing `hi`
        let pc = self.registers[RegId::PC] + Instruction::SIZE as Word;
        let state = self.instruction(hi)?;

        // TODO optimize
        // Should execute `lo` only if there is no rupture in the flow - that means
        // either a breakpoint or some instruction that would skip `lo` such as
        // `RET`, `JI` or `CALL`
        if self.registers[RegId::PC] == pc && state.should_continue() {
            self.instruction(lo)
        } else {
            Ok(state)
        }
    }

    /// Execute a provided instruction
    pub fn instruction<R: Into<RawInstruction> + Copy>(&mut self, raw: R) -> Result<ExecuteState, InterpreterError> {
        #[cfg(feature = "debug")]
        {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into());
            }
        }

        self._instruction(raw.into())
            .map_err(|e| InterpreterError::from_runtime(e, raw.into()))
    }

    #[tracing::instrument(name = "instruction", skip(self))]
    fn _instruction(&mut self, raw: RawInstruction) -> Result<ExecuteState, RuntimeError> {
        let instruction = Instruction::try_from(raw).map_err(|_| RuntimeError::from(PanicReason::ErrorFlag))?;

        tracing::trace!("Instruction: {:?}", instruction);

        // TODO additional branch that might be optimized after
        // https://github.com/FuelLabs/fuel-asm/issues/68
        if self.is_predicate() && !instruction.opcode().is_predicate_allowed() {
            return Err(PanicReason::TransactionValidity.into());
        }

        // Short-hand for retrieving the value from the register with the given ID.
        // We use a macro to "close over" `self.registers` without taking ownership of it.
        macro_rules! r {
            ($id:expr) => {
                self.registers[$id]
            };
        }

        match instruction {
            Instruction::ADD(add) => {
                self.gas_charge(self.gas_costs.add)?;
                let (a, b, c) = add.unpack();
                self.alu_capture_overflow(a.into(), u128::overflowing_add, r!(b).into(), r!(c).into())?;
            }

            Instruction::ADDI(addi) => {
                self.gas_charge(self.gas_costs.addi)?;
                let (a, b, imm) = addi.unpack();
                self.alu_capture_overflow(a.into(), u128::overflowing_add, r!(b).into(), imm.into())?;
            }

            Instruction::AND(and) => {
                self.gas_charge(self.gas_costs.and)?;
                let (a, b, c) = and.unpack();
                self.alu_set(a.into(), r!(b) & r!(c))?;
            }

            Instruction::ANDI(andi) => {
                self.gas_charge(self.gas_costs.andi)?;
                let (a, b, imm) = andi.unpack();
                self.alu_set(a.into(), r!(b) & Word::from(imm))?;
            }

            Instruction::DIV(div) => {
                self.gas_charge(self.gas_costs.div)?;
                let (a, b, c) = div.unpack();
                let c = r!(c);
                self.alu_error(a.into(), Word::div, r!(b), c, c == 0)?;
            }

            Instruction::DIVI(divi) => {
                self.gas_charge(self.gas_costs.divi)?;
                let (a, b, imm) = divi.unpack();
                let imm = Word::from(imm);
                self.alu_error(a.into(), Word::div, r!(b), imm, imm == 0)?;
            }

            Instruction::EQ(eq) => {
                self.gas_charge(self.gas_costs.eq)?;
                let (a, b, c) = eq.unpack();
                self.alu_set(a.into(), (r!(b) == r!(c)) as Word)?;
            }

            Instruction::EXP(exp) => {
                self.gas_charge(self.gas_costs.exp)?;
                let (a, b, c) = exp.unpack();
                self.alu_boolean_overflow(a.into(), alu::exp, r!(b), r!(c))?;
            }

            Instruction::EXPI(expi) => {
                self.gas_charge(self.gas_costs.expi)?;
                let (a, b, imm) = expi.unpack();
                let expo = u32::from(imm);
                self.alu_boolean_overflow(a.into(), Word::overflowing_pow, r!(b), expo)?;
            }

            Instruction::GT(gt) => {
                self.gas_charge(self.gas_costs.gt)?;
                let (a, b, c) = gt.unpack();
                self.alu_set(a.into(), (r!(b) > r!(c)) as Word)?;
            }

            Instruction::LT(lt) => {
                self.gas_charge(self.gas_costs.lt)?;
                let (a, b, c) = lt.unpack();
                self.alu_set(a.into(), (r!(b) < r!(c)) as Word)?;
            }

            Instruction::MLOG(mlog) => {
                self.gas_charge(self.gas_costs.mlog)?;
                let (a, b, c) = mlog.unpack();
                let (lhs, rhs) = (r!(b), r!(c));
                self.alu_error(
                    a.into(),
                    |l, r| l.checked_ilog(r).expect("checked_ilog returned None for valid values") as Word,
                    lhs,
                    rhs,
                    lhs == 0 || rhs <= 1,
                )?;
            }

            Instruction::MOD(mod_) => {
                self.gas_charge(self.gas_costs.mod_op)?;
                let (a, b, c) = mod_.unpack();
                let rhs = r!(c);
                self.alu_error(a.into(), Word::wrapping_rem, r!(b), rhs, rhs == 0)?;
            }

            Instruction::MODI(modi) => {
                self.gas_charge(self.gas_costs.modi)?;
                let (a, b, imm) = modi.unpack();
                let rhs = Word::from(imm);
                self.alu_error(a.into(), Word::wrapping_rem, r!(b), rhs, rhs == 0)?;
            }

            Instruction::MOVE(move_) => {
                self.gas_charge(self.gas_costs.move_op)?;
                let (a, b) = move_.unpack();
                self.alu_set(a.into(), r!(b))?;
            }

            Instruction::MOVI(movi) => {
                self.gas_charge(self.gas_costs.movi)?;
                let (a, imm) = movi.unpack();
                self.alu_set(a.into(), Word::from(imm))?;
            }

            Instruction::MROO(mroo) => {
                self.gas_charge(self.gas_costs.mroo)?;
                let (a, b, c) = mroo.unpack();
                let (lhs, rhs) = (r!(b), r!(c));
                self.alu_error(
                    a.into(),
                    |l, r| checked_nth_root(l, r).expect("checked_nth_root returned None for valid values") as Word,
                    lhs,
                    rhs,
                    rhs == 0,
                )?;
            }

            Instruction::MUL(mul) => {
                self.gas_charge(self.gas_costs.mul)?;
                let (a, b, c) = mul.unpack();
                self.alu_capture_overflow(a.into(), u128::overflowing_mul, r!(b).into(), r!(c).into())?;
            }

            Instruction::MULI(muli) => {
                self.gas_charge(self.gas_costs.muli)?;
                let (a, b, imm) = muli.unpack();
                self.alu_capture_overflow(a.into(), u128::overflowing_mul, r!(b).into(), imm.into())?;
            }

            Instruction::NOOP(_noop) => {
                self.gas_charge(self.gas_costs.noop)?;
                self.alu_clear()?;
            }

            Instruction::NOT(not) => {
                self.gas_charge(self.gas_costs.not)?;
                let (a, b) = not.unpack();
                self.alu_set(a.into(), !r!(b))?;
            }

            Instruction::OR(or) => {
                self.gas_charge(self.gas_costs.or)?;
                let (a, b, c) = or.unpack();
                self.alu_set(a.into(), r!(b) | r!(c))?;
            }

            Instruction::ORI(ori) => {
                self.gas_charge(self.gas_costs.ori)?;
                let (a, b, imm) = ori.unpack();
                self.alu_set(a.into(), r!(b) | Word::from(imm))?;
            }

            Instruction::SLL(sll) => {
                self.gas_charge(self.gas_costs.sll)?;
                let (a, b, c) = sll.unpack();

                self.alu_error(
                    a.into(),
                    |l, r| {
                        l.checked_shl(u32::try_from(r).expect("value out of range"))
                            .unwrap_or_default()
                    },
                    r!(b),
                    r!(c),
                    u32::try_from(r!(c)).is_err(),
                )?;
            }

            Instruction::SLLI(slli) => {
                self.gas_charge(self.gas_costs.slli)?;
                let (a, b, imm) = slli.unpack();
                let rhs = u32::from(imm);
                self.alu_set(a.into(), r!(b).checked_shl(rhs).unwrap_or_default())?;
            }

            Instruction::SRL(srl) => {
                self.gas_charge(self.gas_costs.srl)?;
                let (a, b, c) = srl.unpack();
                self.alu_error(
                    a.into(),
                    |l, r| {
                        l.checked_shr(u32::try_from(r).expect("value out of range"))
                            .unwrap_or_default()
                    },
                    r!(b),
                    r!(c),
                    u32::try_from(r!(c)).is_err(),
                )?;
            }

            Instruction::SRLI(srli) => {
                self.gas_charge(self.gas_costs.srli)?;
                let (a, b, imm) = srli.unpack();
                let rhs = u32::from(imm);
                self.alu_set(a.into(), r!(b).checked_shr(rhs).unwrap_or_default())?;
            }

            Instruction::SUB(sub) => {
                self.gas_charge(self.gas_costs.sub)?;
                let (a, b, c) = sub.unpack();
                self.alu_capture_overflow(a.into(), u128::overflowing_sub, r!(b).into(), r!(c).into())?;
            }

            Instruction::SUBI(subi) => {
                self.gas_charge(self.gas_costs.subi)?;
                let (a, b, imm) = subi.unpack();
                self.alu_capture_overflow(a.into(), u128::overflowing_sub, r!(b).into(), imm.into())?;
            }

            Instruction::XOR(xor) => {
                self.gas_charge(self.gas_costs.xor)?;
                let (a, b, c) = xor.unpack();
                self.alu_set(a.into(), r!(b) ^ r!(c))?;
            }

            Instruction::XORI(xori) => {
                self.gas_charge(self.gas_costs.xori)?;
                let (a, b, imm) = xori.unpack();
                self.alu_set(a.into(), r!(b) ^ Word::from(imm))?;
            }

            Instruction::JI(ji) => {
                self.gas_charge(self.gas_costs.ji)?;
                let imm = ji.unpack();
                self.jump(imm.into())?;
            }

            Instruction::JNEI(jnei) => {
                self.gas_charge(self.gas_costs.jnei)?;
                let (a, b, imm) = jnei.unpack();
                self.jump_not_equal(r!(a), r!(b), imm.into())?;
            }

            Instruction::JNZI(jnzi) => {
                self.gas_charge(self.gas_costs.jnzi)?;
                let (a, imm) = jnzi.unpack();
                self.jump_not_zero(r!(a), imm.into())?;
            }

            Instruction::JMP(jmp) => {
                self.gas_charge(self.gas_costs.jmp)?;
                let a = jmp.unpack();
                self.jump(r!(a))?;
            }

            Instruction::JNE(jne) => {
                self.gas_charge(self.gas_costs.jne)?;
                let (a, b, c) = jne.unpack();
                self.jump_not_equal(r!(a), r!(b), r!(c))?;
            }

            Instruction::RET(ret) => {
                self.gas_charge(self.gas_costs.ret)?;
                let a = ret.unpack();
                let ra = r!(a);
                self.ret(ra)?;
                return Ok(ExecuteState::Return(ra));
            }

            Instruction::RETD(retd) => {
                let (a, b) = retd.unpack();
                let len = r!(b);
                self.dependent_gas_charge(self.gas_costs.retd, len)?;
                return self.ret_data(r!(a), len).map(ExecuteState::ReturnData);
            }

            Instruction::RVRT(rvrt) => {
                self.gas_charge(self.gas_costs.rvrt)?;
                let a = rvrt.unpack();
                let ra = r!(a);
                self.revert(ra);
                return Ok(ExecuteState::Revert(ra));
            }

            Instruction::SMO(smo) => {
                let (a, b, c, d) = smo.unpack();
                self.dependent_gas_charge(self.gas_costs.smo, r!(b))?;
                self.message_output(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::ALOC(aloc) => {
                self.gas_charge(self.gas_costs.aloc)?;
                let a = aloc.unpack();
                self.malloc(r!(a))?;
            }

            Instruction::CFEI(cfei) => {
                self.gas_charge(self.gas_costs.cfei)?;
                let imm = cfei.unpack();
                self.stack_pointer_overflow(Word::overflowing_add, imm.into())?;
            }

            Instruction::CFSI(cfsi) => {
                self.gas_charge(self.gas_costs.cfsi)?;
                let imm = cfsi.unpack();
                self.stack_pointer_overflow(Word::overflowing_sub, imm.into())?;
            }

            Instruction::LB(lb) => {
                self.gas_charge(self.gas_costs.lb)?;
                let (a, b, imm) = lb.unpack();
                self.load_byte(a.into(), r!(b), imm.into())?;
            }

            Instruction::LW(lw) => {
                self.gas_charge(self.gas_costs.lw)?;
                let (a, b, imm) = lw.unpack();
                self.load_word(a.into(), r!(b), imm.into())?;
            }

            Instruction::MCL(mcl) => {
                let (a, b) = mcl.unpack();
                let len = r!(b);
                self.dependent_gas_charge(self.gas_costs.mcl, len)?;
                self.memclear(r!(a), len)?;
            }

            Instruction::MCLI(mcli) => {
                let (a, imm) = mcli.unpack();
                let len = Word::from(imm);
                self.dependent_gas_charge(self.gas_costs.mcli, len)?;
                self.memclear(r!(a), len)?;
            }

            Instruction::MCP(mcp) => {
                let (a, b, c) = mcp.unpack();
                let len = r!(c);
                self.dependent_gas_charge(self.gas_costs.mcp, len)?;
                self.memcopy(r!(a), r!(b), len)?;
            }

            Instruction::MCPI(mcpi) => {
                self.gas_charge(self.gas_costs.mcpi)?;
                let (a, b, imm) = mcpi.unpack();
                let len = imm.into();
                self.memcopy(r!(a), r!(b), len)?;
            }

            Instruction::MEQ(meq) => {
                let (a, b, c, d) = meq.unpack();
                let len = r!(d);
                self.dependent_gas_charge(self.gas_costs.meq, len)?;
                self.memeq(a.into(), r!(b), r!(c), len)?;
            }

            Instruction::SB(sb) => {
                self.gas_charge(self.gas_costs.sb)?;
                let (a, b, imm) = sb.unpack();
                self.store_byte(r!(a), r!(b), imm.into())?;
            }

            Instruction::SW(sw) => {
                self.gas_charge(self.gas_costs.sw)?;
                let (a, b, imm) = sw.unpack();
                self.store_word(r!(a), r!(b), imm.into())?;
            }

            Instruction::BAL(bal) => {
                self.gas_charge(self.gas_costs.bal)?;
                let (a, b, c) = bal.unpack();
                self.contract_balance(a.into(), r!(b), r!(c))?;
            }

            Instruction::BHEI(bhei) => {
                self.gas_charge(self.gas_costs.bhei)?;
                let a = bhei.unpack();
                self.set_block_height(a.into())?;
            }

            Instruction::BHSH(bhsh) => {
                self.gas_charge(self.gas_costs.bhsh)?;
                let (a, b) = bhsh.unpack();
                self.block_hash(r!(a), r!(b))?;
            }

            Instruction::BURN(burn) => {
                self.gas_charge(self.gas_costs.burn)?;
                let a = burn.unpack();
                self.burn(r!(a))?;
            }

            Instruction::CALL(call) => {
                let (a, b, c, d) = call.unpack();
                let state = self.call(r!(a), r!(b), r!(c), r!(d))?;
                // raise revert state to halt execution for the callee
                if let ProgramState::Revert(ra) = state {
                    return Ok(ExecuteState::Revert(ra));
                }
            }

            Instruction::CB(cb) => {
                self.gas_charge(self.gas_costs.cb)?;
                let a = cb.unpack();
                self.block_proposer(r!(a))?;
            }

            Instruction::CCP(ccp) => {
                let (a, b, c, d) = ccp.unpack();
                let len = r!(d);
                self.dependent_gas_charge(self.gas_costs.ccp, len)?;
                self.code_copy(r!(a), r!(b), r!(c), len)?;
            }

            Instruction::CROO(croo) => {
                self.gas_charge(self.gas_costs.croo)?;
                let (a, b) = croo.unpack();
                self.code_root(r!(a), r!(b))?;
            }

            Instruction::CSIZ(csiz) => {
                let (a, b) = csiz.unpack();
                self.code_size(a.into(), r!(b))?;
            }

            Instruction::LDC(ldc) => {
                let (a, b, c) = ldc.unpack();
                self.dependent_gas_charge(self.gas_costs.ldc, r!(c))?;
                self.load_contract_code(r!(a), r!(b), r!(c))?;
            }

            Instruction::LOG(log) => {
                self.gas_charge(self.gas_costs.log)?;
                let (a, b, c, d) = log.unpack();
                self.log(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::LOGD(logd) => {
                let (a, b, c, d) = logd.unpack();
                self.dependent_gas_charge(self.gas_costs.logd, r!(d))?;
                self.log_data(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::MINT(mint) => {
                self.gas_charge(self.gas_costs.mint)?;
                let a = mint.unpack();
                self.mint(r!(a))?;
            }

            Instruction::SCWQ(scwq) => {
                self.gas_charge(self.gas_costs.scwq)?;
                let (a, b, c) = scwq.unpack();
                self.state_clear_qword(r!(a), b.into(), r!(c))?;
            }

            Instruction::SRW(srw) => {
                self.gas_charge(self.gas_costs.srw)?;
                let (a, b, c) = srw.unpack();
                self.state_read_word(a.into(), b.into(), r!(c))?;
            }

            Instruction::SRWQ(srwq) => {
                let (a, b, c, d) = srwq.unpack();
                self.dependent_gas_charge(self.gas_costs.srwq, r!(d))?;
                self.state_read_qword(r!(a), b.into(), r!(c), r!(d))?;
            }

            Instruction::SWW(sww) => {
                self.gas_charge(self.gas_costs.sww)?;
                let (a, b, c) = sww.unpack();
                self.state_write_word(r!(a), b.into(), r!(c))?;
            }

            Instruction::SWWQ(swwq) => {
                self.gas_charge(self.gas_costs.swwq)?;
                let (a, b, c, d) = swwq.unpack();
                self.state_write_qword(r!(a), b.into(), r!(c), r!(d))?;
            }

            Instruction::TIME(time) => {
                self.gas_charge(self.gas_costs.time)?;
                let (a, b) = time.unpack();
                self.timestamp(a.into(), r!(b))?;
            }

            Instruction::ECR(ecr) => {
                self.gas_charge(self.gas_costs.ecr)?;
                let (a, b, c) = ecr.unpack();
                self.ecrecover(r!(a), r!(b), r!(c))?;
            }

            Instruction::K256(k256) => {
                self.gas_charge(self.gas_costs.k256)?;
                let (a, b, c) = k256.unpack();
                self.keccak256(r!(a), r!(b), r!(c))?;
            }

            Instruction::S256(s256) => {
                self.gas_charge(self.gas_costs.s256)?;
                let (a, b, c) = s256.unpack();
                self.sha256(r!(a), r!(b), r!(c))?;
            }

            Instruction::FLAG(flag) => {
                self.gas_charge(self.gas_costs.flag)?;
                let a = flag.unpack();
                self.set_flag(r!(a))?;
            }

            Instruction::GM(gm) => {
                self.gas_charge(self.gas_costs.gm)?;
                let (a, imm) = gm.unpack();
                self.metadata(a.into(), imm.into())?;
            }

            Instruction::GTF(gtf) => {
                self.gas_charge(self.gas_costs.gtf)?;
                let (a, b, imm) = gtf.unpack();
                self.get_transaction_field(a.into(), r!(b), imm.into())?;
            }

            Instruction::TR(tr) => {
                self.gas_charge(self.gas_costs.tr)?;
                let (a, b, c) = tr.unpack();
                self.transfer(r!(a), r!(b), r!(c))?;
            }

            Instruction::TRO(tro) => {
                self.gas_charge(self.gas_costs.tro)?;
                let (a, b, c, d) = tro.unpack();
                self.transfer_output(r!(a), r!(b), r!(c), r!(d))?;
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

#[cfg(test)]
mod tests;
