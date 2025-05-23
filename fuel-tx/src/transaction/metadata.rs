use alloc::vec::Vec;
use fuel_types::{
    Bytes32,
    ChainId,
    canonical::Serialize,
};

use crate::{
    UniqueIdentifier,
    ValidityError,
    Witness,
    field,
};

/// Entity support metadata computation to cache results.
pub trait Cacheable {
    /// The cache is already computed.
    ///
    /// # Note: `true` doesn't mean that the cache is actual.
    fn is_computed(&self) -> bool;

    /// Computes the cache for the entity.
    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError>;
}

impl Cacheable for super::Transaction {
    fn is_computed(&self) -> bool {
        match self {
            Self::Script(tx) => tx.is_computed(),
            Self::Create(tx) => tx.is_computed(),
            Self::Mint(tx) => tx.is_computed(),
            Self::Upgrade(tx) => tx.is_computed(),
            Self::Upload(tx) => tx.is_computed(),
            Self::Blob(tx) => tx.is_computed(),
            #[cfg(feature = "chargeable-tx-v2")]
            Self::ScriptV2(tx) => tx.is_computed(),
        }
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        match self {
            Self::Script(tx) => tx.precompute(chain_id),
            Self::Create(tx) => tx.precompute(chain_id),
            Self::Mint(tx) => tx.precompute(chain_id),
            Self::Upgrade(tx) => tx.precompute(chain_id),
            Self::Upload(tx) => tx.precompute(chain_id),
            Self::Blob(tx) => tx.precompute(chain_id),
            #[cfg(feature = "chargeable-tx-v2")]
            Self::ScriptV2(tx) => tx.precompute(chain_id),
        }
    }
}

/// Common metadata for `Script` and `Create` transactions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommonMetadata {
    pub id: Bytes32,
    pub inputs_offset: usize,
    pub inputs_offset_at: Vec<usize>,
    pub inputs_predicate_offset_at: Vec<Option<(usize, usize)>>,
    pub outputs_offset: usize,
    pub outputs_offset_at: Vec<usize>,
    pub witnesses_offset: usize,
    pub witnesses_offset_at: Vec<usize>,
    #[cfg(feature = "chargeable-tx-v2")]
    pub static_witnesses_offset: usize,
    #[cfg(feature = "chargeable-tx-v2")]
    pub static_witnesses_offset_at: Vec<usize>,
}

impl CommonMetadata {
    /// Computes the `Metadata` for the `tx` transaction.
    /// Returns `None` if the transaction is invalid.
    pub fn compute<Tx>(tx: &Tx, chain_id: &ChainId) -> Result<Self, ValidityError>
    where
        Tx: UniqueIdentifier,
        Tx: field::Inputs,
        Tx: field::Outputs,
        Tx: field::Witnesses,
    {
        use itertools::Itertools;

        let id = tx.id(chain_id);

        let inputs_predicate_offset_at = tx
            .inputs()
            .iter()
            .enumerate()
            .map(|(i, _)| tx.inputs_predicate_offset_at(i))
            .collect_vec();

        let mut input_offset = tx.inputs_offset();
        let mut inputs_offset_at = Vec::with_capacity(tx.inputs().len());
        for (index, input) in tx.inputs().iter().enumerate() {
            let i = input_offset;
            input_offset = input_offset
                .checked_add(input.size())
                .ok_or(ValidityError::SerializedInputTooLarge { index })?;
            inputs_offset_at.push(i);
        }
        let _input_bytes = tx.inputs()[0].to_bytes();
        let _input_size = tx.inputs()[0].size();
        let mut _input_encoded = Vec::new();
        tx.inputs().encode_dynamic(&mut _input_encoded).unwrap();

        let mut output_offset = tx.outputs_offset();
        let mut outputs_offset_at = Vec::with_capacity(tx.outputs().len());
        for (index, output) in tx.outputs().iter().enumerate() {
            let i = output_offset;
            output_offset = output_offset
                .checked_add(output.size())
                .ok_or(ValidityError::SerializedOutputTooLarge { index })?;
            outputs_offset_at.push(i);
        }

        let _output_bytes = tx.outputs()[0].to_bytes();

        let mut witness_offset = tx.witnesses_offset();
        let mut witnesses_offset_at = Vec::with_capacity(tx.witnesses().len());
        for (index, witnesses) in tx.witnesses().iter().enumerate() {
            let i = witness_offset;
            witness_offset = witness_offset
                .checked_add(witnesses.size())
                .ok_or(ValidityError::SerializedWitnessTooLarge { index })?;
            witnesses_offset_at.push(i);
        }

        #[cfg(feature = "chargeable-tx-v2")]
        let mut static_witnesses_offset_at =
            Vec::with_capacity(tx.static_witnesses().len());
        #[cfg(feature = "chargeable-tx-v2")]
        {
            let mut static_witness_offset = tx.static_witnesses_offset();
            for (index, static_witness) in tx.static_witnesses().iter().enumerate() {
                let i = static_witness_offset;
                static_witness_offset = static_witness_offset
                    .checked_add(static_witness.size())
                    .ok_or(ValidityError::SerializedWitnessTooLarge { index })?;
                static_witnesses_offset_at.push(i);
            }
        }

        Ok(Self {
            id,
            inputs_offset: tx.inputs_offset(),
            inputs_offset_at,
            inputs_predicate_offset_at,
            outputs_offset: tx.outputs_offset(),
            outputs_offset_at,
            witnesses_offset: tx.witnesses_offset(),
            witnesses_offset_at,
            #[cfg(feature = "chargeable-tx-v2")]
            static_witnesses_offset: tx.static_witnesses_offset(),
            #[cfg(feature = "chargeable-tx-v2")]
            static_witnesses_offset_at,
        })
    }
}
