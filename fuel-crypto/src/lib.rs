//! Fuel cryptographic primitives.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]

#[cfg(test)]
// Satisfy unused_crate_dependencies lint for self-dependency enabling test features
use fuel_crypto as _;

/// Required export to implement [`Keystore`].
#[doc(no_inline)]
pub use borrown;
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

pub mod ed25519;
pub mod secp256r1;

#[cfg(test)]
mod tests;

pub use error::Error;
pub use hasher::Hasher;
pub use message::Message;

#[cfg(all(feature = "std", feature = "random"))]
pub use mnemonic::generate_mnemonic_phrase;

mod secp256k1 {
    mod public;
    mod secret;
    mod signature;

    pub use public::PublicKey;
    pub use secret::SecretKey;
    pub use signature::Signature;
}

// The default cryptographic primitives
pub use self::secp256k1::*;
