# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased] - yyyy-mm-dd

Description of the upcoming release here.

### Added

#### Breaking

- [#454](https://github.com/FuelLabs/fuel-vm/pull/454): VM native array-backed types `Address`, `AssetId`, `ContractId`, `Bytes4`, `Bytes8`, `Bytes20`, `Bytes32`, `Nonce`, `MessageId`, `Salt` now use more compact representation instead of hex-encoded string when serialized using serde format that sets `is_human_readable` to false.
- [#456](https://github.com/FuelLabs/fuel-vm/pull/456): Added a new type - `ChainId` to represent the identifier of the chain. 
It is a wrapper around the `u64`, so any `u64` can be converted into this type via `.into()` or `ChainId::new(...)`.

### Changed

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): Automatically sort storage slots for creation transactions.

#### Breaking

- [#456](https://github.com/FuelLabs/fuel-vm/pull/456): The basic methods `UniqueIdentifier::id`, `Signable::sign_inputs`, 
and `Input::predicate_owner` use `ChainId` instead of the `ConsensusParameters`. 
It is a less strict requirement than before because you can get `ChainId` 
from `ConsensusParameters.chain_id`, and it makes the API cleaner. 
It affects all downstream functions that use listed methods.
- [#450](https://github.com/FuelLabs/fuel-vm/pull/450): The Merkle root of a contract's code is now calculated by partitioning the code into chunks of 16 KiB, instead of 8 bytes. If the last leaf is does not a full 16 KiB, it is padded with `0` up to the nearest multiple of 8 bytes. This affects the `ContractId` and `PredicateId` calculations, breaking all code that used hardcoded values.

### Fixed

- Some fix here 1
- Some fix here 2

#### Breaking

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): The storage slots with the same key inside of the `Create` transaction are forbidden.
