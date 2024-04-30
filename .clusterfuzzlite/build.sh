#!/bin/bash -eu

cd $SRC/fuel-vm

cd fuel-vm

export CARGO_CFG_CURVE25519_DALEK_BACKEND=serial # This fixes building on nightly-2023-12-28-x86_64-unknown-linux-gnu, which is no longer compatible with the SIMD feature of CURVE25519; building on stable does not work because ASan is a dependency of coverage
cargo fuzz build -O --sanitizer none

cp fuzz/target/x86_64-unknown-linux-gnu/release/grammar_aware_advanced $OUT/
#cp fuzz/target/x86_64-unknown-linux-gnu/release/grammar_aware $OUT/
