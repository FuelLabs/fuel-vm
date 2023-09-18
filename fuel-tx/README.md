# Fuel Specification Implementation

[![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-tx?label=latest)](https://crates.io/crates/fuel-tx)
[![docs](https://docs.rs/fuel-tx/badge.svg)](https://docs.rs/fuel-tx/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

This crate contains a definition of types from the [specification](https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/index.md),
with canonical serialization and deserialization. The `Transaction` and `Checked<Tx>` type 
implements fee calculation and [validation of rules](https://github.com/FuelLabs/fuel-specs/blob/master/src/protocol/tx-validity.md) defined by the specification.