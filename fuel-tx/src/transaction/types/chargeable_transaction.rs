use crate::{
    ConsensusParameters,
    Input,
    Output,
    UniqueIdentifier,
    ValidityError,
    Witness,
    field::ChargeableBody,
    policies::Policies,
    transaction::{
        Chargeable,
        field::{
            Inputs,
            Outputs,
            Policies as PoliciesField,
            Witnesses,
        },
        id::PrepareSign,
        metadata::CommonMetadata,
        validity::{
            FormatValidityChecks,
            check_common_part,
        },
    },
};
use educe::Educe;
use fuel_types::{
    BlockHeight,
    Bytes32,
    ChainId,
    bytes,
    canonical::Serialize,
};
use hashbrown::HashMap;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChargeableMetadata<Body> {
    pub common: CommonMetadata,
    pub body: Body,
}

#[cfg(feature = "da-compression")]
use fuel_compression::Compressible;

#[cfg(feature = "da-compression")]
pub trait BodyConstraints:
    for<'a> Compressible<
    Compressed: core::fmt::Debug
                    + PartialEq
                    + Clone
                    + serde::Serialize
                    + serde::Deserialize<'a>,
>
{
}

#[cfg(feature = "da-compression")]
impl<T> BodyConstraints for T where
    T: for<'a> Compressible<
        Compressed: core::fmt::Debug
                        + PartialEq
                        + Clone
                        + serde::Serialize
                        + serde::Deserialize<'a>,
    >
{
}

#[cfg(not(feature = "da-compression"))]
pub trait BodyConstraints {}
#[cfg(not(feature = "da-compression"))]
impl<T> BodyConstraints for T {}

#[derive(Clone, Educe)]
#[educe(Eq, PartialEq, Hash, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
#[cfg_attr(feature = "da-compression", compress(discard(MetadataBody)))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct ChargeableTransaction<Body, MetadataBody>
where
    Body: BodyConstraints,
{
    pub(crate) body: Body,
    pub(crate) policies: Policies,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
    pub(crate) witnesses: Vec<Witness>,
    #[serde(skip)]
    #[cfg_attr(feature = "da-compression", compress(skip))]
    #[educe(PartialEq(ignore))]
    #[educe(Hash(ignore))]
    #[canonical(skip)]
    pub(crate) metadata: Option<ChargeableMetadata<MetadataBody>>,
}

impl<Body, MetadataBody> Default for ChargeableTransaction<Body, MetadataBody>
where
    Body: BodyConstraints + Default,
{
    fn default() -> Self {
        Self {
            body: Default::default(),
            policies: Policies::new()
                .with_maturity(0.into())
                .with_witness_limit(10000),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        }
    }
}

impl<Body, MetadataBody> ChargeableTransaction<Body, MetadataBody>
where
    Body: BodyConstraints,
{
    pub fn metadata(&self) -> &Option<ChargeableMetadata<MetadataBody>> {
        &self.metadata
    }
}

impl<Body, MetadataBody> PrepareSign for ChargeableTransaction<Body, MetadataBody>
where
    Body: BodyConstraints + PrepareSign,
    Self: ChargeableBody<Body>,
{
    fn prepare_sign(&mut self) {
        self.body.prepare_sign();
        self.inputs_mut().iter_mut().for_each(Input::prepare_sign);
        self.outputs_mut().iter_mut().for_each(Output::prepare_sign);
    }
}

impl<Body, MetadataBody> UniqueIdentifier for ChargeableTransaction<Body, MetadataBody>
where
    Body: BodyConstraints + PrepareSign,
    Self: Clone,
    Self: ChargeableBody<Body>,
    Self: fuel_types::canonical::Serialize,
{
    fn id(&self, chain_id: &ChainId) -> Bytes32 {
        if let Some(id) = self.cached_id() {
            return id;
        }

        let mut clone = self.clone();

        // Empties fields that should be zero during the signing.
        clone.prepare_sign();
        clone.witnesses_mut().clear();

        crate::transaction::compute_transaction_id(chain_id, &mut clone)
    }

    fn cached_id(&self) -> Option<Bytes32> {
        self.metadata.as_ref().map(|m| m.common.id)
    }
}

