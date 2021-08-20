use super::ExecuteState;
use crate::data::InterpreterStorage;
use crate::interpreter::{ExecuteError, Interpreter};

use fuel_asm::{Opcode, Word};

use std::ops::Div;

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn execute(&mut self, op: Opcode) -> Result<ExecuteState, ExecuteError> {
        let mut result = Ok(ExecuteState::Proceed);

        #[cfg(feature = "debug")]
        {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into());
            }
        }

        match op {
            Opcode::ADD(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_add, self.registers[rb], self.registers[rc])
            }

            Opcode::ADDI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_add, self.registers[rb], imm as Word)
            }

            Opcode::AND(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb] & self.registers[rc])
            }

            Opcode::ANDI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb] & (imm as Word))
            }

            Opcode::DIV(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => self
                .alu_error(
                    ra,
                    Word::div,
                    self.registers[rb],
                    self.registers[rc],
                    self.registers[rc] == 0,
                ),

            Opcode::DIVI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_error(ra, Word::div, self.registers[rb], imm as Word, imm == 0)
            }

            Opcode::EQ(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, (self.registers[rb] == self.registers[rc]) as Word)
            }

            Opcode::EXP(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_pow, self.registers[rb], self.registers[rc] as u32)
            }

            Opcode::EXPI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_pow, self.registers[rb], imm as u32)
            }

            Opcode::GT(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, (self.registers[rb] > self.registers[rc]) as Word)
            }

            Opcode::LT(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, (self.registers[rb] < self.registers[rc]) as Word)
            }

            Opcode::MLOG(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => self
                .alu_error(
                    ra,
                    |b, c| (b as f64).log(c as f64).trunc() as Word,
                    self.registers[rb],
                    self.registers[rc],
                    self.registers[rb] == 0 || self.registers[rc] <= 1,
                ),

            Opcode::MOD(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => self
                .alu_error(
                    ra,
                    Word::wrapping_rem,
                    self.registers[rb],
                    self.registers[rc],
                    self.registers[rc] == 0,
                ),

            Opcode::MODI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_error(ra, Word::wrapping_rem, self.registers[rb], imm as Word, imm == 0)
            }

            Opcode::MOVE(ra, rb) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb])
            }

            Opcode::MROO(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => self
                .alu_error(
                    ra,
                    |b, c| (b as f64).powf((c as f64).recip()).trunc() as Word,
                    self.registers[rb],
                    self.registers[rc],
                    self.registers[rc] == 0,
                ),

            Opcode::MUL(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_mul, self.registers[rb], self.registers[rc])
            }

            Opcode::MULI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_mul, self.registers[rb], imm as Word)
            }

            Opcode::NOOP if self.gas_charge(&op).is_ok() => self.alu_clear(),

            Opcode::NOT(ra, rb) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, !self.registers[rb])
            }

            Opcode::OR(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb] | self.registers[rc])
            }

            Opcode::ORI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb] | (imm as Word))
            }

            Opcode::SLL(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_shl, self.registers[rb], self.registers[rc] as u32)
            }

            Opcode::SLLI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_shl, self.registers[rb], imm as u32)
            }

            Opcode::SRL(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_shr, self.registers[rb], self.registers[rc] as u32)
            }

            Opcode::SRLI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_shr, self.registers[rb], imm as u32)
            }

            Opcode::SUB(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_sub, self.registers[rb], self.registers[rc])
            }

            Opcode::SUBI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_overflow(ra, Word::overflowing_sub, self.registers[rb], imm as Word)
            }

            Opcode::XOR(ra, rb, rc) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb] ^ self.registers[rc])
            }

            Opcode::XORI(ra, rb, imm) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.alu_set(ra, self.registers[rb] ^ (imm as Word))
            }

            Opcode::CIMV(ra, rb, rc)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self
                        .check_input_maturity(ra, self.registers[rb], self.registers[rc])
                        .is_ok() => {}

            Opcode::CTMV(ra, rb)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self.check_tx_maturity(ra, self.registers[rb]).is_ok() => {}

            Opcode::JI(imm) if self.gas_charge(&op).is_ok() && self.jump(imm as Word).is_ok() => {}

            Opcode::JNEI(ra, rb, imm)
                if self.gas_charge(&op).is_ok()
                    && self
                        .jump_not_equal_imm(self.registers[ra], self.registers[rb], imm as Word)
                        .is_ok() => {}

            Opcode::RET(ra) if self.gas_charge(&op).is_ok() && self.ret(ra).is_ok() => {
                result = Ok(ExecuteState::Return(self.registers[ra]));
            }

            Opcode::ALOC(ra) if self.gas_charge(&op).is_ok() && self.malloc(self.registers[ra]).is_ok() => {}

            Opcode::CFEI(imm) if self.gas_charge(&op).is_ok() && self.extend_call_frame(imm as Word).is_ok() => {}

            Opcode::CFSI(imm) if self.gas_charge(&op).is_ok() && self.shrink_call_frame(imm as Word).is_ok() => {}

            Opcode::LB(ra, rb, imm)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self.load_byte(ra, self.registers[rb], imm as Word).is_ok() => {}

            Opcode::LW(ra, rb, imm)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self.load_word(ra, self.registers[rb], imm as Word).is_ok() => {}

            Opcode::MCL(ra, rb)
                if self.gas_charge(&op).is_ok() && self.memclear(self.registers[ra], self.registers[rb]).is_ok() => {}

            Opcode::MCLI(ra, imm)
                if self.gas_charge(&op).is_ok() && self.memclear(self.registers[ra], imm as Word).is_ok() => {}

            Opcode::MCP(ra, rb, rc)
                if self.gas_charge(&op).is_ok()
                    && self
                        .memcopy(self.registers[ra], self.registers[rb], self.registers[rc])
                        .is_ok() => {}

            Opcode::MEQ(ra, rb, rc, rd)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self
                        .memeq(ra, self.registers[rb], self.registers[rc], self.registers[rd])
                        .is_ok() => {}

            Opcode::SB(ra, rb, imm)
                if self.gas_charge(&op).is_ok()
                    && self
                        .store_byte(self.registers[ra], self.registers[rb], imm as Word)
                        .is_ok() => {}

            Opcode::SW(ra, rb, imm)
                if self.gas_charge(&op).is_ok()
                    && self
                        .store_word(self.registers[ra], self.registers[rb], imm as Word)
                        .is_ok() => {}

            Opcode::BHEI(ra) if Self::is_register_writable(ra) && self.gas_charge(&op).is_ok() => {
                self.registers[ra] = self.block_height() as Word;
                self.inc_pc();
            }

            Opcode::BHSH(ra, rb)
                if self.gas_charge(&op).is_ok() && self.block_hash(self.registers[ra], self.registers[rb]).is_ok() => {}

            Opcode::BURN(ra) if self.gas_charge(&op).is_ok() && self.burn(self.registers[ra]).is_ok() => {}

            Opcode::CALL(ra, rb, rc, rd)
                if self.gas_charge(&op).is_ok()
                    && self
                        .call(
                            self.registers[ra],
                            self.registers[rb],
                            self.registers[rc],
                            self.registers[rd],
                        )
                        .is_ok() => {}

            Opcode::CB(ra) if self.gas_charge(&op).is_ok() && self.block_proposer(self.registers[ra]).is_ok() => {}

            Opcode::CCP(ra, rb, rc, rd)
                if self.gas_charge(&op).is_ok()
                    && self
                        .code_copy(
                            self.registers[ra],
                            self.registers[rb],
                            self.registers[rc],
                            self.registers[rd],
                        )
                        .is_ok() => {}

            Opcode::CROO(ra, rb)
                if self.gas_charge(&op).is_ok() && self.code_root(self.registers[ra], self.registers[rb]).is_ok() => {}

            Opcode::CSIZ(ra, rb)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self.code_size(ra, self.registers[rb]).is_ok() => {}

            // TODO LDC
            // TODO Append to receipts
            Opcode::LOG(ra, rb, rc, rd) if self.gas_charge(&op).is_ok() && self.log_append(&[ra, rb, rc, rd]) => {}

            // TODO LOGD
            Opcode::MINT(ra) if self.gas_charge(&op).is_ok() && self.mint(self.registers[ra]).is_ok() => {}

            // TODO RETD
            // TODO RVRT
            // TODO SLDC
            Opcode::SRW(ra, rb)
                if Self::is_register_writable(ra)
                    && self.gas_charge(&op).is_ok()
                    && self.state_read_word(ra, self.registers[rb]).is_ok() => {}

            Opcode::SRWQ(ra, rb)
                if self.gas_charge(&op).is_ok()
                    && self.state_read_qword(self.registers[ra], self.registers[rb]).is_ok() => {}

            Opcode::SWW(ra, rb)
                if self.gas_charge(&op).is_ok()
                    && self.state_write_word(self.registers[ra], self.registers[rb]).is_ok() => {}

            Opcode::SWWQ(ra, rb)
                if self.gas_charge(&op).is_ok()
                    && self.state_write_qword(self.registers[ra], self.registers[rb]).is_ok() => {}

            Opcode::ECR(ra, rb, rc)
                if self.gas_charge(&op).is_ok()
                    && self
                        .ecrecover(self.registers[ra], self.registers[rb], self.registers[rc])
                        .is_ok() => {}

            Opcode::K256(ra, rb, rc)
                if self.gas_charge(&op).is_ok()
                    && self
                        .keccak256(self.registers[ra], self.registers[rb], self.registers[rc])
                        .is_ok() => {}

            Opcode::S256(ra, rb, rc)
                if self.gas_charge(&op).is_ok()
                    && self
                        .sha256(self.registers[ra], self.registers[rb], self.registers[rc])
                        .is_ok() => {}

            Opcode::XIL(ra, rb)
                if self.gas_charge(&op).is_ok() && self.transaction_input_length(ra, self.registers[rb]).is_ok() => {}

            Opcode::XIS(ra, rb)
                if self.gas_charge(&op).is_ok() && self.transaction_input_start(ra, self.registers[rb]).is_ok() => {}

            Opcode::XOL(ra, rb)
                if self.gas_charge(&op).is_ok() && self.transaction_output_length(ra, self.registers[rb]).is_ok() => {}

            Opcode::XOS(ra, rb)
                if self.gas_charge(&op).is_ok() && self.transaction_output_start(ra, self.registers[rb]).is_ok() => {}

            Opcode::XWL(ra, rb)
                if self.gas_charge(&op).is_ok() && self.transaction_witness_length(ra, self.registers[rb]).is_ok() => {}

            Opcode::XWS(ra, rb)
                if self.gas_charge(&op).is_ok() && self.transaction_witness_start(ra, self.registers[rb]).is_ok() => {}

            Opcode::FLAG(ra) if self.gas_charge(&op).is_ok() => {
                self.set_flag(self.registers[ra]);
            }

            _ => result = Err(ExecuteError::OpcodeFailure(op)),
        }

        result
    }
}
