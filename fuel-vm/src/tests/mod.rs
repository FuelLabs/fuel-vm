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
mod coins;
mod continue_on_error;
mod contract;
mod crypto;
mod debugger;
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
mod receipts;
mod spec;
mod upgrade;
mod upload;
mod validation;
mod wideint;
