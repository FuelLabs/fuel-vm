use super::Interpreter;
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_types::bytes::SizedBytes;
use fuel_types::{RegisterId, Word};

impl<S> Interpreter<S> {
    pub(crate) fn transaction_input_length(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;
        self.registers[ra] = self
            .tx
            .inputs()
            .get(b as usize)
            .ok_or(PanicReason::InputNotFound)
            .map(|input| input.serialized_size() as Word)?;

        self.inc_pc()
    }

    pub(crate) fn transaction_input_start(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;
        self.registers[ra] =
            (self.tx_offset() + self.tx.input_offset(b as usize).ok_or(PanicReason::InputNotFound)?) as Word;

        self.inc_pc()
    }

    pub(crate) fn transaction_output_length(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;
        self.registers[ra] = self
            .tx
            .outputs()
            .get(b as usize)
            .ok_or(PanicReason::OutputNotFound)
            .map(|output| output.serialized_size() as Word)?;

        self.inc_pc()
    }

    pub(crate) fn transaction_output_start(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;
        self.registers[ra] =
            (self.tx_offset() + self.tx.output_offset(b as usize).ok_or(PanicReason::OutputNotFound)?) as Word;

        self.inc_pc()
    }

    pub(crate) fn transaction_witness_length(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;
        self.registers[ra] = self
            .tx
            .witnesses()
            .get(b as usize)
            .ok_or(PanicReason::WitnessNotFound)
            .map(|witness| witness.serialized_size() as Word)?;

        self.inc_pc()
    }

    pub(crate) fn transaction_witness_start(&mut self, ra: RegisterId, b: Word) -> Result<(), RuntimeError> {
        Self::is_register_writable(ra)?;
        self.registers[ra] =
            (self.tx_offset() + self.tx.witness_offset(b as usize).ok_or(PanicReason::WitnessNotFound)?) as Word;

        self.inc_pc()
    }
}
