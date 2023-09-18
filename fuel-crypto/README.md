# Fuel Crypto

[![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-crypto?label=latest)](https://crates.io/crates/fuel-crypto)
[![docs](https://docs.rs/fuel-crypto/badge.svg)](https://docs.rs/fuel-crypto/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Fuel cryptographic primitives defined by the [specification](https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/cryptographic-primitives.md).

## Compile features

- `std`: Unless set, the crate will link to the core-crate instead of the std-crate. More info [here](https://docs.rust-embedded.org/book/intro/no-std.html).
- `random`: Implement `no-std` [rand](https://crates.io/crates/rand) features for the provided types.
- `serde`: Add support for [serde](https://crates.io/crates/serde) for the provided types.
