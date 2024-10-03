#!/bin/bash -eu

cd $SRC/fuel-vm

cd fuel-vm

cargo fuzz build -O --sanitizer none

cp fuzz/target/x86_64-unknown-linux-gnu/release/grammar_aware_advanced $OUT/
