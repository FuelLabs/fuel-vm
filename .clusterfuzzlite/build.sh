#!/bin/bash -eu

cd $SRC/fuel-vm

cd fuel-vm

rustup install nightly

cargo +nightly fuzz build -O

cp fuzz/target/x86_64-unknown-linux-gnu/release/grammar_aware_advanced $OUT/
cp fuzz/target/x86_64-unknown-linux-gnu/release/grammar_aware $OUT/
