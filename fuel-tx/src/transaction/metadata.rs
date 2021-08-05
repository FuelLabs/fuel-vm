use super::{Bytes32, Transaction};
use crate::bytes::SizedBytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub struct Metadata {
    id: Bytes32,
    script_data_offset: Option<usize>,
    input_coin_predicate_offset: Vec<Option<usize>>,
    inputs_offset: Vec<usize>,
    outputs_offset: Vec<usize>,
    witnesses_offset: Vec<usize>,
}

impl Metadata {
    pub const fn new(
        id: Bytes32,
        script_data_offset: Option<usize>,
        input_coin_predicate_offset: Vec<Option<usize>>,
        inputs_offset: Vec<usize>,
        outputs_offset: Vec<usize>,
        witnesses_offset: Vec<usize>,
    ) -> Self {
        Self {
            id,
            script_data_offset,
            input_coin_predicate_offset,
            inputs_offset,
            outputs_offset,
            witnesses_offset,
        }
    }

    pub const fn id(&self) -> &Bytes32 {
        &self.id
    }

    pub fn script_data_offset(&self) -> Option<usize> {
        self.script_data_offset
    }

    pub fn input_coin_predicate_offset(&self, index: usize) -> Option<usize> {
        self.input_coin_predicate_offset
            .get(index)
            .copied()
            .flatten()
    }

    pub fn inputs_offset(&self, index: usize) -> Option<usize> {
        self.inputs_offset.get(index).copied()
    }

    pub fn outputs_offset(&self, index: usize) -> Option<usize> {
        self.outputs_offset.get(index).copied()
    }

    pub fn witnesses_offset(&self, index: usize) -> Option<usize> {
        self.witnesses_offset.get(index).copied()
    }
}

impl Transaction {
    fn metadata_mut(&mut self) -> &mut Option<Metadata> {
        match self {
            Self::Script { metadata, .. } => metadata,
            Self::Create { metadata, .. } => metadata,
        }
    }

    pub fn precompute_metadata(&mut self) {
        let id = self._id();

        let script_data_offset = self._script_data_offset();
        let input_coin_predicate_offset = self
            .inputs()
            .iter()
            .enumerate()
            .map(|(i, _)| self._input_coin_predicate_offset(i))
            .collect();

        let offset = self.inputs_offset();
        let inputs_offset = self
            .inputs()
            .iter()
            .scan(offset, |offset, input| {
                let i = *offset;
                *offset += input.serialized_size();

                Some(i)
            })
            .collect();

        let offset = self.outputs_offset();
        let outputs_offset = self
            .outputs()
            .iter()
            .scan(offset, |offset, output| {
                let i = *offset;
                *offset += output.serialized_size();

                Some(i)
            })
            .collect();

        let offset = self.witnesses_offset();
        let witnesses_offset = self
            .witnesses()
            .iter()
            .scan(offset, |offset, witness| {
                let i = *offset;
                *offset += witness.serialized_size();

                Some(i)
            })
            .collect();

        let metadata = Metadata::new(
            id,
            script_data_offset,
            input_coin_predicate_offset,
            inputs_offset,
            outputs_offset,
            witnesses_offset,
        );

        self.metadata_mut().replace(metadata);
    }
}
