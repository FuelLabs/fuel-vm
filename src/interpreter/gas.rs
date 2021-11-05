use super::Interpreter;
use crate::consts::*;
use crate::error::InterpreterError;
use crate::gas::GasUnit;

use fuel_asm::OpcodeRepr;
use fuel_types::Word;

pub mod consts;

impl<S> Interpreter<S> {
    pub(crate) const fn gas_cost_const(op: OpcodeRepr) -> Word {
        use OpcodeRepr::*;

        match op {
            ADD | EXP | MUL | SLL | SRL | SUB | ADDI | EXPI | MULI | SLLI | SRLI | SUBI => {
                GasUnit::Arithmetic(1).join(GasUnit::RegisterWrite(3))
            }

            MLOG | MROO => GasUnit::ArithmeticExpensive(1).join(GasUnit::RegisterWrite(3)),

            AND | EQ | GT | LT | OR | XOR | NOT | ANDI | MOVE | ORI | XORI => GasUnit::RegisterWrite(3),

            DIV | MOD | DIVI | MODI => GasUnit::Arithmetic(1).join(GasUnit::Branching(1)),

            NOOP | JI | JNEI => GasUnit::Atom(1),

            ALOC | LB => GasUnit::Arithmetic(1)
                .join(GasUnit::Branching(1))
                .join(GasUnit::RegisterWrite(1)),

            // TODO Compile-time panic didn't land in stable yet
            // https://github.com/rust-lang/rust/issues/51999
            _ => loop {}, //panic!("Opcode is not gas constant"),
        }
        .cost()
    }

    /// Return the constant term of a variable gas instruction
    // TODO Rust support for const fn pointers didn't land in stable yet
    // This fn should return both the base and the variable fn
    // https://github.com/rust-lang/rust/issues/57563
    pub(crate) const fn gas_cost_monad_base(op: OpcodeRepr) -> Word {
        use OpcodeRepr::*;

        match op {
            MCL | MCLI => GasUnit::Arithmetic(1).join(GasUnit::MemoryOwnership(1)),

            MCP | MCPI => GasUnit::Arithmetic(2).join(GasUnit::MemoryOwnership(1)),

            // TODO Compile-time panic didn't land in stable yet
            // https://github.com/rust-lang/rust/issues/51999
            _ => loop {}, //panic!("Opcode is not variable gas"),
        }
        .cost()
    }

    pub(crate) fn gas_charge_monad(&mut self, monad: fn(Word) -> Word, arg: Word) -> Result<(), InterpreterError> {
        self.gas_charge(monad(arg))
    }

    pub(crate) fn gas_charge(&mut self, gas: Word) -> Result<(), InterpreterError> {
        let gas = !self.is_predicate() as Word * gas;

        if gas > self.registers[REG_CGAS] {
            self.registers[REG_GGAS] -= self.registers[REG_CGAS];
            self.registers[REG_CGAS] = 0;

            Err(InterpreterError::OutOfGas)
        } else {
            self.registers[REG_CGAS] -= gas;

            Ok(())
        }
    }
}
