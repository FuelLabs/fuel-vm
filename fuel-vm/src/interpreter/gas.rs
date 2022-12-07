use super::Interpreter;
use crate::arith;
use crate::consts::*;
use crate::error::RuntimeError;
use crate::gas::GasUnit;

use fuel_asm::{Opcode, PanicReason};
use fuel_types::Word;

pub mod consts;

impl<S, Tx> Interpreter<S, Tx> {
    pub(crate) fn remaining_gas(&self) -> Word {
        self.registers[REG_GGAS]
    }

    /// Maps [`Opcode`] to a [`GasUnit`] price.
    ///
    /// # Panic
    /// This function panics for codes that do not have an assigned price.
    /// This function should only be used in a const context to avoid
    /// runtime panics.
    pub(crate) const fn gas_cost_const(op: Opcode) -> Word {
        use GasUnit::*;
        use Opcode::*;

        match op {
            ADD | MUL | SLL | SRL | SUB | ADDI | MULI | SLLI | SRLI | SUBI => Arithmetic(1).join(RegisterWrite(3)),

            EXP | EXPI | MLOG => ArithmeticExpensive(1).join(RegisterWrite(3)),

            MROO => ArithmeticExpensive(1).join(Branching(1)).join(RegisterWrite(3)),

            AND | EQ | OR | XOR | NOT | ANDI | ORI | XORI => Arithmetic(1).join(RegisterWrite(3)),

            GT | LT | MOVE | MOVI => RegisterWrite(3),

            DIV | MOD | DIVI | MODI | JMP | JI | JNE | JNEI | JNZI => Arithmetic(1).join(Branching(1)),

            NOOP => Atom(1),

            ALOC | LB => Arithmetic(1).join(Branching(1)).join(RegisterWrite(1)),

            RET | RVRT => Arithmetic(3).join(RegisterWrite(VM_REGISTER_COUNT as Word)),
            RETD => Arithmetic(3)
                .join(RegisterWrite(VM_REGISTER_COUNT as Word))
                .join(Hash(1)),

            // Output message size
            SMO => Arithmetic(3)
                .join(Branching(2))
                .join(MemoryWrite(3 * 48))
                .join(StorageWriteWord(1)),

            CFEI | CFSI => Arithmetic(1).join(RegisterWrite(1)).join(Branching(1)),

            LW => Arithmetic(1).join(Branching(1)).join(RegisterWrite(8)),

            SB => Arithmetic(1).join(MemoryOwnership(1)).join(MemoryWrite(1)),

            SW => Arithmetic(1).join(MemoryOwnership(1)).join(MemoryWrite(8)),

            BAL => Arithmetic(2).join(Branching(1)).join(StorageReadTree(1)),

            BHEI => Branching(1).join(StorageWriteWord(1)),

            BHSH => Branching(1)
                .join(StorageReadTree(1))
                .join(MemoryOwnership(1))
                .join(MemoryWrite(32)),

            BURN => Branching(1)
                .join(Arithmetic(1))
                .join(StorageReadTree(1))
                .join(StorageWriteTree(1)),

            CALL => Arithmetic(5).join(Branching(2)).join(MemoryWrite(64)),

            CB => Branching(1).join(StorageReadTree(1)).join(MemoryWrite(32)),

            CROO => Branching(2)
                .join(StorageReadTree(1))
                .join(MemoryOwnership(1))
                .join(MemoryWrite(32)),

            CSIZ => Arithmetic(1).join(Branching(2)).join(StorageReadTree(1)),

            LDC => Arithmetic(6)
                .join(Branching(3))
                .join(MemoryOwnership(1))
                .join(MemoryWrite(64)),

            LOG => MemoryWrite(32),

            LOGD => MemoryWrite(32).join(Hash(1)),

            MINT => Arithmetic(1).join(Branching(2)).join(StorageWriteTree(2)),

            SCWQ => Arithmetic(1)
                .join(Branching(2))
                .join(StorageWriteTree(4))
                .join(RegisterWrite(1)),

            SRW => Arithmetic(1)
                .join(Branching(2))
                .join(StorageReadTree(1))
                .join(RegisterWrite(1)),

            SRWQ => Arithmetic(1)
                .join(Branching(2))
                .join(StorageReadTree(4))
                .join(RegisterWrite(1)),

            SWW => Arithmetic(1)
                .join(Branching(2))
                .join(StorageWriteTree(1))
                .join(RegisterWrite(1)),

            SWWQ => Arithmetic(1)
                .join(Branching(2))
                .join(StorageWriteTree(4))
                .join(RegisterWrite(1)),

            TIME => Arithmetic(1).join(Branching(2)).join(StorageReadTree(4)),

            ECR => Arithmetic(2).join(Branching(2)).join(Recover(1)).join(MemoryWrite(64)),

            K256 | S256 => Arithmetic(2).join(Branching(2)).join(Hash(1)).join(MemoryWrite(32)),

            FLAG => RegisterWrite(1),

            GM => Arithmetic(3).join(Branching(3)),

            GTF => Arithmetic(3).join(Branching(32)),

            TR => Arithmetic(3).join(Branching(6)).join(StorageWriteWord(2)),

            TRO => Arithmetic(3).join(Branching(4)).join(StorageWriteWord(2)),

            _ => panic!("Opcode is not gas constant"),
        }
        .join(Atom(1))
        .cost()
    }

    /// Return the constant term of a variable gas instruction
    // TODO Rust support for const fn pointers didn't land in stable yet
    // This fn should return both the base and the variable fn
    // https://github.com/rust-lang/rust/issues/57563
    pub(crate) const fn gas_cost_monad_base(op: Opcode) -> Word {
        use GasUnit::*;
        use Opcode::*;

        match op {
            MCL | MCLI => Arithmetic(1).join(MemoryOwnership(1)),

            MCP | MCPI => Arithmetic(2).join(MemoryOwnership(1)),

            MEQ => Arithmetic(3).join(Branching(2)).join(RegisterWrite(1)),

            CCP => Branching(2).join(StorageReadTree(1)),

            _ => panic!("Opcode is not variable gas"),
        }
        .cost()
    }

    pub(crate) fn gas_charge_monad<F>(&mut self, monad: F, arg: Word) -> Result<(), RuntimeError>
    where
        F: FnOnce(Word) -> Word,
    {
        self.gas_charge(monad(arg))
    }

    pub(crate) fn gas_charge(&mut self, gas: Word) -> Result<(), RuntimeError> {
        let gas = !self.is_predicate() as Word * gas;

        #[cfg(feature = "profile-coverage")]
        {
            let location = self.current_location();
            self.profiler.data_mut().coverage_mut().set(location);
        }

        #[cfg(feature = "profile-gas")]
        {
            let gas_use = gas.min(self.registers[REG_CGAS]);
            let location = self.current_location();
            self.profiler.data_mut().gas_mut().add(location, gas_use);
        }

        if gas > self.registers[REG_CGAS] {
            self.registers[REG_GGAS] = arith::sub_word(self.registers[REG_GGAS], self.registers[REG_CGAS])?;
            self.registers[REG_CGAS] = 0;

            Err(PanicReason::OutOfGas.into())
        } else {
            self.registers[REG_CGAS] = arith::sub_word(self.registers[REG_CGAS], gas)?;
            self.registers[REG_GGAS] = arith::sub_word(self.registers[REG_GGAS], gas)?;

            Ok(())
        }
    }
}
