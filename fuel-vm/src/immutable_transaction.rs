//! A immutable transaction is type-wrapper for transactions which have their metadata pre-computed.
//! This type ensure that the metadata computed corresponds to the transaction associated.
//! This type allow to pass to `CheckedTransaction` without recomputing the metadata.

use fuel_tx::{Cacheable, FormatValidityChecks, ValidityError};
use fuel_types::ChainId;

/// Immutable transaction.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Immutable<Tx: Cacheable + FormatValidityChecks> {
    pub(crate) tx: Tx,
}

impl<Tx: Cacheable + FormatValidityChecks> Immutable<Tx> {
    /// Create a new immutable transaction.
    fn new(mut tx: Tx, chain_id: &ChainId) -> Result<Self, ValidityError> {
        tx.precompute(chain_id)?;
        Ok(Self { tx })
    }
}

impl<Tx: Cacheable + FormatValidityChecks> FormatValidityChecks for Immutable<Tx> {
    fn check(
            &self,
            block_height: fuel_types::BlockHeight,
            consensus_params: &fuel_tx::ConsensusParameters,
        ) -> Result<(), ValidityError> {
        self.tx.check(block_height, consensus_params)
    }

    fn check_signatures(&self, chain_id: &ChainId) -> Result<(), ValidityError> {
        self.tx.check_signatures(chain_id)
    }

    fn check_without_signatures(
            &self,
            block_height: fuel_types::BlockHeight,
            consensus_params: &fuel_tx::ConsensusParameters,
        ) -> Result<(), ValidityError> {
        self.tx.check_without_signatures(block_height, consensus_params)
    }
}