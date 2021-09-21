# Fuel Data

Provides base data types for the Fuel infrastructure.

The traits `Storage` and `MerkleStorage` will be used as base in fuel-vm to define the client requirements for the VM implementation.

# Features

* `default/std` - Enable `libstd` functionalities with `Storage` and `MerkleStorage`.
* `random` - Enable `std` and create random generator implementations for the provided types.
* `serde-types` - Enable `serde::{serialize, deserialize}` for the provided types
* `serde-types-minimal` - Enable `no-std` `serde::{serialize, deserialize}` for the provided types
