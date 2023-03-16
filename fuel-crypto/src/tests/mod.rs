// It is used in the benches
use criterion as _;
use k256 as _;

mod hasher;
mod mnemonic;
mod signature;
mod signer;

#[cfg(feature = "serde")]
mod serde;
#[cfg(not(feature = "serde"))]
use bincode as _;
