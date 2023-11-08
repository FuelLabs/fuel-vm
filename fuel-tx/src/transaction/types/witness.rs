use derivative::Derivative;
use fuel_types::fmt_truncated_hex;

use alloc::vec::Vec;

use crate::{
    Input,
    TxId,
    ValidityError,
};
use fuel_crypto::{
    Message,
    Signature,
};

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

#[derive(Derivative, Default, Clone, PartialEq, Eq, Hash)]
#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
pub struct Witness {
    #[derivative(Debug(format_with = "fmt_truncated_hex::<16>"))]
    data: Vec<u8>,
}

impl Witness {
    pub const fn as_vec(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn as_vec_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    /// ECRecover an address from a witness
    pub fn recover_witness(
        &self,
        txhash: &TxId,
        witness_index: usize,
    ) -> Result<fuel_types::Address, ValidityError> {
        let bytes = <[u8; Signature::LEN]>::try_from(self.as_ref()).map_err(|_| {
            ValidityError::InputInvalidSignature {
                index: witness_index,
            }
        })?;
        let signature = Signature::from_bytes(bytes);

        let message = Message::from_bytes_ref(txhash);

        signature
            .recover(message)
            .map_err(|_| ValidityError::InputInvalidSignature {
                index: witness_index,
            })
            .map(|pk| Input::owner(&pk))
    }
}

impl From<Vec<u8>> for Witness {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl From<&[u8]> for Witness {
    fn from(data: &[u8]) -> Self {
        data.to_vec().into()
    }
}

impl AsRef<[u8]> for Witness {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl AsMut<[u8]> for Witness {
    fn as_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

impl Extend<u8> for Witness {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.data.extend(iter);
    }
}

#[cfg(feature = "random")]
impl Distribution<Witness> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Witness {
        let len = rng.gen_range(0..512);

        let mut data = alloc::vec![0u8; len];
        rng.fill_bytes(data.as_mut_slice());

        data.into()
    }
}
