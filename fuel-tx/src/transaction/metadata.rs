use alloc::vec::Vec;
use fuel_types::{
    canonical::Serialize,
    Bytes32,
    ChainId,
};

use crate::{
    field,
    UniqueIdentifier,
    ValidityError,
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
        }
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), ValidityError> {
        match self {
            Self::Script(tx) => tx.precompute(chain_id),
            Self::Create(tx) => tx.precompute(chain_id),
            Self::Mint(tx) => tx.precompute(chain_id),
            Self::Upgrade(tx) => tx.precompute(chain_id),
            Self::Upload(tx) => tx.precompute(chain_id),
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
}

impl CommonMetadata {
    /// Computes the `Metadata` for the `tx` transaction.
    pub fn compute<Tx>(tx: &Tx, chain_id: &ChainId) -> Self
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

        let mut offset = tx.inputs_offset();
        let inputs_offset = offset;
        let inputs_offset_at = tx
            .inputs()
            .iter()
            .map(|input| {
                let i = offset;
                offset = offset.saturating_add(input.size());
                i
            })
            .collect_vec();

        let outputs_offset = offset;
        #[cfg(feature = "internals")]
        assert_eq!(outputs_offset, tx.outputs_offset());

        let outputs_offset_at = tx
            .outputs()
            .iter()
            .map(|output| {
                let i = offset;
                offset = offset.saturating_add(output.size());
                i
            })
            .collect_vec();

        let witnesses_offset = offset;
        #[cfg(feature = "internals")]
        assert_eq!(witnesses_offset, tx.witnesses_offset());

        let witnesses_offset_at = tx
            .witnesses()
            .iter()
            .map(|witness| {
                let i = offset;
                offset = offset.saturating_add(witness.size());
                i
            })
            .collect_vec();

        Self {
            id,
            inputs_offset,
            inputs_offset_at,
            inputs_predicate_offset_at,
            outputs_offset,
            outputs_offset_at,
            witnesses_offset,
            witnesses_offset_at,
        }
    }
}
