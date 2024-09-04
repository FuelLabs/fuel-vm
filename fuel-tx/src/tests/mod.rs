#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

mod offset;
mod valid_cases;

#[cfg(feature = "serde")]
mod bytes;
#[cfg(feature = "da-compression")]
mod da_compression;
#[cfg(feature = "serde")]
mod display;

#[cfg(not(feature = "serde"))]
use bincode as _;

#[cfg(not(feature = "da-compression"))]
use bimap as _;
#[cfg(not(feature = "da-compression"))]
use tokio as _;
