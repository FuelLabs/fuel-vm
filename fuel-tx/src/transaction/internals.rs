use crate::{Input, Output, Transaction, Witness};

use fuel_asm::Word;
use itertools::Itertools;

use alloc::vec::Vec;
use core::hash::Hash;

// TODO https://github.com/FuelLabs/fuel-tx/issues/148
pub(crate) fn next_duplicate<U>(iter: impl Iterator<Item = U>) -> Option<U>
where
    U: PartialEq + Ord + Copy + Hash,
{
    #[cfg(not(feature = "std"))]
    return iter
        .sorted()
        .as_slice()
        .windows(2)
        .filter_map(|u| (u[0] == u[1]).then(|| u[0]))
        .next();

    #[cfg(feature = "std")]
    return iter.duplicates().next();
}

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

    /// Set the transaction bytecode, if create variant. Return none otherwise.
    pub fn set_bytecode(&mut self, bytecode: Witness) -> Option<()> {
        self._set_bytecode(bytecode)
    }

    pub fn inputs_mut(&mut self) -> &mut [Input] {
        self._inputs_mut()
    }

    pub fn outputs_mut(&mut self) -> &mut [Output] {
        self._outputs_mut()
    }

    pub fn witnesses_mut(&mut self) -> &mut [Witness] {
        self._witnesses_mut()
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
            Self::Script { script, .. } => {
                *script = _script;
                Some(())
            }
            Self::Create { .. } => None,
        }
    }

    pub(crate) fn _set_bytecode(&mut self, bytecode: Witness) -> Option<()> {
        match self {
            Self::Script { .. } => None,
            Self::Create {
                bytecode_length,
                bytecode_witness_index,
                witnesses,
                ..
            } => {
                *bytecode_length = (bytecode.as_ref().len() / 4) as Word;
                *bytecode_witness_index = witnesses.len() as u8;

                witnesses.push(bytecode);

                Some(())
            }
        }
    }
}
