use crate::{Input, Output, Transaction, Witness};

use alloc::vec::Vec;

#[cfg(feature = "internals")]
impl Transaction {
    /// Append an input to the transaction
    pub fn add_input(&mut self, input: Input) {
        self._add_input(input);
    }

    /// Append an output to the transaction
    pub fn add_output(&mut self, output: Output) {
        self._add_output(output);
    }

    /// Append a witness to the transaction
    pub fn add_witness(&mut self, witness: Witness) {
        self._add_witness(witness);
    }

    /// Set the transaction script, if script variant. Return none otherwise.
    pub fn set_script(&mut self, script: Vec<u8>) -> Option<()> {
        self._set_script(script)
    }
}

impl Transaction {
    pub(crate) fn _add_input(&mut self, input: Input) {
        match self {
            Self::Script { inputs, .. } => inputs.push(input),
            Self::Create { inputs, .. } => inputs.push(input),
        }
    }

    pub(crate) fn _add_output(&mut self, output: Output) {
        match self {
            Self::Script { outputs, .. } => outputs.push(output),
            Self::Create { outputs, .. } => outputs.push(output),
        }
    }

    pub(crate) fn _add_witness(&mut self, witness: Witness) {
        match self {
            Self::Script { witnesses, .. } => witnesses.push(witness),
            Self::Create { witnesses, .. } => witnesses.push(witness),
        }
    }

    pub(crate) fn _set_script(&mut self, _script: Vec<u8>) -> Option<()> {
        match self {
            Self::Script { script, .. } => Some(*script = _script),
            Self::Create { .. } => None,
        }
    }
}
