use fuel_types::Bytes32;
use sha2::{
    Digest,
    Sha256,
    digest::Update,
};

use core::iter;

/// Standard hasher
#[derive(Debug, Default, Clone)]
pub struct Hasher(Sha256);

impl Hasher {
    /// Length of the output
    pub const OUTPUT_LEN: usize = Bytes32::LEN;

    /// Append data to the hasher
    pub fn input<B>(&mut self, data: B)
    where
        B: AsRef<[u8]>,
    {
        sha2::Digest::update(&mut self.0, data)
    }

    /// Consume, append data and return the hasher
    pub fn chain<B>(self, data: B) -> Self
    where
        B: AsRef<[u8]>,
    {
        Self(self.0.chain(data))
    }

    /// Consume, append the items of the iterator and return the hasher
    pub fn extend_chain<B, I>(mut self, iter: I) -> Self
    where
        B: AsRef<[u8]>,
        I: IntoIterator<Item = B>,
    {
        self.extend(iter);

        self
    }

    /// Reset the hasher to the default state
    pub fn reset(&mut self) {
        self.0.reset();
    }

    /// Hash the provided data, returning its digest
    pub fn hash<B>(data: B) -> Bytes32
    where
        B: AsRef<[u8]>,
    {
        let mut hasher = Sha256::new();

        sha2::Digest::update(&mut hasher, data);

        <[u8; Bytes32::LEN]>::from(hasher.finalize()).into()
    }

    /// Consume the hasher, returning the digest
    pub fn finalize(self) -> Bytes32 {
        <[u8; Bytes32::LEN]>::from(self.0.finalize()).into()
    }

    /// Return the digest without consuming the hasher
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
