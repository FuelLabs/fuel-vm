use fuel_types::Bytes32;
use sha2::{Digest, Sha256};

use std::iter;

#[derive(Debug, Default, Clone)]
pub struct Hasher(Sha256);

impl Hasher {
    pub fn input<B>(&mut self, data: B)
    where
        B: AsRef<[u8]>,
    {
        self.0.update(data)
    }

    pub fn chain<B>(self, data: B) -> Self
    where
        B: AsRef<[u8]>,
    {
        Self(self.0.chain(data))
    }

    pub fn reset(&mut self) {
        self.0.reset();
    }

    pub fn hash<B>(data: B) -> Bytes32
    where
        B: AsRef<[u8]>,
    {
        let mut hasher = Sha256::new();

        hasher.update(data);

        <[u8; Bytes32::LEN]>::from(hasher.finalize()).into()
    }

    pub fn digest(&self) -> Bytes32 {
        <[u8; Bytes32::LEN]>::from(self.0.clone().finalize()).into()
    }
}

impl<B> iter::FromIterator<B> for Hasher
where
    B: AsRef<[u8]>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = B>,
    {
        iter.into_iter().fold(Hasher::default(), Hasher::chain)
    }
}

impl<B> Extend<B> for Hasher
where
    B: AsRef<[u8]>,
{
    fn extend<T: IntoIterator<Item = B>>(&mut self, iter: T) {
        iter.into_iter().for_each(|b| self.input(b))
    }
}
