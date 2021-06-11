use super::{Contract, ExecuteError, Interpreter, MemoryRange};
use crate::consts::*;
use crate::data::InterpreterStorage;

use fuel_asm::{Opcode, RegisterId, Word};
use fuel_tx::bytes::Deserializable;
use fuel_tx::bytes::{SerializableVec, SizedBytes};
use fuel_tx::consts::*;
use fuel_tx::{Color, Input, Output, Transaction};
use itertools::Itertools;
use tracing::debug;

use std::convert::TryFrom;
use std::mem;
use std::ops::Div;

const WORD_SIZE: usize = mem::size_of::<Word>();

impl<S> Interpreter<S>
where
    S: InterpreterStorage,
{
    pub fn init(&mut self, tx: Transaction) -> Result<(), ExecuteError> {
        self._init(tx, false)
    }

    fn _init(&mut self, mut tx: Transaction, predicate: bool) -> Result<(), ExecuteError> {
        tx.validate(self.block_height() as Word)?;

        self.frames.clear();
        self.log.clear();

        // Optimized for memset
        self.registers.iter_mut().for_each(|r| *r = 0);

        self.registers[REG_ONE] = 1;
        self.registers[REG_SSP] = 0;

        // Set heap area
        self.registers[REG_FP] = VM_MAX_RAM - 1;
        self.registers[REG_HP] = self.registers[REG_FP];

        self.push_stack(tx.id().as_ref())?;

        let zeroes = &[0; MAX_INPUTS as usize * (Color::size_of() + WORD_SIZE)];
        let mut ssp = self.registers[REG_SSP] as usize;

        self.push_stack(zeroes)?;
        if !predicate {
            tx.inputs()
                .iter()
                .filter_map(|input| match input {
                    Input::Coin { color, .. } => Some(color),
                    _ => None,
                })
                .sorted()
                .take(MAX_INPUTS as usize)
                .try_for_each::<_, Result<_, ExecuteError>>(|color| {
                    let balance = self.color_balance(color)?;

                    self.memory[ssp..ssp + Color::size_of()].copy_from_slice(color.as_ref());
                    ssp += Color::size_of();

                    self.memory[ssp..ssp + WORD_SIZE].copy_from_slice(&balance.to_be_bytes());
                    ssp += WORD_SIZE;

                    Ok(())
                })?;
        }

        let tx_size = tx.serialized_size() as Word;
        self.push_stack(&tx_size.to_be_bytes())?;
        self.push_stack(tx.to_bytes().as_slice())?;

        self.registers[REG_SP] = self.registers[REG_SSP];

        self.tx = tx;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), ExecuteError> {
        let tx = &self.tx;

        match tx {
            Transaction::Create {
                salt, static_contracts, ..
            } => {
                if static_contracts
                    .iter()
                    .any(|id| !self.check_contract_exists(id).unwrap_or(false))
                {
                    Err(ExecuteError::TransactionCreateStaticContractNotFound)?
                }

                let contract = Contract::try_from(tx)?;
                let id = contract.address(salt.as_ref());
                if !tx
                    .outputs()
                    .iter()
                    .any(|output| matches!(output, Output::ContractCreated { contract_id } if contract_id == &id))
                {
                    Err(ExecuteError::TransactionCreateIdNotInTx)?;
                }

                self.storage.insert(id, contract)?;

                // Verify predicates
                // https://github.com/FuelLabs/fuel-specs/blob/master/specs/protocol/tx_validity.md#predicate-verification
                // TODO this should be abstracted with the client
                let predicates: Vec<MemoryRange> = tx
                    .inputs()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, input)| match input {
                        Input::Coin { predicate, .. } if !predicate.is_empty() => tx
                            .input_coin_predicate_offset(i)
                            .map(|ofs| (ofs as Word, predicate.len() as Word)),
                        _ => None,
                    })
                    .map(|(ofs, len)| (ofs + Self::tx_mem_address() as Word, len))
                    .map(|(ofs, len)| MemoryRange::new(ofs, len))
                    .collect();

                predicates
                    .iter()
                    .try_for_each(|predicate| self.verify_predicate(predicate))?;

                Ok(())
            }

            Transaction::Script { .. } => {
                let offset = (Self::tx_mem_address() + Transaction::script_offset()) as Word;

                self.registers[REG_PC] = offset;
                self.registers[REG_IS] = offset;
                self.registers[REG_GGAS] = self.tx.gas_limit();
                self.registers[REG_CGAS] = self.tx.gas_limit();

                // TODO set tree balance

                self.run_program()
            }
        }
    }

    pub fn run_program(&mut self) -> Result<(), ExecuteError> {
        loop {
            if self.registers[REG_PC] >= VM_MAX_RAM {
                return Err(ExecuteError::ProgramOverflow);
            }

            let op = self.memory[self.registers[REG_PC] as usize..]
                .chunks_exact(4)
                .next()
                .map(Opcode::from_bytes_unchecked)
                .ok_or(ExecuteError::ProgramOverflow)?;

            if let Opcode::RET(ra) = op {
                if Self::is_valid_register(ra) && self.ret(ra) && self.inc_pc() {
                    return Ok(());
                } else {
                    return Err(ExecuteError::OpcodeFailure(op));
                }
            }

            self.execute(op)?;
        }
    }

    pub fn verify_predicate(&mut self, predicate: &MemoryRange) -> Result<(), ExecuteError> {
        // TODO initialize VM with tx prepared for sign
        let (start, end) = predicate.boundaries(&self);

        self.registers[REG_PC] = start;
        self.registers[REG_IS] = start;

        // TODO optimize
        loop {
            let pc = self.registers[REG_PC];
            let op = self.memory[pc as usize..]
                .chunks_exact(Opcode::BYTES_SIZE)
                .next()
                .map(Opcode::from_bytes_unchecked)
                .ok_or(ExecuteError::PredicateOverflow)?;

            if let Opcode::RET(ra) = op {
                return self
                    .registers
                    .get(ra)
                    .ok_or(ExecuteError::OpcodeFailure(op))
                    .and_then(|ret| {
                        if ret == &1 {
                            Ok(())
                        } else {
                            Err(ExecuteError::PredicateFailure)
                        }
                    });
            }

            self.execute(op)?;

            if self.registers[REG_PC] < pc || self.registers[REG_PC] >= end {
                return Err(ExecuteError::PredicateOverflow);
            }
        }
    }

    pub fn execute_tx_bytes(storage: S, bytes: &[u8]) -> Result<Self, ExecuteError> {
        let tx = Transaction::from_bytes(bytes)?;

        Self::execute_tx(storage, tx)
    }

    pub fn execute_tx(storage: S, tx: Transaction) -> Result<Self, ExecuteError> {
        let mut vm = Interpreter::with_storage(storage);

        vm.init(tx)?;
        vm.run()?;

        Ok(vm)
    }

    pub fn execute(&mut self, op: Opcode) -> Result<(), ExecuteError> {
        let mut result = Ok(());

        debug!("Executing {:?}", op);
        debug!(
            "Current state: {:?}",
            op.registers()
                .iter()
                .filter_map(|r| *r)
                .map(|r| (r, self.registers.get(r).copied()))
                .collect::<Vec<(RegisterId, Option<Word>)>>()
        );

        match op {
            Opcode::ADD(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_overflow(ra, Word::overflowing_add, self.registers[rb], self.registers[rc])
            }

            Opcode::ADDI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_overflow(ra, Word::overflowing_add, self.registers[rb], imm as Word)
            }

            Opcode::AND(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_set(ra, self.registers[rb] & self.registers[rc])
            }

            Opcode::ANDI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_set(ra, self.registers[rb] & (imm as Word))
            }

            Opcode::DIV(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => self.alu_error(
                ra,
                Word::div,
                self.registers[rb],
                self.registers[rc],
                self.registers[rc] == 0,
            ),

            Opcode::DIVI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_error(ra, Word::div, self.registers[rb], imm as Word, imm == 0)
            }

            Opcode::EQ(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_set(ra, (self.registers[rb] == self.registers[rc]) as Word)
            }

            Opcode::EXP(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_overflow(ra, Word::overflowing_pow, self.registers[rb], self.registers[rc] as u32)
            }

            Opcode::EXPI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_overflow(ra, Word::overflowing_pow, self.registers[rb], imm as u32)
            }

            Opcode::GT(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_set(ra, (self.registers[rb] > self.registers[rc]) as Word)
            }

            Opcode::MLOG(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => self.alu_error(
                ra,
                |b, c| (b as f64).log(c as f64).trunc() as Word,
                self.registers[rb],
                self.registers[rc],
                self.registers[rb] == 0 || self.registers[rc] <= 1,
            ),

            Opcode::MROO(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => self.alu_error(
                ra,
                |b, c| (b as f64).powf((c as f64).recip()).trunc() as Word,
                self.registers[rb],
                self.registers[rc],
                self.registers[rc] == 0,
            ),

            Opcode::MOD(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => self.alu_error(
                ra,
                Word::wrapping_rem,
                self.registers[rb],
                self.registers[rc],
                self.registers[rc] == 0,
            ),

            Opcode::MODI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_error(ra, Word::wrapping_rem, self.registers[rb], imm as Word, imm == 0)
            }

            Opcode::MOVE(ra, rb) if Self::is_valid_register_couple_alu(ra, rb) => self.alu_set(ra, self.registers[rb]),

            Opcode::MUL(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_overflow(ra, Word::overflowing_mul, self.registers[rb], self.registers[rc])
            }

            Opcode::MULI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_overflow(ra, Word::overflowing_mul, self.registers[rb], imm as Word)
            }

            Opcode::NOOP => self.alu_clear(),

            Opcode::NOT(ra, rb) if Self::is_valid_register_couple_alu(ra, rb) => self.alu_set(ra, !self.registers[rb]),

            Opcode::OR(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_set(ra, self.registers[rb] | self.registers[rc])
            }

            Opcode::ORI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_set(ra, self.registers[rb] | (imm as Word))
            }

            Opcode::SLL(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_overflow(ra, Word::overflowing_shl, self.registers[rb], self.registers[rc] as u32)
            }

            Opcode::SLLI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_overflow(ra, Word::overflowing_shl, self.registers[rb], imm as u32)
            }

            Opcode::SRL(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_overflow(ra, Word::overflowing_shr, self.registers[rb], self.registers[rc] as u32)
            }

            Opcode::SRLI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_overflow(ra, Word::overflowing_shr, self.registers[rb], imm as u32)
            }

            Opcode::SUB(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_overflow(ra, Word::overflowing_sub, self.registers[rb], self.registers[rc])
            }

            Opcode::SUBI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_overflow(ra, Word::overflowing_sub, self.registers[rb], imm as Word)
            }

            Opcode::XOR(ra, rb, rc) if Self::is_valid_register_triple_alu(ra, rb, rc) => {
                self.alu_set(ra, self.registers[rb] ^ self.registers[rc])
            }

            Opcode::XORI(ra, rb, imm) if Self::is_valid_register_couple_alu(ra, rb) => {
                self.alu_set(ra, self.registers[rb] ^ (imm as Word))
            }

            Opcode::CIMV(ra, rb, rc)
                if Self::is_valid_register_triple_alu(ra, rb, rc)
                    && self.check_input_maturity(ra, self.registers[rb], self.registers[rc])
                    && self.inc_pc() => {}

            Opcode::CTMV(ra, rb)
                if Self::is_valid_register_couple_alu(ra, rb)
                    && self.check_tx_maturity(ra, self.registers[rb])
                    && self.inc_pc() => {}

            Opcode::JI(imm) if self.jump(imm as Word) => {}

            Opcode::JNEI(ra, rb, imm)
                if Self::is_valid_register_couple(ra, rb)
                    && self.jump_not_equal_imm(self.registers[ra], self.registers[rb], imm as Word) => {}

            Opcode::RET(ra) if Self::is_valid_register(ra) && self.ret(ra) && self.inc_pc() => {}

            Opcode::ALOC(ra) if Self::is_valid_register(ra) && self.malloc(self.registers[ra]) && self.inc_pc() => {}

            Opcode::CFEI(imm) if self.stack_pointer_overflow(Word::overflowing_add, imm as Word) && self.inc_pc() => {}

            Opcode::CFSI(imm) if self.stack_pointer_overflow(Word::overflowing_sub, imm as Word) && self.inc_pc() => {}

            Opcode::LB(ra, rb, imm)
                if Self::is_valid_register_couple_alu(ra, rb)
                    && self.load_byte(ra, rb, imm as Word)
                    && self.inc_pc() => {}

            Opcode::LW(ra, rb, imm)
                if Self::is_valid_register_couple_alu(ra, rb)
                    && self.load_word(ra, self.registers[rb], imm as Word)
                    && self.inc_pc() => {}

            Opcode::MCL(ra, rb)
                if Self::is_valid_register_couple(ra, rb)
                    && self.memclear(self.registers[ra], self.registers[rb])
                    && self.inc_pc() => {}

            Opcode::MCLI(ra, imm)
                if Self::is_valid_register(ra) && self.memclear(self.registers[ra], imm as Word) && self.inc_pc() => {}

            Opcode::MCP(ra, rb, rc)
                if Self::is_valid_register_triple(ra, rb, rc)
                    && self.memcopy(self.registers[ra], self.registers[rb], self.registers[rc])
                    && self.inc_pc() => {}

            Opcode::MEQ(ra, rb, rc, rd)
                if Self::is_valid_register_quadruple_alu(ra, rb, rc, rd)
                    && self.memeq(ra, self.registers[rb], self.registers[rc], self.registers[rd])
                    && self.inc_pc() => {}

            Opcode::SB(ra, rb, imm)
                if Self::is_valid_register_couple(ra, rb)
                    && self.store_byte(self.registers[ra], self.registers[rb], imm as Word)
                    && self.inc_pc() => {}

            Opcode::SW(ra, rb, imm)
                if Self::is_valid_register_couple(ra, rb)
                    && self.store_word(self.registers[ra], self.registers[rb], imm as Word)
                    && self.inc_pc() => {}

            Opcode::BHEI(ra) if Self::is_valid_register_alu(ra) && self.inc_pc() => {
                self.registers[ra] = self.block_height() as Word
            }

            // TODO BLOCKHASH: Block hash
            Opcode::BURN(ra) if Self::is_valid_register(ra) && self.burn(self.registers[ra]) && self.inc_pc() => {}

            Opcode::CALL(ra, rb, rc, rd)
                if Self::is_valid_register_quadruple(ra, rb, rc, rd)
                    && self.call(
                        self.registers[ra],
                        self.registers[rb],
                        self.registers[rc],
                        self.registers[rd],
                    ) => {}

            Opcode::CCP(ra, rb, rc, rd)
                if Self::is_valid_register_quadruple(ra, rb, rc, rd)
                    && self.code_copy(
                        self.registers[ra],
                        self.registers[rb],
                        self.registers[rc],
                        self.registers[rd],
                    )
                    && self.inc_pc() => {}

            // TODO CODEROOT: Code Merkle root
            // TODO CODESIZE: Code size
            // TODO COINBASE: Block proposer address
            // TODO LOADCODE: Load code from an external contract
            Opcode::LOG(ra, rb, rc, rd)
                if Self::is_valid_register_quadruple(ra, rb, rc, rd)
                    && self.log_append(&[ra, rb, rc, rd])
                    && self.inc_pc() => {}

            Opcode::MINT(ra) if Self::is_valid_register(ra) && self.mint(self.registers[ra]) && self.inc_pc() => {}

            // TODO REVERT: Revert
            // TODO SLOADCODE: Load code from static list
            // TODO SRW: State read word
            // TODO SRWQ: State read 32 bytes
            // TODO SWW: State write word
            // TODO SWWQ: State write 32 bytes
            // TODO TRANSFER: Transfer coins to contract
            // TODO TRANSFEROUT: Transfer coins to output
            Opcode::ECR(ra, rb, rc)
                if Self::is_valid_register_triple(ra, rb, rc)
                    && self.ecrecover(self.registers[ra], self.registers[rb], self.registers[rc])
                    && self.inc_pc() => {}

            Opcode::K256(ra, rb, rc)
                if Self::is_valid_register_triple(ra, rb, rc)
                    && self.keccak256(self.registers[ra], self.registers[rb], self.registers[rc])
                    && self.inc_pc() => {}

            Opcode::S256(ra, rb, rc)
                if Self::is_valid_register_triple(ra, rb, rc)
                    && self.sha256(self.registers[ra], self.registers[rb], self.registers[rc])
                    && self.inc_pc() => {}

            Opcode::FLAG(ra) if Self::is_valid_register(ra) && self.inc_pc() => self.set_flag(self.registers[ra]),

            _ => result = Err(ExecuteError::OpcodeFailure(op)),
        }

        debug!(
            "After   state: {:?}",
            op.registers()
                .iter()
                .filter_map(|r| *r)
                .map(|r| (r, self.registers.get(r).copied()))
                .collect::<Vec<(RegisterId, Option<Word>)>>()
        );

        result
    }
}
