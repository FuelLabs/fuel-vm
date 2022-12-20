use fuel_types::bytes::{self, WORD_SIZE};

#[cfg(feature = "random")]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::io;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Witness {
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

        let mut data = vec![0u8; len];
        rng.fill_bytes(data.as_mut_slice());

        data.into()
    }
}

impl bytes::SizedBytes for Witness {
    fn serialized_size(&self) -> usize {
        WORD_SIZE + bytes::padded_len(self.data.as_slice())
    }
}

#[cfg(feature = "std")]
impl io::Read for Witness {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        bytes::store_bytes(buf, self.data.as_slice()).map(|(n, _)| n)
    }
}

#[cfg(feature = "std")]
impl io::Write for Witness {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        bytes::restore_bytes(buf).map(|(n, data, _)| {
            self.data = data;
            n
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
