use crate::digest::Digest;
use generic_array::{typenum, GenericArray};
use sha2::Digest as DigestImpl;
use sha2::Sha256 as Sha256Impl;

pub struct Sha256 {
    internal: Sha256Impl,
}

impl Digest for Sha256 {
    type OutputSize = typenum::U32;

    fn new() -> Self {
        Self {
            internal: Sha256Impl::new(),
        }
    }

    fn update(&mut self, input: impl AsRef<[u8]>) {
        DigestImpl::update(&mut self.internal, input.as_ref());
    }

    fn finalize(self) -> GenericArray<u8, Self::OutputSize> {
        self.internal.finalize()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hex;

    #[test]
    fn finalize_returns_a_byte_array_of_32_bytes() {
        let mut hash = Sha256::new();
        let data = String::from("hello world");
        hash.update(data);
        let result = hash.finalize();

        assert_eq!(result.len(), 32);
    }

    #[test]
    fn finalize_returns_the_sha256_hash_of_the_empty_string_given_no_input() {
        let hash = Sha256::new();
        let result = hash.finalize();

        let hex = hex::encode(result);
        let expected_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hex, expected_hex);
    }

    #[test]
    fn finalize_returns_the_sha256_hash_of_the_given_input() {
        let mut hash = Sha256::new();
        hash.update("Hello, World!");
        let result = hash.finalize();

        let hex = hex::encode(result);
        let expected_hex = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert_eq!(hex, expected_hex);
    }

    #[test]
    fn finalize_returns_the_sha256_hash_of_the_given_multiple_inputs() {
        let mut hash = Sha256::new();
        hash.update("12345");
        hash.update("67890");
        let result = hash.finalize();

        let hex = hex::encode(result);
        let expected_hex = "c775e7b757ede630cd0aa1113bd102661ab38829ca52a6422ab782862f268646";
        assert_eq!(hex, expected_hex);
    }
}
