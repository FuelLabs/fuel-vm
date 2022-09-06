//! Storage backend implementations.

use core::marker::PhantomData;
use fuel_storage::Mappable;
use fuel_tx::Contract;
use fuel_types::{AssetId, Bytes32, ContractId, Salt, Word};

mod interpreter;
mod memory;
mod predicate;

pub use interpreter::InterpreterStorage;
pub use memory::MemoryStorage;
pub use predicate::PredicateStorage;

/// The storage type for contract's raw byte code.
pub struct ContractsRawCode;

impl Mappable for ContractsRawCode {
    type Key = ContractId;
    type SetValue = [u8];
    type GetValue = Contract;
}

/// The storage type for contract's additional information.
pub struct ContractsInfo;

impl Mappable for ContractsInfo {
    type Key = ContractId;
    /// `Salt` - is the salt used during creation of the contract for uniques.
    /// `Byte32` - is the root hash of the contract's code.
    type SetValue = (Salt, Bytes32);
    type GetValue = Self::SetValue;
}

/// The storage type for contract's assets balances.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsAssets<'a>(PhantomData<&'a ()>);

impl<'a> Mappable for ContractsAssets<'a> {
    type Key = (&'a ContractId, &'a AssetId);
    type SetValue = Word;
    type GetValue = Self::SetValue;
}

/// The storage type for contract's state.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsState<'a>(PhantomData<&'a ()>);

impl<'a> Mappable for ContractsState<'a> {
    type Key = (&'a ContractId, &'a Bytes32);
    type SetValue = Bytes32;
    type GetValue = Self::SetValue;
}
