use alloc::vec::Vec;
use fuel_types::Bytes32;

#[cfg(feature = "std")]
use crate::{field, UniqueIdentifier};

/// Entity support metadata computation to cache results.
pub trait Cacheable {
    /// The cache is already computed.
    ///
    /// # Note: `true` doesn't mean that the cache is actual.
    fn is_computed(&self) -> bool;

    /// Computes the cache for the entity.
    fn precompute(&mut self);
}

#[cfg(feature = "std")]
impl Cacheable for super::Transaction {
    fn is_computed(&self) -> bool {
        match self {
            Self::Script(script) => script.is_computed(),
            Self::Create(create) => create.is_computed(),
            Self::Mint(mint) => mint.is_computed(),
        }
    }

    fn precompute(&mut self) {
        match self {
            Self::Script(script) => script.precompute(),
            Self::Create(create) => create.precompute(),
            Self::Mint(mint) => mint.precompute(),
        }
    }
}

/// Common metadata for `Script` and `Create` transactions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CommonMetadata {
    pub id: Bytes32,
    pub inputs_offset: usize,
    pub inputs_offset_at: Vec<usize>,
    pub inputs_predicate_offset_at: Vec<Option<(usize, usize)>>,
    pub outputs_offset: usize,
    pub outputs_offset_at: Vec<usize>,
    pub witnesses_offset: usize,
    pub witnesses_offset_at: Vec<usize>,
    pub serialized_size: usize,
}

#[cfg(feature = "std")]
impl CommonMetadata {
    /// Computes the `Metadata` for the `tx` transaction.
    pub fn compute<Tx>(tx: &Tx) -> Self
    where
        Tx: UniqueIdentifier,
        Tx: field::Inputs,
        Tx: field::Outputs,
        Tx: field::Witnesses,
        Tx: fuel_types::bytes::SizedBytes,
    {
        use fuel_types::bytes::SizedBytes;
        use itertools::Itertools;

        let id = tx.id();

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
                offset += input.serialized_size();
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
                offset += output.serialized_size();
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
                offset += witness.serialized_size();
                i
            })
            .collect_vec();
        let serialized_size = offset;

        #[cfg(feature = "internals")]
        assert_eq!(serialized_size, tx.serialized_size());

        Self {
            id,
            inputs_offset,
            inputs_offset_at,
            inputs_predicate_offset_at,
            outputs_offset,
            outputs_offset_at,
            witnesses_offset,
            witnesses_offset_at,
            serialized_size,
        }
    }
}
