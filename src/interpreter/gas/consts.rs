use crate::gas::GasUnit;
use crate::interpreter::Interpreter;

use fuel_asm::Opcode;
use fuel_types::Word;

pub const GAS_ADD: Word = Interpreter::<()>::gas_cost_const(Opcode::ADD);
pub const GAS_ADDI: Word = Interpreter::<()>::gas_cost_const(Opcode::ADDI);
pub const GAS_AND: Word = Interpreter::<()>::gas_cost_const(Opcode::AND);
pub const GAS_ANDI: Word = Interpreter::<()>::gas_cost_const(Opcode::ANDI);
pub const GAS_DIV: Word = Interpreter::<()>::gas_cost_const(Opcode::DIV);
pub const GAS_DIVI: Word = Interpreter::<()>::gas_cost_const(Opcode::DIVI);
pub const GAS_EQ: Word = Interpreter::<()>::gas_cost_const(Opcode::EQ);
pub const GAS_EXP: Word = Interpreter::<()>::gas_cost_const(Opcode::EXP);
pub const GAS_EXPI: Word = Interpreter::<()>::gas_cost_const(Opcode::EXPI);
pub const GAS_GT: Word = Interpreter::<()>::gas_cost_const(Opcode::GT);
pub const GAS_LT: Word = Interpreter::<()>::gas_cost_const(Opcode::LT);
pub const GAS_MLOG: Word = Interpreter::<()>::gas_cost_const(Opcode::MLOG);
pub const GAS_MOD: Word = Interpreter::<()>::gas_cost_const(Opcode::MOD);
pub const GAS_MODI: Word = Interpreter::<()>::gas_cost_const(Opcode::MODI);
pub const GAS_MOVE: Word = Interpreter::<()>::gas_cost_const(Opcode::MOVE);
pub const GAS_MOVI: Word = Interpreter::<()>::gas_cost_const(Opcode::MOVI);
pub const GAS_MROO: Word = Interpreter::<()>::gas_cost_const(Opcode::MROO);
pub const GAS_MUL: Word = Interpreter::<()>::gas_cost_const(Opcode::MUL);
pub const GAS_MULI: Word = Interpreter::<()>::gas_cost_const(Opcode::MULI);
pub const GAS_NOOP: Word = Interpreter::<()>::gas_cost_const(Opcode::NOOP);
pub const GAS_NOT: Word = Interpreter::<()>::gas_cost_const(Opcode::NOT);
pub const GAS_OR: Word = Interpreter::<()>::gas_cost_const(Opcode::OR);
pub const GAS_ORI: Word = Interpreter::<()>::gas_cost_const(Opcode::ORI);
pub const GAS_SLL: Word = Interpreter::<()>::gas_cost_const(Opcode::SLL);
pub const GAS_SLLI: Word = Interpreter::<()>::gas_cost_const(Opcode::SLLI);
pub const GAS_SRL: Word = Interpreter::<()>::gas_cost_const(Opcode::SRL);
pub const GAS_SRLI: Word = Interpreter::<()>::gas_cost_const(Opcode::SRLI);
pub const GAS_SUB: Word = Interpreter::<()>::gas_cost_const(Opcode::SUB);
pub const GAS_SUBI: Word = Interpreter::<()>::gas_cost_const(Opcode::SUBI);
pub const GAS_XOR: Word = Interpreter::<()>::gas_cost_const(Opcode::XOR);
pub const GAS_XORI: Word = Interpreter::<()>::gas_cost_const(Opcode::XORI);
pub const GAS_JI: Word = Interpreter::<()>::gas_cost_const(Opcode::JI);
pub const GAS_JNEI: Word = Interpreter::<()>::gas_cost_const(Opcode::JNEI);
pub const GAS_JNZI: Word = Interpreter::<()>::gas_cost_const(Opcode::JNZI);
pub const GAS_JMP: Word = Interpreter::<()>::gas_cost_const(Opcode::JMP);
pub const GAS_JNE: Word = Interpreter::<()>::gas_cost_const(Opcode::JNE);
pub const GAS_RET: Word = Interpreter::<()>::gas_cost_const(Opcode::RET);
pub const GAS_RETD: Word = Interpreter::<()>::gas_cost_const(Opcode::RETD);
pub const GAS_RVRT: Word = Interpreter::<()>::gas_cost_const(Opcode::RVRT);
pub const GAS_SMO: Word = Interpreter::<()>::gas_cost_const(Opcode::SMO);
pub const GAS_ALOC: Word = Interpreter::<()>::gas_cost_const(Opcode::ALOC);
pub const GAS_CFEI: Word = Interpreter::<()>::gas_cost_const(Opcode::CFEI);
pub const GAS_CFSI: Word = Interpreter::<()>::gas_cost_const(Opcode::CFSI);
pub const GAS_LB: Word = Interpreter::<()>::gas_cost_const(Opcode::LB);
pub const GAS_LW: Word = Interpreter::<()>::gas_cost_const(Opcode::LW);
pub const GAS_SB: Word = Interpreter::<()>::gas_cost_const(Opcode::SB);
pub const GAS_SW: Word = Interpreter::<()>::gas_cost_const(Opcode::SW);
pub const GAS_BAL: Word = Interpreter::<()>::gas_cost_const(Opcode::BAL);
pub const GAS_BHEI: Word = Interpreter::<()>::gas_cost_const(Opcode::BHEI);
pub const GAS_BHSH: Word = Interpreter::<()>::gas_cost_const(Opcode::BHSH);
pub const GAS_BURN: Word = Interpreter::<()>::gas_cost_const(Opcode::BURN);
pub const GAS_CALL: Word = Interpreter::<()>::gas_cost_const(Opcode::CALL);
pub const GAS_CB: Word = Interpreter::<()>::gas_cost_const(Opcode::CB);
pub const GAS_CROO: Word = Interpreter::<()>::gas_cost_const(Opcode::CROO);
pub const GAS_CSIZ: Word = Interpreter::<()>::gas_cost_const(Opcode::CSIZ);
pub const GAS_LDC: Word = Interpreter::<()>::gas_cost_const(Opcode::LDC);
pub const GAS_LOG: Word = Interpreter::<()>::gas_cost_const(Opcode::LOG);
pub const GAS_LOGD: Word = Interpreter::<()>::gas_cost_const(Opcode::LOGD);
pub const GAS_MINT: Word = Interpreter::<()>::gas_cost_const(Opcode::MINT);
pub const GAS_SCWQ: Word = Interpreter::<()>::gas_cost_const(Opcode::SCWQ);
pub const GAS_SRW: Word = Interpreter::<()>::gas_cost_const(Opcode::SRW);
pub const GAS_SRWQ: Word = Interpreter::<()>::gas_cost_const(Opcode::SRWQ);
pub const GAS_SWW: Word = Interpreter::<()>::gas_cost_const(Opcode::SWW);
pub const GAS_SWWQ: Word = Interpreter::<()>::gas_cost_const(Opcode::SWWQ);
pub const GAS_TIME: Word = Interpreter::<()>::gas_cost_const(Opcode::TIME);
pub const GAS_ECR: Word = Interpreter::<()>::gas_cost_const(Opcode::ECR);
pub const GAS_K256: Word = Interpreter::<()>::gas_cost_const(Opcode::K256);
pub const GAS_S256: Word = Interpreter::<()>::gas_cost_const(Opcode::S256);
pub const GAS_FLAG: Word = Interpreter::<()>::gas_cost_const(Opcode::FLAG);
pub const GAS_GM: Word = Interpreter::<()>::gas_cost_const(Opcode::GM);
pub const GAS_GTF: Word = Interpreter::<()>::gas_cost_const(Opcode::GTF);
pub const GAS_TR: Word = Interpreter::<()>::gas_cost_const(Opcode::TR);
pub const GAS_TRO: Word = Interpreter::<()>::gas_cost_const(Opcode::TRO);

