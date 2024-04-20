#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

mod offset;
mod valid_cases;

#[cfg(feature = "serde")]
mod bytes;
#[cfg(feature = "serde")]
mod display;

#[cfg(not(feature = "serde"))]
use bincode as _;
