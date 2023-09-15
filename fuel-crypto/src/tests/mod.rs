// It is used in the benches
use criterion as _;
use k256 as _;

mod hasher;

#[cfg(feature = "std")]
mod mnemonic;

mod signature;

#[cfg(feature = "serde")]
mod serde;
#[cfg(not(feature = "serde"))]
use bincode as _;
