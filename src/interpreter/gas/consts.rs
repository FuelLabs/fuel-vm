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

// TODO define gas cost
pub const GAS_CIMV: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CTMV: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_JI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_JNEI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_RET: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_RETD: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_RVRT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_ALOC: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CFEI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CFSI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_LB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_LW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_MEQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_SB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_SW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_BAL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_BHEI: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_BHSH: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_BURN: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CALL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CB: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CCP: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CROO: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_CSIZ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_LDC: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_LOG: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_LOGD: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_MINT: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_SRW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_SRWQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_SWW: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_SWWQ: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_ECR: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_K256: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_S256: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_XIL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_XIS: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_XOL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_XOS: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_XWL: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_XWS: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_FLAG: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_GM: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
pub const GAS_TR: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);
// pub const GAS_TRO: Word = Interpreter::<()>::gas_cost_const(OpcodeRepr::ADD);

// Variable gas cost
const GAS_OP_MEMORY_WRITE: Word = GasUnit::MemoryWrite(0).unit_price();

const GAS_MCL_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCL);
const GAS_MCLI_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCLI);
const GAS_MCP_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCP);
const GAS_MCPI_BASE: Word = Interpreter::<()>::gas_cost_monad_base(OpcodeRepr::MCPI);

pub const GAS_MCL: fn(Word) -> Word = |m| GAS_MCL_BASE + GAS_OP_MEMORY_WRITE * m;
pub const GAS_MCLI: fn(Word) -> Word = |m| GAS_MCLI_BASE + GAS_OP_MEMORY_WRITE * m;
pub const GAS_MCP: fn(Word) -> Word = |m| GAS_MCP_BASE + GAS_OP_MEMORY_WRITE * m;
pub const GAS_MCPI: fn(Word) -> Word = |m| GAS_MCPI_BASE + GAS_OP_MEMORY_WRITE * m;
