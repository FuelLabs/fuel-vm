//! Fuel cryptographic primitives.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

mod error;
mod hasher;
mod public;
mod secret;

pub use error::Error;
pub use hasher::Hasher;
pub use public::PublicKey;
pub use secret::SecretKey;

/// Signature of a message
pub type Signature = fuel_types::Bytes64;
