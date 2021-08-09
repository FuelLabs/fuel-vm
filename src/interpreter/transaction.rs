use super::{ExecuteError, Interpreter, RegisterId};

use fuel_asm::Word;
use fuel_tx::bytes::SizedBytes;

impl<S> Interpreter<S> {
    pub(crate) fn transaction_input_length(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        self.registers[ra] = self
            .tx
            .inputs()
            .get(b as usize)
            .ok_or(ExecuteError::InputNotFound)
            .map(|input| input.serialized_size() as Word)?;

        Ok(())
    }

    pub(crate) fn transaction_input_start(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        self.registers[ra] =
            (Self::tx_mem_address() + self.tx.input_offset(b as usize).ok_or(ExecuteError::InputNotFound)?) as Word;

        Ok(())
    }

    pub(crate) fn transaction_output_length(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        self.registers[ra] = self
            .tx
            .outputs()
            .get(b as usize)
            .ok_or(ExecuteError::OutputNotFound)
            .map(|output| output.serialized_size() as Word)?;

        Ok(())
    }

    pub(crate) fn transaction_output_start(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        self.registers[ra] =
            (Self::tx_mem_address() + self.tx.output_offset(b as usize).ok_or(ExecuteError::OutputNotFound)?) as Word;

        Ok(())
    }

    pub(crate) fn transaction_witness_length(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        self.registers[ra] = self
            .tx
            .witnesses()
            .get(b as usize)
            .ok_or(ExecuteError::OutputNotFound)
            .map(|witness| witness.serialized_size() as Word)?;

        Ok(())
    }

    pub(crate) fn transaction_witness_start(&mut self, ra: RegisterId, b: Word) -> Result<(), ExecuteError> {
        self.registers[ra] = (Self::tx_mem_address()
            + self
                .tx
                .witness_offset(b as usize)
                .ok_or(ExecuteError::WitnessNotFound)?) as Word;

        Ok(())
    }
}
