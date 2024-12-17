#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]
#![allow(non_snake_case)]

use futures as _;
use ntest as _;
use tokio as _;
use tokio_rayon as _;

mod test_helpers;

mod alu;
mod backtrace;
mod blob;
mod blockchain;
mod cgas;
mod code_coverage;
mod coins;
mod contract;
mod crypto;
mod encoding;
mod external;
mod flow;
mod gas_factor;
mod jump_absolute;
mod jump_relative;
mod limits;
mod log;
mod memory;
mod metadata;
mod outputs;
mod predicate;
mod profile_gas;
mod receipts;
mod serde_profile;
mod spec;
mod upgrade;
mod upload;
mod validation;
mod wideint;
