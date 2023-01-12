# Fuel execution environment

 [![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

An implementation of the [FuelVM specification](https://github.com/FuelLabs/fuel-specs/blob/master/src/vm/index.md) used by [fuel-core](https://github.com/FuelLabs/fuel-core) and [the Sway compiler](https://github.com/FuelLabs/sway/).

## Crates living here

Crate | Version | Description
------|---------|-------------
fuel-asm | [![crates.io](https://img.shields.io/crates/v/fuel-asm)](https://crates.io/crates/fuel-asm) | Instruction set
fuel-crypto | [![crates.io](https://img.shields.io/crates/v/fuel-crypto)](https://crates.io/crates/fuel-crypto) | Cryptographic primitives
fuel-storage | [![crates.io](https://img.shields.io/crates/v/fuel-storage)](https://crates.io/crates/fuel-storage) | Storage abstraction
fuel-tx | [![crates.io](https://img.shields.io/crates/v/fuel-tx)](https://crates.io/crates/fuel-tx) | Transaction fields, types and checking
fuel-types | [![crates.io](https://img.shields.io/crates/v/fuel-types)](https://crates.io/crates/fuel-types) | Atomic types used by the VM
fuel-vm | [![crates.io](https://img.shields.io/crates/v/fuel-vm)](https://crates.io/crates/fuel-vm) | The VM itself
