#![no_main]

use fuel_vm_fuzz::{decode, execute};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(data) = decode(data) {
        execute(data);
    }
});
