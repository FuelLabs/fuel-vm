# Fuel VM custom serialization derive macros

[![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-derive?label=latest)](https://crates.io/crates/fuel-derive)
[![docs](https://docs.rs/fuel-derive/badge.svg)](https://docs.rs/fuel-derive/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

This crate contains derive macros for canonical serialization and deserialization. This is used with [`fuel-types/src/canonical.rs`](fuel-types/src/canonical.rs) module which contains the associated traits and their implementations for native Rust types. It also contains compression macros exported by `fuel-compression`.
