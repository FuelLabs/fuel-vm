use super::{ExecuteError, Interpreter};
use crate::consts::*;

use fuel_asm::{Opcode, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GasUnit {
    Atom(Word),
    Arithmetic(Word),
    ArithmeticExpensive(Word),
    RegisterRead(Word),
    RegisterWrite(Word),
    Branching(Word),
    MemoryOwnership(Word),
    MemoryWrite(Word),
    Accumulated(Word),
    Undefined,
}

impl GasUnit {
    pub const fn cost(&self) -> Word {
        use GasUnit::*;

        match self {
            Atom(1) => 1,
            Arithmetic(1) => 5,
            ArithmeticExpensive(1) => 7,
            RegisterRead(1) => 1,
            RegisterWrite(1) => 2,
            Branching(1) => 10,
            MemoryOwnership(1) => 9,
            MemoryWrite(1) => 8,
            Undefined => 20,

            Atom(n) => *n * Atom(1).cost(),
            Arithmetic(n) => *n * Arithmetic(1).cost(),
            ArithmeticExpensive(n) => *n * ArithmeticExpensive(1).cost(),
            RegisterRead(n) => *n * RegisterRead(1).cost(),
            RegisterWrite(n) => *n * RegisterWrite(1).cost(),
            Branching(n) => *n * Branching(1).cost(),
            MemoryOwnership(n) => *n * MemoryOwnership(1).cost(),
            MemoryWrite(n) => *n * MemoryWrite(1).cost(),
            Accumulated(c) => *c,
        }
    }

    pub const fn join(self, other: Self) -> Self {
        Self::Accumulated(self.cost() + other.cost())
    }
}

impl<S> Interpreter<S> {
    /// Calculate the gas cost for the current runtime state.
    ///
    /// This function will access register values directly without any check.
    /// This means it must be protected by executor validators such as
    /// `is_valid_register_triple_alu` - otherwise it may panic.
    ///
    /// The checks are intentionally not performed so the gas cost can be as
    /// optimal as possible
    pub const fn gas_cost(&self, op: &Opcode) -> Word {
        use Opcode::*;

        match op {
            ADD(_, _, _) | EXP(_, _, _) | MUL(_, _, _) | SLL(_, _, _) | SRL(_, _, _) | SUB(_, _, _) => {
                GasUnit::Arithmetic(1)
                    .join(GasUnit::RegisterRead(3))
                    .join(GasUnit::RegisterWrite(3))
            }

            MLOG(_, _, _) | MROO(_, _, _) => GasUnit::ArithmeticExpensive(1)
                .join(GasUnit::RegisterRead(3))
                .join(GasUnit::RegisterWrite(3)),

            ADDI(_, _, _) | EXPI(_, _, _) | MULI(_, _, _) | SLLI(_, _, _) | SRLI(_, _, _) | SUBI(_, _, _) => {
                GasUnit::Arithmetic(1)
                    .join(GasUnit::RegisterRead(2))
                    .join(GasUnit::RegisterWrite(3))
            }

            AND(_, _, _) | EQ(_, _, _) | GT(_, _, _) | OR(_, _, _) | XOR(_, _, _) => {
                GasUnit::RegisterRead(3).join(GasUnit::RegisterWrite(3))
            }

            ANDI(_, _, _) | MOVE(_, _) | ORI(_, _, _) | XORI(_, _, _) => {
                GasUnit::RegisterRead(2).join(GasUnit::RegisterWrite(3))
            }

            DIV(_, _, _) | MOD(_, _, _) => GasUnit::RegisterRead(3)
                .join(GasUnit::Arithmetic(1))
                .join(GasUnit::Branching(1)),

            DIVI(_, _, _) | MODI(_, _, _) => GasUnit::RegisterRead(2)
                .join(GasUnit::Arithmetic(1))
                .join(GasUnit::Branching(1)),

            NOOP | JI(_) => GasUnit::Atom(1),

            JNEI(_, _, _) => GasUnit::RegisterRead(2),

            ALOC(_) => GasUnit::RegisterRead(1)
                .join(GasUnit::Arithmetic(1))
                .join(GasUnit::Branching(1))
                .join(GasUnit::RegisterWrite(1)),

            LB(_, _, _) => GasUnit::RegisterRead(2)
                .join(GasUnit::Arithmetic(1))
                .join(GasUnit::Branching(1))
                .join(GasUnit::RegisterWrite(1)),

            MCL(_, rb) => GasUnit::RegisterRead(2)
                .join(GasUnit::Arithmetic(1))
                .join(GasUnit::Branching(1))
                .join(GasUnit::MemoryOwnership(1))
                .join(GasUnit::MemoryWrite(self.registers[*rb])),

            _ => GasUnit::Undefined,
        }
        .cost()
    }

    // TODO enable const flag
    // https://github.com/rust-lang/rust/issues/57349
    pub fn gas_charge(&mut self, op: &Opcode) -> Result<(), ExecuteError> {
        let cost = !self.is_predicate() as Word * self.gas_cost(op);

        if cost > self.registers[REG_CGAS] {
            self.registers[REG_GGAS] -= self.registers[REG_CGAS];
            self.registers[REG_CGAS] = 0;

            Err(ExecuteError::OutOfGas)
        } else {
            self.registers[REG_CGAS] -= cost;

            Ok(())
        }
    }
}
