//! Fuel cryptographic primitives.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

/// Required export to implement [`Keystore`].
pub use borrown;

mod error;
mod hasher;
mod keystore;
mod message;
mod public;
mod secret;
mod signature;
mod signer;

pub use error::Error;
pub use hasher::Hasher;
pub use keystore::Keystore;
pub use message::Message;
pub use public::PublicKey;
pub use secret::SecretKey;
pub use signature::Signature;
pub use signer::Signer;
