use crate::gas::GasUnit;
use crate::interpreter::Interpreter;

use fuel_asm::OpcodeRepr;
use fuel_types::Word;

pub const GAS_ADD: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_ADDI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADDI);
pub const GAS_AND: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::AND);
pub const GAS_ANDI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ANDI);
pub const GAS_DIV: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::DIV);
pub const GAS_DIVI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::DIVI);
pub const GAS_EQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::EQ);
pub const GAS_EXP: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::EXP);
pub const GAS_EXPI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::EXPI);
pub const GAS_GT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::GT);
pub const GAS_LT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::LT);
pub const GAS_MLOG: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MLOG);
pub const GAS_MOD: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MOD);
pub const GAS_MODI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MODI);
pub const GAS_MOVE: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MOVE);
pub const GAS_MOVI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MOVI);
pub const GAS_MROO: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MROO);
pub const GAS_MUL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MUL);
pub const GAS_MULI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MULI);
pub const GAS_NOOP: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::NOOP);
pub const GAS_NOT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::NOT);
pub const GAS_OR: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::OR);
pub const GAS_ORI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ORI);
pub const GAS_SLL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SLL);
pub const GAS_SLLI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SLLI);
pub const GAS_SRL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SRL);
pub const GAS_SRLI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SRLI);
pub const GAS_SUB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SUB);
pub const GAS_SUBI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SUBI);
pub const GAS_XOR: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::XOR);
pub const GAS_XORI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::XORI);
pub const GAS_JI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::JI);
pub const GAS_JNEI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::JNEI);
pub const GAS_JNZI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::JNZI);
pub const GAS_JMP: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::JMP);
pub const GAS_JNE: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::JNE);
pub const GAS_RET: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::RET);
pub const GAS_RETD: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::RETD);
pub const GAS_RVRT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::RVRT);
pub const GAS_SMO: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SMO);
pub const GAS_ALOC: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ALOC);
pub const GAS_CFEI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::CFEI);
pub const GAS_CFSI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::CFSI);
pub const GAS_LB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::LB);
pub const GAS_LW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::LW);
pub const GAS_SB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SB);
pub const GAS_SW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SW);
pub const GAS_BAL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::BAL);
pub const GAS_BHEI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::BHEI);
pub const GAS_BHSH: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::BHSH);
pub const GAS_BURN: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::BURN);
pub const GAS_CALL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::CALL);
pub const GAS_CB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::CB);
pub const GAS_CROO: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::CROO);
pub const GAS_CSIZ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::CSIZ);
pub const GAS_LDC: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::LDC);
pub const GAS_LOG: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::LOG);
pub const GAS_LOGD: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::LOGD);
pub const GAS_MINT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::MINT);
pub const GAS_SCWQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SCWQ);
pub const GAS_SRW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SRW);
pub const GAS_SRWQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SRWQ);
pub const GAS_SWW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SWW);
pub const GAS_SWWQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::SWWQ);
pub const GAS_TIME: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::TIME);
pub const GAS_ECR: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ECR);
pub const GAS_K256: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::K256);
pub const GAS_S256: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::S256);
pub const GAS_FLAG: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::FLAG);
pub const GAS_GM: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::GM);
pub const GAS_GTF: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::GTF);
pub const GAS_TR: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::TR);
pub const GAS_TRO: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::TRO);

const GAS_MCL_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCL);
const GAS_MCLI_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCLI);
const GAS_MCP_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCP);
const GAS_MCPI_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCPI);
const GAS_MEQ_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MEQ);
const GAS_CCP_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::CCP);

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
