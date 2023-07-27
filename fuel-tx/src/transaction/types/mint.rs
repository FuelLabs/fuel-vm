use crate::{
    transaction::{
        compute_transaction_id,
        field::{
            Outputs,
            TxPointer as TxPointerField,
        },
        validity::FormatValidityChecks,
    },
    CheckError,
    ConsensusParameters,
    Output,
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
    ChainId,
    MemLayout,
    MemLocType,
    Word,
};

#[cfg(feature = "std")]
use std::io;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use fuel_types::bytes::{
    self,
    Deserializable,
};

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
/// https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx_format/transaction.md#transactionmint
///
/// This transaction can be created by the block producer and included in the block only
/// by it.
#[derive(Default, Debug, Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Serialize, fuel_types::canonical::Deserialize)]
#[derivative(Eq, PartialEq, Hash)]
pub struct Mint {
    /// The location of the transaction in the block.
    pub(crate) tx_pointer: TxPointer,
    /// The list of `Output::Coin` generated by the block producer.
    pub(crate) outputs: Vec<Output>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
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
        compute_transaction_id(chain_id, &mut clone)
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

#[cfg(feature = "std")]
impl io::Read for Mint {
    fn read(&mut self, full_buf: &mut [u8]) -> io::Result<usize> {
        let serialized_size = self.serialized_size();
        if full_buf.len() < serialized_size {
            return Err(bytes::eof())
        }

        let buf: &mut [_; Self::LEN] = full_buf
            .get_mut(..Self::LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        bytes::store_number_at(
            buf,
            Self::layout(Self::LAYOUT.repr),
            crate::TransactionRepr::Mint as u8,
        );
        let Mint {
            tx_pointer,
            outputs,
            metadata: _,
        } = self;

        let n = tx_pointer.read(&mut buf[Self::LAYOUT.tx_pointer.range()])?;
        if n != Self::LAYOUT.tx_pointer.size() {
            return Err(bytes::eof())
        }
        bytes::store_number_at(
            buf,
            Self::layout(Self::LAYOUT.outputs_len),
            outputs.len() as Word,
        );

        let mut buf = full_buf.get_mut(Self::LEN..).ok_or(bytes::eof())?;
        for output in outputs {
            let output_len = output.read(buf)?;
            buf = &mut buf[output_len..];
        }

        Ok(serialized_size)
    }
}

#[cfg(feature = "std")]
impl io::Write for Mint {
    fn write(&mut self, full_buf: &[u8]) -> io::Result<usize> {
        let mut n = crate::consts::TRANSACTION_MINT_FIXED_SIZE;
        if full_buf.len() < n {
            return Err(bytes::eof())
        }

        let buf: &[_; Self::LEN] = full_buf
            .get(..Self::LEN)
            .and_then(|slice| slice.try_into().ok())
            .ok_or(bytes::eof())?;

        let identifier = bytes::restore_u8_at(buf, Self::layout(Self::LAYOUT.repr));
        let identifier = crate::TransactionRepr::try_from(identifier as Word)?;
        if identifier != crate::TransactionRepr::Mint {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier to the `Script` is invalid!",
            ))
        }

        // Safety: buffer size is checked
        let tx_pointer = TxPointer::from_bytes(&buf[Self::LAYOUT.tx_pointer.range()])?;
        let outputs_len =
            bytes::restore_usize_at(buf, Self::layout(Self::LAYOUT.outputs_len));

        let mut buf = full_buf.get(Self::LEN..).ok_or(bytes::eof())?;
        let mut outputs = vec![Output::default(); outputs_len];
        for output in outputs.iter_mut() {
            let output_len = output.write(buf)?;
            buf = &buf[output_len..];
            n += output_len;
        }

        *self = Mint {
            tx_pointer,
            outputs,
            metadata: None,
        };

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.outputs
            .iter_mut()
            .try_for_each(|output| output.flush())?;

        Ok(())
    }
}
