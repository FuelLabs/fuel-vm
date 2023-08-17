use crate::{
    transaction::{
        field::{
            Outputs,
            TxPointer as TxPointerField,
        },
        validity::FormatValidityChecks,
    },
    CheckError,
    ConsensusParameters,
    Output,
    TransactionRepr,
    TxPointer,
};
use derivative::Derivative;
use fuel_types::{
    bytes::{
        SizedBytes,
        WORD_SIZE,
    },
    mem_layout,
    BlockHeight,
    Bytes32,
    Word,
};

#[cfg(feature = "std")]
use fuel_types::ChainId;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MintMetadata {
    pub id: Bytes32,
    pub outputs_offset: usize,
    pub outputs_offset_at: Vec<usize>,
}

#[cfg(feature = "std")]
impl MintMetadata {
    fn compute<Tx>(tx: &Tx, chain_id: &ChainId) -> Self
    where
        Tx: crate::UniqueIdentifier,
        Tx: Outputs,
        Tx: SizedBytes,
    {
        use itertools::Itertools;

        let id = tx.id(chain_id);

        let mut offset = tx.outputs_offset();

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

        Self {
            id,
            outputs_offset,
            outputs_offset_at,
        }
    }
}

/// The definition of the `Mint` transaction from the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/transaction.md#transactionmint>
///
/// This transaction can be created by the block producer and included in the block only
/// by it.
#[derive(Default, Debug, Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Serialize, fuel_types::canonical::Deserialize)]
#[canonical(prefix = TransactionRepr::Mint)]
#[derivative(Eq, PartialEq, Hash)]
pub struct Mint {
    /// The location of the transaction in the block.
    pub(crate) tx_pointer: TxPointer,
    /// The list of `Output::Coin` generated by the block producer.
    pub(crate) outputs: Vec<Output>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
    #[canonical(skip)]
    pub(crate) metadata: Option<MintMetadata>,
}

mem_layout!(
    MintLayout for Mint
    repr: u8 = WORD_SIZE,
    tx_pointer: TxPointer = {TxPointer::LEN},
    outputs_len: Word = WORD_SIZE
);

#[cfg(feature = "std")]
impl crate::UniqueIdentifier for Mint {
    fn id(&self, chain_id: &ChainId) -> Bytes32 {
        if let Some(id) = self.cached_id() {
            return id
        }

        let mut clone = self.clone();
        crate::transaction::compute_transaction_id(chain_id, &mut clone)
    }

    fn cached_id(&self) -> Option<Bytes32> {
        self.metadata.as_ref().map(|m| m.id)
    }
}

impl FormatValidityChecks for Mint {
    #[cfg(feature = "std")]
    fn check_signatures(&self, _: &ChainId) -> Result<(), CheckError> {
        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), CheckError> {
        if self.outputs().len() > consensus_params.tx_params().max_outputs as usize {
            return Err(CheckError::TransactionOutputsMax)
        }
        if self.tx_pointer().block_height() != block_height {
            return Err(CheckError::TransactionMintIncorrectBlockHeight)
        }

        let mut assets = Vec::new();
        for output in self.outputs() {
            if let Output::Coin { asset_id, .. } = output {
                if assets.contains(asset_id) {
                    return Err(CheckError::TransactionOutputCoinAssetIdDuplicated(
                        *asset_id,
                    ))
                } else {
                    assets.push(*asset_id);
                }
            } else {
                return Err(CheckError::TransactionMintOutputIsNotCoin)
            }
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl crate::Cacheable for Mint {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), CheckError> {
        self.metadata = None;
        self.metadata = Some(MintMetadata::compute(self, chain_id));
        Ok(())
    }
}

impl SizedBytes for Mint {
    fn serialized_size(&self) -> usize {
        self.outputs_offset()
            + self
                .outputs()
                .iter()
                .map(|w| w.serialized_size())
                .sum::<usize>()
    }
}

mod field {
    use super::*;

    impl TxPointerField for Mint {
        #[inline(always)]
        fn tx_pointer(&self) -> &TxPointer {
            &self.tx_pointer
        }

        #[inline(always)]
        fn tx_pointer_mut(&mut self) -> &mut TxPointer {
            &mut self.tx_pointer
        }

        #[inline(always)]
        fn tx_pointer_static() -> usize {
            WORD_SIZE // `Transaction` enum discriminant
        }
    }

    impl Outputs for Mint {
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
            if let Some(MintMetadata { outputs_offset, .. }) = &self.metadata {
                return *outputs_offset
            }

            self.tx_pointer_offset() + TxPointer::LEN + WORD_SIZE // Outputs size
        }

        #[inline(always)]
        fn outputs_offset_at(&self, idx: usize) -> Option<usize> {
            if let Some(MintMetadata {
                outputs_offset_at, ..
            }) = &self.metadata
            {
                return outputs_offset_at.get(idx).cloned()
            }

            if idx < self.outputs.len() {
                Some(
                    self.outputs_offset()
                        + self
                            .outputs()
                            .iter()
                            .take(idx)
                            .map(|i| i.serialized_size())
                            .sum::<usize>(),
                )
            } else {
                None
            }
        }
    }
}
