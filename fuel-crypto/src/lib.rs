//! Fuel cryptographic primitives.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
// Wrong clippy convention; check
// https://rust-lang.github.io/api-guidelines/naming.html
#![allow(clippy::wrong_self_convention)]
#![deny(unsafe_code)]

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
mod keystore;
mod message;
mod mnemonic;
mod public;
mod secret;
mod signature;
mod signer;

pub use error::Error;
pub use hasher::Hasher;
pub use keystore::Keystore;
pub use message::Message;
#[cfg(all(feature = "std", feature = "random"))]
pub use mnemonic::generate_mnemonic_phrase;
pub use public::PublicKey;
pub use secret::SecretKey;
pub use signature::Signature;
pub use signer::Signer;
