//! Fuel cryptographic primitives.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]

mod error;
mod hasher;
mod message;
mod public;
mod secret;
mod signature;

pub use error::Error;
pub use hasher::Hasher;
pub use message::Message;
pub use public::PublicKey;
pub use secret::SecretKey;
pub use signature::Signature;
