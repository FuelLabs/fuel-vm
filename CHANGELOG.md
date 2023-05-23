# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased] - yyyy-mm-dd

Description of the upcoming release here.

### Added

#### Breaking

- Added a new type - `ChainId` to represent the identifier of the chain. 
It is a wrapper around the `u64`, so any `u64` can be converted into this type via `.into()` or `ChainId::new(...)` - [#456](https://github.com/FuelLabs/fuel-vm/pull/456)


### Changed

- Something changed here 1
- Something changed here 2

#### Breaking

- The basic methods `UniqueIdentifier::id`, `Signable::sign_inputs`, 
and `Input::predicate_owner` use `ChainId` instead of the `ConsensusParameters`. 
It is a less strict requirement than before because you can get `ChainId` 
from `ConsensusParameters.chain_id`, and it makes the API cleaner. 
It affects all downstream functions that use listed methods - [#456](https://github.com/FuelLabs/fuel-vm/pull/456)

### Fixed

- Some fix here 1
- Some fix here 2

#### Breaking
- Some breaking fix here 3
- Some breaking fix here 4
