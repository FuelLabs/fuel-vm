use crate::{
    input,
    output,
    transaction::{
        field::TxPointer as TxPointerField,
        validity::{
            check_size,
            FormatValidityChecks,
        },
    },
    ConsensusParameters,
    TransactionRepr,
    TxId,
    TxPointer,
    ValidityError,
};
use derivative::Derivative;
use fuel_asm::Word;
use fuel_types::{
    bytes::WORD_SIZE,
    AssetId,
    BlockHeight,
    Bytes32,
};

use fuel_types::ChainId;

use fuel_types::canonical::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MintMetadata {
    pub id: Bytes32,
}

impl MintMetadata {
    fn compute<Tx>(tx: &Tx, chain_id: &ChainId) -> Self
    where
        Tx: crate::UniqueIdentifier,
    {
        let id = tx.id(chain_id);

        Self { id }
    }
}

/// The definition of the `Mint` transaction from the specification:
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/transaction.md#transactionmint>
///
/// This transaction can be created by the block producer and included in the block only
/// by it.
#[derive(Default, Debug, Clone, Derivative, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "da-compression", derive(fuel_compression::Compress))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
#[canonical(prefix = TransactionRepr::Mint)]
#[derivative(Eq, PartialEq, Hash)]
pub struct Mint {
    /// The location of the transaction in the block.
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub(crate) tx_pointer: TxPointer,
    /// The `Input::Contract` that assets are minted to.
    pub(crate) input_contract: input::contract::Contract,
    /// The `Output::Contract` that assets are being minted to.
    pub(crate) output_contract: output::contract::Contract,
    /// The amount of funds minted.
    pub(crate) mint_amount: Word,
    /// The asset IDs corresponding to the minted amount.
    pub(crate) mint_asset_id: AssetId,
    /// Gas Price used for current block
    pub(crate) gas_price: Word,
    #[serde(skip)]
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
    #[canonical(skip)]
    #[cfg_attr(feature = "da-compression", compress(skip))]
    pub(crate) metadata: Option<MintMetadata>,
}

impl crate::UniqueIdentifier for Mint {
    fn id(&self, chain_id: &ChainId) -> Bytes32 {
        if let Some(id) = self.cached_id() {
            return id;
        }

        let mut clone = self.clone();
        clone.input_contract.prepare_sign();
        clone.output_contract.prepare_sign();

        crate::transaction::compute_transaction_id(chain_id, &mut clone)
    }

    fn cached_id(&self) -> Option<Bytes32> {
        self.metadata.as_ref().map(|m| m.id)
    }
}

impl FormatValidityChecks for Mint {
    fn check_signatures(&self, _: &ChainId) -> Result<(), ValidityError> {
        Ok(())
    }

    fn check_without_signatures(
        &self,
        block_height: BlockHeight,
        consensus_params: &ConsensusParameters,
    ) -> Result<(), ValidityError> {
        check_size(self, consensus_params.tx_params())?;

        if self.tx_pointer().block_height() != block_height {
            return Err(ValidityError::TransactionMintIncorrectBlockHeight);
        }

        if self.output_contract.input_index != 0 {
            return Err(ValidityError::TransactionMintIncorrectOutputIndex);
        }

        // It is temporary check until https://github.com/FuelLabs/fuel-core/issues/1205
        if &self.mint_asset_id != consensus_params.base_asset_id() {
            return Err(ValidityError::TransactionMintNonBaseAsset);
        }

        Ok(())
    }
}

impl crate::Cacheable for Mint {
    fn is_computed(&self) -> bool {
        self.metadata.is_some()
    }

    fn precompute(&mut self, chain_id: &ChainId) -> Result<(), (TxId, ValidityError)> {
        self.metadata = None;
        self.metadata = Some(MintMetadata::compute(self, chain_id));
        Ok(())
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl Mint {
    // This is a function to clear malleable fields just like it
    // does on other transactions types. Mint never needs this,
    // but we use it for some tests.
    pub fn prepare_sign(&mut self) {
        self.input_contract.prepare_sign();
        self.output_contract.prepare_sign();
    }
}

mod field {
    use super::*;
    use crate::field::{
        InputContract,
        MintAmount,
        MintAssetId,
        MintGasPrice,
        OutputContract,
    };

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

    impl InputContract for Mint {
        #[inline(always)]
        fn input_contract(&self) -> &input::contract::Contract {
            &self.input_contract
        }

        #[inline(always)]
        fn input_contract_mut(&mut self) -> &mut input::contract::Contract {
            &mut self.input_contract
        }

        #[inline(always)]
        fn input_contract_offset(&self) -> usize {
            Self::tx_pointer_static().saturating_add(TxPointer::LEN)
        }
    }

    impl OutputContract for Mint {
        #[inline(always)]
        fn output_contract(&self) -> &output::contract::Contract {
            &self.output_contract
        }

        #[inline(always)]
        fn output_contract_mut(&mut self) -> &mut output::contract::Contract {
            &mut self.output_contract
        }

        #[inline(always)]
        fn output_contract_offset(&self) -> usize {
            self.input_contract_offset()
                .saturating_add(self.input_contract.size())
        }
    }

    impl MintAmount for Mint {
        #[inline(always)]
        fn mint_amount(&self) -> &fuel_types::Word {
            &self.mint_amount
        }

        #[inline(always)]
        fn mint_amount_mut(&mut self) -> &mut fuel_types::Word {
            &mut self.mint_amount
        }

        #[inline(always)]
        fn mint_amount_offset(&self) -> usize {
            self.output_contract_offset()
                .saturating_add(self.output_contract.size())
        }
    }

    impl MintAssetId for Mint {
        #[inline(always)]
        fn mint_asset_id(&self) -> &AssetId {
            &self.mint_asset_id
        }

        #[inline(always)]
        fn mint_asset_id_mut(&mut self) -> &mut AssetId {
            &mut self.mint_asset_id
        }

        #[inline(always)]
        fn mint_asset_id_offset(&self) -> usize {
            self.mint_amount_offset().saturating_add(WORD_SIZE)
        }
    }

    impl MintGasPrice for Mint {
        #[inline(always)]
        fn gas_price(&self) -> &Word {
            &self.gas_price
        }

        #[inline(always)]
        fn gas_price_mut(&mut self) -> &mut Word {
            &mut self.gas_price
        }

        #[inline(always)]
        fn gas_price_offset(&self) -> usize {
            self.mint_asset_id_offset().saturating_add(AssetId::LEN)
        }
    }
}
