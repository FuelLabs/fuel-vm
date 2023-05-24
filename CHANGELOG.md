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

- [#454](https://github.com/FuelLabs/fuel-vm/pull/454): VM native array-backed types 
`Address`, `AssetId`, `ContractId`, `Bytes4`, `Bytes8`, `Bytes20`, `Bytes32`, 
`Nonce`, `MessageId`, `Salt` now use more compact representation instead of 
hex-encoded string when serialized using serde format that sets 
`is_human_readable` to false.

- [#456](https://github.com/FuelLabs/fuel-vm/pull/456): Added a new type - `ChainId` to represent the identifier of the chain. 
It is a wrapper around the `u64`, so any `u64` can be converted into this type via `.into()` or `ChainId::new(...)`.

- [#459](https://github.com/FuelLabs/fuel-vm/pull/459) Require witness index to be specified when adding an unsigned coin to a transaction.
This allows for better reuse of witness data when using the transaction builder and helper methods to make transactions compact.

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

#### Breaking

- [#457](https://github.com/FuelLabs/fuel-vm/pull/457): Transactions got one more validity rule: 
Each `Script` or `Create` transaction requires at least one input coin or message to be spendable. 
It may break code/tests that previously didn't set any spendable inputs. 
Note: `Message` with non-empty `data` field is not spendable.

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): The storage slots with the same key inside the `Create` transaction are forbidden.