pub(crate) trait UniqueFormatValidityChecks {
    /// Checks unique rules inherited from the `Body` for chargeable transaction.
    fn check_unique_rules(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError>;

    fn verify_blob_id(&self) -> Result<(), ValidityError>;
    fn verify_inputs(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError>;
    fn verify_outputs(
        &self,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError>;
}

impl<Body, MetadataBody> FormatValidityChecks
    for ChargeableTransaction<Body, MetadataBody>
where
    Body: BodyConstraints + PrepareSign,
    Self: Clone,
    Self: ChargeableBody<Body>,
    Self: fuel_types::canonical::Serialize,
    Self: Chargeable,
    Self: UniqueFormatValidityChecks,
{
    fn check_signatures(&self, chain_id: &ChainId) -> Result<(), ValidityError> {
        let id = self.id(chain_id);

        // There will be at most len(witnesses) signatures to cache
        let mut recovery_cache = Some(HashMap::with_capacity(self.witnesses().len()));

        self.inputs()
            .iter()
            .enumerate()
            .try_for_each(|(index, input)| {
                input.check_signature(index, &id, self.witnesses(), &mut recovery_cache)
            })?;

        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        check_common_part(self, block_height, consensus_params)?;
        self.check_unique_rules(consensus_params)?;

        Ok(())
    }
}

mod field {
    use super::*;
    use crate::field::ChargeableBody;

    impl<Body, MetadataBody> PoliciesField for ChargeableTransaction<Body, MetadataBody>
    where
        Body: BodyConstraints,
        Self: ChargeableBody<Body>,
    {
        #[inline(always)]
        fn policies(&self) -> &Policies {
            &self.policies
        }

        #[inline(always)]
        fn policies_mut(&mut self) -> &mut Policies {
            &mut self.policies
        }

        #[inline(always)]
        fn policies_offset(&self) -> usize {
            self.body_offset_end()
        }
    }

    impl<Body, MetadataBody> Inputs for ChargeableTransaction<Body, MetadataBody>
    where
        Body: BodyConstraints,
        Self: ChargeableBody<Body>,
    {
        #[inline(always)]
        fn inputs(&self) -> &Vec<Input> {
            &self.inputs
        }

        #[inline(always)]
        fn inputs_mut(&mut self) -> &mut Vec<Input> {
            &mut self.inputs
        }

        #[inline(always)]
        fn inputs_offset(&self) -> usize {
            if let Some(ChargeableMetadata {
                common: CommonMetadata { inputs_offset, .. },
                ..
            }) = &self.metadata
            {
                return *inputs_offset;
            }

            self.policies_offset()
                .saturating_add(self.policies.size_dynamic())
        }

        #[inline(always)]
        fn inputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(ChargeableMetadata {
                common:
                    CommonMetadata {
                        inputs_offset_at, ..
                    },
                ..
            }) = &self.metadata
            {
                return inputs_offset_at.get(idx).cloned();
            }

            if idx < self.inputs.len() {
                Some(
                    self.inputs_offset().saturating_add(
                        self.inputs()
                            .iter()
                            .take(idx)
                            .map(|i| i.size())
                            .reduce(usize::saturating_add)
                            .unwrap_or_default(),
                    ),
                )
            } else {
                None
            }
        }

        #[inline(always)]
        fn inputs_predicate_offset_at(&self, idx: usize) -> Option<(usize, usize)> {
            if let Some(ChargeableMetadata {
                common:
                    CommonMetadata {
                        inputs_predicate_offset_at,
                        ..
                    },
                ..
            }) = &self.metadata
            {
                return inputs_predicate_offset_at.get(idx).cloned().unwrap_or(None);
            }

            self.inputs().get(idx).and_then(|input| {
                input
                    .predicate_offset()
                    .and_then(|predicate| {
                        self.inputs_offset_at(idx)
                            .map(|inputs| inputs.saturating_add(predicate))
                    })
                    .zip(input.predicate_len().and_then(bytes::padded_len_usize))
            })
        }
    }

    impl<Body, MetadataBody> Outputs for ChargeableTransaction<Body, MetadataBody>
    where
        Body: BodyConstraints,
        Self: ChargeableBody<Body>,
    {
        #[inline(always)]
        fn outputs(&self) -> &Vec<Output> {
            &self.outputs
        }

        #[inline(always)]
        fn outputs_mut(&mut self) -> &mut Vec<Output> {
            &mut self.outputs
        }

        #[inline(always)]
        fn outputs_offset(&self) -> usize {
            if let Some(ChargeableMetadata {
                common: CommonMetadata { outputs_offset, .. },
                ..
            }) = &self.metadata
            {
                return *outputs_offset;
            }

            self.inputs_offset().saturating_add(
                self.inputs()
                    .iter()
                    .map(|i| i.size())
                    .reduce(usize::saturating_add)
                    .unwrap_or_default(),
            )
        }

        #[inline(always)]
        fn outputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(ChargeableMetadata {
                common:
                    CommonMetadata {
                        outputs_offset_at, ..
                    },
                ..
            }) = &self.metadata
            {
                return outputs_offset_at.get(idx).cloned();
            }

            if idx < self.outputs.len() {
                Some(
                    self.outputs_offset().saturating_add(
                        self.outputs()
                            .iter()
                            .take(idx)
                            .map(|i| i.size())
                            .reduce(usize::saturating_add)
                            .unwrap_or_default(),
                    ),
                )
            } else {
                None
            }
        }
    }

    impl<Body, MetadataBody> Witnesses for ChargeableTransaction<Body, MetadataBody>
    where
        Body: BodyConstraints,
        Self: ChargeableBody<Body>,
    {
        #[inline(always)]
        fn witnesses(&self) -> &Vec<Witness> {
            &self.witnesses
        }

        #[inline(always)]
        fn witnesses_mut(&mut self) -> &mut Vec<Witness> {
            &mut self.witnesses
        }

        #[inline(always)]
        fn witnesses_offset(&self) -> usize {
            if let Some(ChargeableMetadata {
                common:
                    CommonMetadata {
                        witnesses_offset, ..
                    },
                ..
            }) = &self.metadata
            {
                return *witnesses_offset;
            }

            self.outputs_offset().saturating_add(
                self.outputs()
                    .iter()
                    .map(|i| i.size())
                    .reduce(usize::saturating_add)
                    .unwrap_or_default(),
            )
        }

        #[inline(always)]
        fn witnesses_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(ChargeableMetadata {
                common:
                    CommonMetadata {
                        witnesses_offset_at,
                        ..
                    },
                ..
            }) = &self.metadata
            {
                return witnesses_offset_at.get(idx).cloned();
            }

            if idx < self.witnesses.len() {
                Some(
                    self.witnesses_offset().saturating_add(
                        self.witnesses()
                            .iter()
                            .take(idx)
                            .map(|i| i.size())
                            .reduce(usize::saturating_add)
                            .unwrap_or_default(),
                    ),
                )
            } else {
                None
            }
        }
    }
}
