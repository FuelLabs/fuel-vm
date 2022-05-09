# Fuel Types

[![build](https://github.com/FuelLabs/fuel-types/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-types/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-types?label=latest)](https://crates.io/crates/fuel-types)
[![docs](https://docs.rs/fuel-types/badge.svg)](https://docs.rs/fuel-types/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Rust implementation of the atomic types for the [FuelVM](https://github.com/FuelLabs/fuel-specs).

## Compile features

- `std`: Unless set, the crate will link to the core-crate instead of the std-crate. More info [here](https://docs.rust-embedded.org/book/intro/no-std.html).
- `alloc`: Use [Vec](https://doc.rust-lang.org/alloc/vec/struct.Vec.html) from [alloc](https://doc.rust-lang.org/alloc/index.html) for `no-std`.
- `random`: Implement `no-std` [rand](https://crates.io/crates/rand) features for the provided types.
- `serde`: Add support for [serde](https://crates.io/crates/serde) for the provided types.