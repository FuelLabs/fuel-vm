//! Fuel cryptographic primitives.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]

// Satisfy unused_crate_dependencies lint for self-dependency enabling test features
#[cfg(test)]
use fuel_crypto as _;

use base64ct as _;
use half as _;

/// Required export for using mnemonic keygen on [`SecretKey::new_from_mnemonic`]
#[cfg(feature = "std")]
#[doc(no_inline)]
pub use coins_bip32;
/// Required export for using mnemonic keygen on [`SecretKey::new_from_mnemonic`]
#[cfg(feature = "std")]
#[doc(no_inline)]
pub use coins_bip39;
/// Required export to use various public interfaces in this crate
#[doc(no_inline)]
pub use fuel_types;
#[cfg(feature = "random")]
#[doc(no_inline)]
/// Required export to use randomness features
pub use rand;

mod error;
mod hasher;
mod message;
mod mnemonic;
mod secp256;

pub mod ed25519;

pub use secp256::backend::r1 as secp256r1;

pub use secp256::{
    PublicKey,
    SecretKey,
    Signature,
};

#[cfg(test)]
mod tests;

pub use error::Error;
pub use hasher::Hasher;
pub use message::Message;

#[cfg(all(feature = "std", feature = "random"))]
pub use mnemonic::generate_mnemonic_phrase;
