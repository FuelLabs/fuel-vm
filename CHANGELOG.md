# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased] - yyyy-mm-dd

Description of the upcoming release here.

### Added

#### Breaking

- [#386](https://github.com/FuelLabs/fuel-vm/pull/386): The coin and message inputs 
    got a new field - `predicate_gas_used`. So it breaks the constructor API 
    of these inputs.

    The value of this field is zero for non-predicate inputs, but for the 
    predicates, it indicates the exact amount of gas used by the predicate 
    to execute. If after the execution of the predicate remaining gas is not 
    zero, then the predicate execution failed.
    
    This field is malleable but will be used by the VM, and each predicate 
    should be estimated before performing the verification logic. 
    The `Transaction`, `Create`, and `Script` types implement the 
    `EstimatePredicates` for these purposes.

    ```rust
    /// Provides predicate estimation functionality for the transaction.
    pub trait EstimatePredicates: Sized {
        /// Estimates predicates of the transaction.
        fn estimate_predicates(&mut self, params: &ConsensusParameters, gas_costs: &GasCosts) -> Result<(), CheckError>;
    }
    ```

    During the creation of the `Input`, the best strategy is to use a default 
    value like `0` and call the `estimate_predicates` method to actualize 
    the `predicate_gas_used` after.

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

- [#386](https://github.com/FuelLabs/fuel-vm/pull/386): Several methods of the `TransactionFee` are renamed `total` -> `max_fee` 
    and `bytes` -> `min_fee`. The `TransactionFee::min_fee` take into account the gas used by predicates.

### Fixed

- Some fix here 1
- Some fix here 2

#### Breaking

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): The storage slots with the same key inside of the `Create` transaction are forbidden.
