# Fuel VM interpreter

[![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-vm?label=latest)](https://crates.io/crates/fuel-vm)
[![docs](https://docs.rs/fuel-vm/badge.svg)](https://docs.rs/fuel-vm/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Rust interpreter for the [FuelVM](https://github.com/FuelLabs/fuel-specs).

## Compile features

- `debug`[1]: Expose the `Debugger` structure, allowing the interpreter to interact with `Breakpoints`s.
- `profile-any`[1]: Expose the `Profiler` primitives, allowing the interpreter to trace profiler implementations.
- `profile-gas`[1]: Profiler implementation to trace gas consumption per instruction.
- `random`: Implement [rand](https://crates.io/crates/rand) features for the provided types.
- `serde-types`: Add support for [serde](https://crates.io/crates/serde) for the types exposed by this crate.

[1] This will cause runtime overhead and isn't recommended for production.