const GAS_MCL_BASE: Word = Interpreter::<()>::gas_cost_monad_base(Opcode::MCL);
const GAS_MCLI_BASE: Word = Interpreter::<()>::gas_cost_monad_base(Opcode::MCLI);
const GAS_MCP_BASE: Word = Interpreter::<()>::gas_cost_monad_base(Opcode::MCP);
const GAS_MCPI_BASE: Word = Interpreter::<()>::gas_cost_monad_base(Opcode::MCPI);
const GAS_MEQ_BASE: Word = Interpreter::<()>::gas_cost_monad_base(Opcode::MEQ);
const GAS_CCP_BASE: Word = Interpreter::<()>::gas_cost_monad_base(Opcode::CCP);

const fn memory_read(bytes: Word) -> Word {
    const PAGE_SIZE: u32 = 4096;

    GasUnit::MemoryRead(1)
        .unit_price()
        .saturating_pow(bytes as u32 / PAGE_SIZE)
}

const fn memory_write(bytes: Word) -> Word {
    const PAGE_SIZE: u32 = 4096;

    GasUnit::MemoryWrite(1)
        .unit_price()
        .saturating_pow(bytes as u32 / PAGE_SIZE)
}

pub const GAS_MCL: fn(Word) -> Word = |m| GAS_MCL_BASE.saturating_add(memory_write(m));
pub const GAS_MCLI: fn(Word) -> Word = |m| GAS_MCLI_BASE.saturating_add(memory_write(m));
pub const GAS_MCP: fn(Word) -> Word = |m| GAS_MCP_BASE.saturating_add(memory_write(m));
pub const GAS_MCPI: fn(Word) -> Word = |m| GAS_MCPI_BASE.saturating_add(memory_write(m));
pub const GAS_CCP: fn(Word) -> Word = |m| GAS_CCP_BASE.saturating_add(memory_write(m));
pub const GAS_MEQ: fn(Word) -> Word = |m| GAS_MEQ_BASE.saturating_add(memory_read(m));
