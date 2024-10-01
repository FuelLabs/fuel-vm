#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

mod offset;
mod valid_cases;

mod bytes;
#[cfg(feature = "da-compression")]
mod da_compression;
mod display;
