use crate::double_key;
use fuel_storage::Mappable;
use fuel_types::{
    fmt_truncated_hex,
    Bytes32,
    ContractId,
};

use alloc::{
    vec,
    vec::Vec,
};
use educe::Educe;

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

/// The storage table for contract's hashed key-value state.
///
/// Lifetime is for optimization to avoid `clone`.
pub struct ContractsState;

impl Mappable for ContractsState {
    type Key = Self::OwnedKey;
    /// The table key is combination of the `ContractId` and `Bytes32` hash of the value's
    /// key.
    type OwnedKey = ContractsStateKey;
    type OwnedValue = ContractsStateData;
    type Value = [u8];
}

double_key!(
    ContractsStateKey,
    ContractId,
    contract_id,
    Bytes32,
    state_key
);

/// Storage type for contract state
#[derive(Educe, Clone, PartialEq, Eq, Hash)]
#[educe(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContractsStateData(
    #[educe(Debug(method(fmt_truncated_hex::<16>)))] pub Vec<u8>,
);

// TODO: Remove fixed size default when adding support for dynamic storage
impl Default for ContractsStateData {
    fn default() -> Self {
        Self(vec![0u8; 32])
    }
}

impl From<Vec<u8>> for ContractsStateData {
    fn from(c: Vec<u8>) -> Self {
        Self(c)
    }
}

impl From<&[u8]> for ContractsStateData {
    fn from(c: &[u8]) -> Self {
        Self(c.into())
    }
}

impl From<&mut [u8]> for ContractsStateData {
    fn from(c: &mut [u8]) -> Self {
        Self(c.into())
    }
}

impl From<ContractsStateData> for Vec<u8> {
    fn from(c: ContractsStateData) -> Vec<u8> {
        c.0
    }
}

impl AsRef<[u8]> for ContractsStateData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for ContractsStateData {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

#[cfg(feature = "random")]
impl Distribution<ContractsStateData> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ContractsStateData {
        ContractsStateData(rng.gen::<Bytes32>().to_vec())
    }
}

impl IntoIterator for ContractsStateData {
    type IntoIter = alloc::vec::IntoIter<Self::Item>;
    type Item = u8;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
