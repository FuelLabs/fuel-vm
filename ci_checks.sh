#!/usr/bin/env bash

# The script runs almost all CI checks locally.
#
# Requires installed:
# - Rust `1.85.0`
# - Nightly rust formatter
# - `rustup target add thumbv6m-none-eabi`
# - `rustup target add wasm32-unknown-unknown`
# - `cargo install cargo-sort`
# - `cargo install cargo-make`

cargo +nightly fmt --all -- --check &&
cargo sort -w --check &&
cargo clippy --all-targets --all-features -- -D warnings -D clippy::dbg_macro &&
cargo check --all-targets &&
cargo check --all-targets -p fuel-asm &&
cargo check --all-targets -p fuel-crypto &&
cargo check --all-targets -p fuel-merkle &&
cargo check --all-targets -p fuel-storage &&
cargo check --all-targets -p fuel-tx &&
cargo check --all-targets -p fuel-types &&
cargo check --all-targets -p fuel-vm &&
cargo check --all-targets --no-default-features &&
cargo check --all-targets --all-features &&
cargo check --target thumbv6m-none-eabi -p fuel-asm -p fuel-storage -p fuel-merkle --no-default-features &&
cargo check --target wasm32-unknown-unknown -p fuel-crypto --no-default-features &&
cargo check --target wasm32-unknown-unknown -p fuel-types --features serde --no-default-features &&
cargo check --target wasm32-unknown-unknown -p fuel-tx --features alloc --no-default-features &&
cargo check --target wasm32-unknown-unknown -p fuel-vm --features alloc --no-default-features &&
cargo rustc --target wasm32-unknown-unknown -p fuel-types --features typescript --crate-type=cdylib &&
cargo rustc --target wasm32-unknown-unknown -p fuel-asm --features typescript --crate-type=cdylib &&
cargo make check &&
cargo test --all-targets --all-features &&
cargo test --all-targets --no-default-features &&
cargo test --all-targets --no-default-features --features serde &&
cargo test --all-targets --no-default-features --features alloc &&
cargo test --all-targets --features random &&
cargo test --all-targets --features serde
