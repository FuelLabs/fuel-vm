# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added
- [#603](https://github.com/FuelLabs/fuel-vm/pull/603): Added `MerkleRootCalculator`for efficient in-memory Merkle root calculation.
- [#603](https://github.com/FuelLabs/fuel-vm/pull/606): Added Serialization and Deserialization support to `MerkleRootCalculator`.

### Changed

- [#595](https://github.com/FuelLabs/fuel-vm/pull/595): Removed `wee_alloc` dependency from `fuel-asm`. It now uses the builtin allocator on web targets as well.

#### Breaking

- [#598](https://github.com/FuelLabs/fuel-vm/pull/598): Update cost model for `ldc` opcode to take into account contract size.
- [#604](https://github.com/FuelLabs/fuel-vm/pull/604): Removed `ChainId` from `PredicateId` calculation. It changes the generated address of the predicates and may break tests or logic that uses hard-coded predicate IDs.
- [#594](https://github.com/FuelLabs/fuel-vm/pull/594): Add new predicate input validation tests. Also improves error propagation so that predicate error message better reflects the reason for invalidity.
- [#596](https://github.com/FuelLabs/fuel-vm/pull/596): Remove `core::ops::{Add, Sub}` impls from `BlockHeight`. Use `succ` and `pred` to access adjacent blocks, or perform arithmetic directly on the wrapped integer instead.
- [#593](https://github.com/FuelLabs/fuel-vm/pull/593): Reworked `Mint` transaction to work with `Input::Contract` and `Output::Contract` instead of `Output::Coin`. It allows account-based fee collection for the block producer.


## [Version 0.38.0]

### Added

- [#586](https://github.com/FuelLabs/fuel-vm/pull/586): Added `default_asset` method to the `ContractIdExt` trait implementation, to mirror the `default` method on AssetId in the Sway std lib.

### Changed

#### Breaking

- [#578](https://github.com/FuelLabs/fuel-vm/pull/578): Support `no_std` environments for `fuel-crypto`, falling back to a pure-Rust crypto implementation.
- [#582](https://github.com/FuelLabs/fuel-vm/pull/582): Make `fuel-vm` and `fuel-tx` crates compatible with `no_std` + `alloc`. This includes reworking all error handling that used `std::io::Error`, replacing some `std::collection::{HashMap, HashSet}` with `hashbrown::{HashMap, HashSet}` and many changes to feature-gating of APIs.
- [#587](https://github.com/FuelLabs/fuel-vm/pull/587): Replace `thiserror` dependency with `derive_more`, so that `core::fmt::Display` is implemented without the `std` feature. Removes `std::io::Error` trait impls from the affected types.
- [#588](https://github.com/FuelLabs/fuel-vm/pull/588): Re-worked the size calculation of the canonical serialization/deserialization.

#### Removed

- [#588](https://github.com/FuelLabs/fuel-vm/pull/588): Removed `SerializedSize` and `SerializedFixedSize` traits. Removed support for `SIZE_NO_DYNAMIC` and `SIZE_STATIC`. Removed enum attributes from derive macro for `Serialize` and `Deserialize` traits.

## [Version 0.37.0]

#### Breaking

- [#573](https://github.com/FuelLabs/fuel-vm/pull/573): Added `base_asset_id` as a required field to `FeeParameters`. `base_asset_id` is used to supply the ID of the base asset. 
- [#554](https://github.com/FuelLabs/fuel-vm/pull/554): Removed `debug` feature from the `fuel-vm`. The debugger is always available and becomes active after calling any `set_*` method.
- [#537](https://github.com/FuelLabs/fuel-vm/pull/537): Use dependent cost for `k256`, `s256`, `mcpi`, `scwq`, `swwq` opcodes.
    These opcodes charged inadequately low costs in comparison to the amount of work.
    This change should make all transactions that used these opcodes much more expensive than before.
- [#533](https://github.com/FuelLabs/fuel-vm/pull/533): Use custom serialization for fuel-types to allow no_std compilation.

## [Version 0.36.1]

### Changed

- [#546](https://github.com/FuelLabs/fuel-vm/pull/546): Improve debug formatting of instruction in panic receipts.

### Fixed

- [#574](https://github.com/FuelLabs/fuel-vm/pull/574): Enforce fixed 32-byte input length for LHS and RHS inputs to the BMT's internal node sum.
- [#547](https://github.com/FuelLabs/fuel-vm/pull/547): Bump `ed25519-dalek` to `2.0.0` to deal with RustSec Advisory. 

#### Breaking
- [#524](https://github.com/FuelLabs/fuel-vm/pull/524): Fix a crash in `CCP` instruction when overflowing contract bounds. Fix a bug in `CCP` where overflowing contract bounds in a different way would not actually copy the contract bytes, but just zeroes out the section. Fix a bug in `LDC` where it would revert the transaction when the contract bounds were exceeded, when it's just supposed to fill the rest of the bytes with zeroes.


## [Version 0.36.0]

### Changed

- [#525](https://github.com/FuelLabs/fuel-vm/pull/525): The `$hp` register is no longer restored to it's previous value when returning from a call, making it possible to return heap-allocated types from `CALL`.
- [#535](https://github.com/FuelLabs/fuel-vm/pull/535): Add better test coverage for TR and TRO.

#### Breaking

- [#514](https://github.com/FuelLabs/fuel-vm/pull/514/): Add `ChainId` and `GasCosts` to `ConsensusParameters`. 
    Break down `ConsensusParameters` into sub-structs to match usage. Change signatures of functions to ask for
    necessary fields only.
- [#532](https://github.com/FuelLabs/fuel-vm/pull/532): The `TRO` instruction now reverts when attempting to send zero coins to an output. Panic reason of this `TransferZeroCoins`, and `TR` was changed to use the same panic reason as well.

### Fixed

- [#511](https://github.com/FuelLabs/fuel-vm/pull/511): Changes multiple panic reasons to be more accurate, and internally refactors instruction fetch logic to be less error-prone.

- [#529](https://github.com/FuelLabs/fuel-vm/pull/529) [#534](https://github.com/FuelLabs/fuel-vm/pull/534): Enforcing async WASM initialization for all NPM wrapper packages.

- [#531](https://github.com/FuelLabs/fuel-vm/pull/531): UtxoId::from_str and TxPointer::from_str no longer crash on invalid input with multibyte characters. Also adds clippy lints to prevent future issues.

#### Breaking

- [#527](https://github.com/FuelLabs/fuel-vm/pull/527): The balances are empty during predicate estimation/verification.

## [Version 0.35.3]

### Changed

- [#542](https://github.com/FuelLabs/fuel-vm/pull/542/): Make the `fuel-tx` WASM compatible with `serde` feature enabled.

## [Version 0.35.2]

### Changed

#### Breaking

- [#539](https://github.com/FuelLabs/fuel-vm/pull/539/): Rollbacked the change for the gas charging formula. 
    Actualized the gas prices for opcodes.

## [Version 0.35.1]

### Added

- [#499](https://github.com/FuelLabs/fuel-vm/pull/499/): The `wasm_bindgen` support of `fuel-asm` and `fuel-types`.
    Each new release also publish a typescript analog of the `fuel-asm` and `fuel-types` crates to the npm.

## [Version 0.35.0]

The release mostly fixes funding during the audit and integration with the bridge. But the release also contains some new features like:
- Asynchronous predicate estimation/verification.
- Multi-asset support per contract.
- Support Secp256r1 signature recovery and Ed25519 verificaiton.


### Added

- [#486](https://github.com/FuelLabs/fuel-vm/pull/486/): Adds `ed25519` signature verification and `secp256r1` signature recovery to `fuel-crypto`, and corresponding opcodes `ED19` and `ECR1` to `fuel-vm`.

- [#486](https://github.com/FuelLabs/fuel-vm/pull/498): Adds `PSHL`, `PSHH`, `POPH` and `POPL` instructions, which allow cheap push and pop stack operations with multiple registers.

- [#500](https://github.com/FuelLabs/fuel-vm/pull/500): Introduced `ParallelExecutor` trait
    and made available async versions of verify and estimate predicates.
    Updated tests to test for both parallel and sequential execution.
    Fixed a bug in `transaction/check_predicate_owners`.

#### Breaking

- [#506](https://github.com/FuelLabs/fuel-vm/pull/506): Added new `Mint` and `Burn` variants to `Receipt` enum.
    It affects serialization and deserialization with new variants.

### Changed

#### Breaking

- [#506](https://github.com/FuelLabs/fuel-vm/pull/506): The `mint` and `burn` 
    opcodes accept a new `$rB` register. It is a sub-identifier used to generate an 
    `AssetId` by [this rule](https://github.com/FuelLabs/fuel-specs/blob/master/src/identifiers/asset.md). 
    This feature allows having multi-asset per one contract. It is a huge breaking change, and 
    after this point, `ContractId` can't be equal to `AssetId`.

    The conversion like `AssetId::from(*contract_id)` is no longer valid. Instead, the `ContractId` implements the `ContractIdExt` trait:
    ```rust
    /// Trait extends the functionality of the `ContractId` type.
    pub trait ContractIdExt {
        /// Creates an `AssetId` from the `ContractId` and `sub_id`.
        fn asset_id(&self, sub_id: &Bytes32) -> AssetId;
    }
    ```

- [#506](https://github.com/FuelLabs/fuel-vm/pull/506): The `mint` and `burn` 
    opcodes affect the `receipts_root` of the `Script` transaction.

### Removed

#### Breaking

- [#486](https://github.com/FuelLabs/fuel-vm/pull/486/): Removes apparently unused `Keystore` and `Signer` traits from `fuel-crypto`. Also renames `ECR` opcode to `ECK1`.

### Fixed

- [#500](https://github.com/FuelLabs/fuel-vm/pull/500): Fixed a bug where `MessageCoinPredicate` wasn't checked for in `check_predicate_owners`.

#### Breaking

- [#502](https://github.com/FuelLabs/fuel-vm/pull/502): The algorithm used by the
    binary Merkle tree for generating Merkle proofs has been updated to remove
    the leaf data from the proof set. This change allows BMT proofs to conform
    to the format expected by the Solidity contracts used for verifying proofs.

- [#503](https://github.com/FuelLabs/fuel-vm/pull/503): Use correct amount of gas in call
    receipts when limited by cgas. Before this change, the `Receipt::Call` could show an incorrect value for the gas limit.

- [#504](https://github.com/FuelLabs/fuel-vm/pull/504): The `CROO` and `CSIZ` opcodes require 
    the existence of corresponding `ContractId` in the transaction's 
    inputs(the same behavior as for the `CROO` opcode).

- [#504](https://github.com/FuelLabs/fuel-vm/pull/504): The size of the contract 
    was incorrectly padded. It affects the end of the call frame in the memory, 
    making it not 8 bytes align. Also, it affects the cost of the contract 
    call(in some cases, we charged less in some more).

- [#504](https://github.com/FuelLabs/fuel-vm/pull/504): The charging for `DependentCost`
    was done incorrectly, devaluing the `dep_per_unit` part. After the fixing of 
    this, the execution should become much more expensive.

- [#505](https://github.com/FuelLabs/fuel-vm/pull/505): The `data` field of the `Receipt` 
    is not part of the canonical serialization and deserialization anymore. The SDK should use the 
    `Receipt` type instead of `OpaqueReceipt`. The `Receipt.raw_payload` will be removed for the 
    `fuel-core 0.20`. The `data` field is optional now. The SDK should update serialization and 
    deserialization for `MessageOut`, `LogData`, and `ReturnData` receipts.

- [#505](https://github.com/FuelLabs/fuel-vm/pull/505): The `len` field of the `Receipt` 
    is not padded anymore and represents an initial value.

## [Version 0.34.1]

Mainly new opcodes prices and small performance improvements in the `BinaryMerkleTree`.

### Changed

- [#492](https://github.com/FuelLabs/fuel-vm/pull/492): Minor improvements to BMT
    internals, including a reduction in usage of `Box`, using `expect(...)` over
    `unwrap()`, and additional comments.

#### Breaking

- [#493](https://github.com/FuelLabs/fuel-vm/pull/493): The default `GasCostsValues`
    is updated according to the benches with `fuel-core 0.19`. 
    It may break some unit tests that compare actual gas usage with expected.

## [Version 0.34.0]

This release contains fixes for critical issues that we found before the audit. 
Mainly, these changes pertain to the Sparse Merkle Tree (SMT) and related 
code. The SMT API was extended to provide more flexibility and to allow users 
to select the most appropriate method for their performance needs. Where 
possible, sequential SMT updates were replaced with constructors that take in a
complete data set.

### Added

- [#476](https://github.com/FuelLabs/fuel-vm/pull/476): The `fuel_vm::Call` supports `From<[u8; Self::LEN]>` and `Into<[u8; Self::LEN]>`.

- [#484](https://github.com/FuelLabs/fuel-vm/pull/484): The `sparse::in_memory::MerkleTree`
    got new methods `from_set`, `root_from_set`, and `nodes_from_set` methods. These methods allow
    a more optimal way to build and calculate the SMT when you know all leaves.
    The `Contract::initial_state_root` is much faster now (by ~15 times).

### Removed

- [#478](https://github.com/FuelLabs/fuel-vm/pull/478): The `CheckedMemRange` is replaced by the `MemoryRange`.

### Changed

- [#477](https://github.com/FuelLabs/fuel-vm/pull/477): The `PanicReason::UnknownPanicReason` is `0x00`.
    The `PanicReason` now implements `From<u8>` instead of `TryFrom<u8>` and can't return an error anymore.

- [#478](https://github.com/FuelLabs/fuel-vm/pull/478): The `memcopy` method is updated
    and returns `MemoryWriteOverlap` instead of `MemoryOverflow`.

### Fixed

- [#482](https://github.com/FuelLabs/fuel-vm/pull/482): This PR address a security 
    issue where updates to a Sparse Merkle Tree could deliberately overwrite existing
    leaves by setting the leaf key to the hash of an existing leaf or node. This is 
    done by removing the insertion of the leaf using the leaf key.

- [#484](https://github.com/FuelLabs/fuel-vm/pull/484): Fixed bug with not-working `CreateMetadata`.


#### Breaking

- [#473](https://github.com/FuelLabs/fuel-vm/pull/473): CFS and CFSI were not validating
    that the new `$sp` value isn't below `$ssp`, allowing write access to non-owned
    memory. This is now fixed, and attempting to set an incorrect `$sp` value panics.

- [#485](https://github.com/FuelLabs/fuel-vm/pull/485): This PR addresses a security
    issue where the user may manipulate the structure of the Sparse Merkle Tree. 
    SMT expects hashed storage key wrapped into a `MerkleTreeKey` structure. 
    The change is breaking because it changes the `state_root` generated by the SMT 
    and may change the `ContractId` if the `Create` transaction has non-empty `StoargeSlot`s.


## [Version 0.33.0]

The release contains a lot of breaking changes. 
Most of them are audit blockers and affect the protocol itself.
Starting this release we plan to maintain the changelog file and describe all minor and major changes that make sense.

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

- [#462](https://github.com/FuelLabs/fuel-vm/pull/462): Adds a `cache` parameter to `Input::check` and `Input::check_signature`.
  This is used to avoid redundant signature recovery when multiple inputs share the same witness index.

### Changed

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): Automatically sort storage slots for creation transactions.

#### Breaking

- [#386](https://github.com/FuelLabs/fuel-vm/pull/386): Several methods of the `TransactionFee` are renamed `total` -> `max_fee`
  and `bytes` -> `min_fee`. The `TransactionFee::min_fee` take into account the gas used by predicates.

- [#450](https://github.com/FuelLabs/fuel-vm/pull/450): The Merkle root of a contract's code is now calculated by partitioning the code into chunks of 16 KiB, instead of 8 bytes. If the last leaf is does not a full 16 KiB, it is padded with `0` up to the nearest multiple of 8 bytes. This affects the `ContractId` and `PredicateId` calculations, breaking all code that used hardcoded values.

- [#456](https://github.com/FuelLabs/fuel-vm/pull/456): The basic methods `UniqueIdentifier::id`, `Signable::sign_inputs`, 
and `Input::predicate_owner` use `ChainId` instead of the `ConsensusParameters`. 
It is a less strict requirement than before because you can get `ChainId` 
from `ConsensusParameters.chain_id`, and it makes the API cleaner. 
It affects all downstream functions that use listed methods.

- [#463](https://github.com/FuelLabs/fuel-vm/pull/463): Moves verification that the `Output::ContractCreated` 
output contains valid `contract_id` and `state_root`(the values from the `Output` match with calculated 
values from the bytecode, storage slots, and salt) from `fuel-vm` to `fuel-tx`. 
It means the end-user will receive this error earlier on the SDK side before `dry_run` instead of after.

### Fixed

#### Breaking

- [#457](https://github.com/FuelLabs/fuel-vm/pull/457): Transactions got one more validity rule: 
Each `Script` or `Create` transaction requires at least one input coin or message to be spendable. 
It may break code/tests that previously didn't set any spendable inputs. 
Note: `Message` with non-empty `data` field is not spendable.

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): The storage slots with the same key inside the `Create` transaction are forbidden.
