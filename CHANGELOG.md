# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added
- [#838](https://github.com/FuelLabs/fuel-vm/pull/838): Implemented `AsRef<[u8]>` and `TryFrom<&[u8]>` for DA compression types: ScriptCode, PredicateCode, RegistryKey.

## [Version 0.57.1]

### Fixed
- [#835](https://github.com/FuelLabs/fuel-vm/pull/835): Fixing WASM-NPM packaging and publishing

## [Version 0.57.0]

### Added
- [#670](https://github.com/FuelLabs/fuel-vm/pull/670): Add DA compression functionality to `Transaction` and any types within
- [#733](https://github.com/FuelLabs/fuel-vm/pull/733): Add LibAFL based fuzzer and update `secp256k1` version to 0.29.1.
- [#825](https://github.com/FuelLabs/fuel-vm/pull/733): Avoid leaking partially allocated memory when array deserialization fails

### Changed
- [#824](https://github.com/FuelLabs/fuel-vm/pull/824): Use `self` instead of `&self` during decompression.
- [#823](https://github.com/FuelLabs/fuel-vm/pull/823): Returned the old behaviour of the json serialization for policies.

#### Breaking
- [#826](https://github.com/FuelLabs/fuel-vm/pull/826): Skip the panic reason from canonical serialization of the panic receipt.
- [#821](https://github.com/FuelLabs/fuel-vm/pull/821): Added `block_transaction_size_limit` to `ConsensusParameters`. It adds a new `ConensusParametersV2` as a variant of the `ConsensusParameters`.
- [#670](https://github.com/FuelLabs/fuel-vm/pull/670): The `predicate` field of `fuel_tx::input::Coin` is now a wrapper struct `PredicateCode`.

### Fixed
- [#822](https://github.com/FuelLabs/fuel-vm/pull/822): Return recipient as an owner for the message inputs.

## [Version 0.56.0]

### Added
- [#796](https://github.com/FuelLabs/fuel-vm/pull/796): Added implementation of the `MerkleRootStorage` for references.

### Changed
- [#806](https://github.com/FuelLabs/fuel-vm/pull/806): Update MSRV to 1.79.0.

#### Breaking
- [#780](https://github.com/FuelLabs/fuel-vm/pull/780): Added `Blob` transaction, and `BSIZ` and `BLDD` instructions. Also allows `LDC` to load blobs.
- [#795](https://github.com/FuelLabs/fuel-vm/pull/795): Fixed `ed19` instruction to take variable length message instead of a fixed-length one. Changed the gas cost to be `DependentCost`.

## [Version 0.55.0]

### Added
- [#781](https://github.com/FuelLabs/fuel-vm/pull/781): Added `base_asset_id` to checked metadata.

### Changed
- [#784](https://github.com/FuelLabs/fuel-vm/pull/784): Avoid storage lookups for side nodes in the SMT.
- [#787](https://github.com/FuelLabs/fuel-vm/pull/787): Fixed charge functions to profile cost before charging.

#### Breaking
- [#783](https://github.com/FuelLabs/fuel-vm/pull/783): Remove unnecessary look up for old values by adding new methods to the `StorageMutate` trait.  The old `insert` and `remove` are now `replace` and `take`. The new `insert` and `remove` don't return a value.
- [#783](https://github.com/FuelLabs/fuel-vm/pull/783): Renamed methods of `StorageWrite` trait from `write`, `replace`, `take` to `write_bytes`, `replace_bytes`, `take_bytes`.
- [#788](https://github.com/FuelLabs/fuel-vm/pull/788): Fix truncating `sp` to `MEM_SIZE` in `grow_stack`, and allow empty writes to zero-length ranges at `$hp`.

### Fixed

#### Breaking
- [#789](https://github.com/FuelLabs/fuel-vm/pull/789): Avoid conversion into `usize` type and use `u32` or `u64` instead. The change is breaking since could return other errors for 32-bit systems.
- [#786](https://github.com/FuelLabs/fuel-vm/pull/786): Fixed the CCP opcode to charge for the length from the input arguments.
- [#785](https://github.com/FuelLabs/fuel-vm/pull/785): Require `ContractCreated` output in the `Create` transaction. The `TransactionBuilder<Create>` has a `add_contract_created` method to simplify the creation of the `ContractCreated` output for tests.


## [Version 0.54.1]

### Changed
- [#776](https://github.com/FuelLabs/fuel-vm/pull/776): Charge for max length in LDC opcode.

## [Version 0.54.0]

### Added

- [#770](https://github.com/FuelLabs/fuel-vm/pull/770): Cache contract inputs in the VM.

### Changed
- [#768](https://github.com/FuelLabs/fuel-vm/pull/768): Charge for LDC opcode before loading the contract into memory.

- [#771](https://github.com/FuelLabs/fuel-vm/pull/771): Take into account spent gas during synchronous predicates estimation.

#### Breaking
- [#769](https://github.com/FuelLabs/fuel-vm/pull/769): Use `DependentCost` for `CFE` and `CFEI` opcodes.
- [#767](https://github.com/FuelLabs/fuel-vm/pull/767): Fixed no zeroing malleable fields for `Create` transaction.
- [#765](https://github.com/FuelLabs/fuel-vm/pull/765): Corrected the gas units for WDOP and WQOP.

### Removed
- [#772](https://github.com/FuelLabs/fuel-vm/pull/772): Removed redundant `self.receipts.root()` call.

## [Version 0.53.0]

### Added

- [#751](https://github.com/FuelLabs/fuel-vm/pull/751):  Improve test coverage.

### Changed

- [#753](https://github.com/FuelLabs/fuel-vm/pull/753): Fix an ownership check bug in `CCP` instruction.

## [Version 0.52.0]

### Changed

#### Breaking

- [#748](https://github.com/FuelLabs/fuel-vm/pull/748): Make `VmMemoryPool::get_new` async.
- [#747](https://github.com/FuelLabs/fuel-vm/pull/747): Use `DependentCost` for `aloc` opcode. The cost of the `aloc` opcode is now dependent on the size of the allocation.

## [Version 0.51.0]

### Added

- [#732](https://github.com/FuelLabs/fuel-vm/pull/732):  Adds `reset` method to VM memory.

#### Breaking

- [#732](https://github.com/FuelLabs/fuel-vm/pull/732): Makes the VM generic over the memory type, allowing reuse of relatively expensive-to-allocate VM memories through `VmMemoryPool`. Functions and traits which require VM initalization such as `estimate_predicates` now take either the memory or `VmMemoryPool` as an argument. The `Interpterter::eq` method now only compares accessible memory regions. `Memory` was renamed into `MemoryInstance` and `Memory` is a trait now.

### Changed

#### Breaking

- [#743](https://github.com/FuelLabs/fuel-vm/pull/743): Zeroes `$flag` on `CALL`, so that contracts can assume clean `$flag` state.
- [#737](https://github.com/FuelLabs/fuel-vm/pull/737): Panic on instructions with non-zero reserved part.

## [Version 0.50.0]

### Changed

- [#725](https://github.com/FuelLabs/fuel-vm/pull/725): Adds more clippy lints to catch possible integer overflow and casting bugs on compile time.
- [#729](https://github.com/FuelLabs/fuel-vm/pull/729): Adds more clippy lints to `fuel-merkle` to catch possible integer overflow and casting bugs on compile time. It also does some internal refactoring.

### Added

#### Breaking

- [#725](https://github.com/FuelLabs/fuel-vm/pull/725): `UtxoId::from_str` now rejects inputs with multiple `0x` prefixes. Many `::from_str` implementations also reject extra data in the end of the input, instead of silently ignoring it. `UtxoId::from_str` allows a single `:` between the fields. Unused `GasUnit` struct removed.
- [#726](https://github.com/FuelLabs/fuel-vm/pull/726): Removed code related to Binary Merkle Sum Trees (BMSTs). The BMST is deprecated and not used in production environments. 
- [#729](https://github.com/FuelLabs/fuel-vm/pull/729): Removed default implementation of `Node::key_size_bits`, implementors must now define it themselves. Also some helper traits have been merged together, or their types changed.
### Fixed

#### Breaking

- [#736](https://github.com/FuelLabs/fuel-vm/pull/736): LDC instruction now works in internal contexts as well. Call frames use code size padded to word alignment.

## [Version 0.49.0]

### Added

- [#721](https://github.com/FuelLabs/fuel-vm/pull/721): Added additional logic to the BMT proof verification algorithm to check the length of the provided proof set against the index provided in the proof.

#### Breaking

- [#719](https://github.com/FuelLabs/fuel-vm/pull/719): Fix overflow in `LDC` instruction when contract size with padding would overflow.
- [#715](https://github.com/FuelLabs/fuel-vm/pull/715): The `Interpreter` supports the processing of the `Upload` transaction. The change affects `InterpreterStorage`, adding `StorageMutate<UploadedBytes>` constrain.
- [#714](https://github.com/FuelLabs/fuel-vm/pull/714): The change adds a new `Upload` transaction that allows uploading huge byte code on chain subsection by subsection. This transaction is chargeable and is twice as expensive as the `Create` transaction. Anyone can submit this transaction.
- [#712](https://github.com/FuelLabs/fuel-vm/pull/712): The `Interpreter` supports the processing of the `Upgrade` transaction. The change affects `InterpreterStorage`, adding 5 new methods that must be implemented.
- [#707](https://github.com/FuelLabs/fuel-vm/pull/707): The change adds a new `Upgrade` transaction that allows upgrading either consensus parameters or state transition function used by the network to produce future blocks.
    The purpose of the upgrade is defined by the `Upgrade Purpose` type:
    
    ```rust
    pub enum UpgradePurpose {
        /// The upgrade is performed to change the consensus parameters.
        ConsensusParameters {
            /// The index of the witness in the [`Witnesses`] field that contains
            /// the serialized consensus parameters.
            witness_index: u16,
            /// The hash of the serialized consensus parameters.
            /// Since the serialized consensus parameters live inside witnesses(malleable
            /// data), any party can override them. The `checksum` is used to verify that the
            /// data was not modified.
            checksum: Bytes32,
        },
        /// The upgrade is performed to change the state transition function.
        StateTransition {
            /// The Merkle root of the new bytecode of the state transition function.
            /// The bytecode must be present on the blockchain(should be known by the
            /// network) at the moment of inclusion of this transaction.
            root: Bytes32,
        },
    }
    ```
    
    The `Upgrade` transaction is chargeable, and the sender should pay for it. Transaction inputs should contain only base assets.
    
    Only the privileged address can upgrade the network. The privileged address can be either a real account or a predicate.
    
    Since serialized consensus parameters are small(< 2kb), they can be part of the upgrade transaction and live inside of witness data. The bytecode of the blockchain state transition function is huge ~1.6MB(relative to consensus parameters), and it is impossible to fit it into one transaction. So when we perform the upgrade of the state transition function, it should already be available on the blockchain. The transaction to actually upload the bytecode(`Upload` transaction) will implemented in the https://github.com/FuelLabs/fuel-core/issues/1754.

### Changed

- [#707](https://github.com/FuelLabs/fuel-vm/pull/707): Used the same pattern everywhere in the codebase: 
    ```rust
                 Self::Script(tx) => tx.encode_static(buffer),
                 Self::Create(tx) => tx.encode_static(buffer),
                 Self::Mint(tx) => tx.encode_static(buffer),
                 Self::Upgrade(tx) => tx.encode_static(buffer),
    ```
  
    Instead of:
    ```rust
                 Transaction::Script(script) => script.encode_static(buffer),
                 Transaction::Create(create) => create.encode_static(buffer),
                 Transaction::Mint(mint) => mint.encode_static(buffer),
                 Transaction::Upgrade(upgrade) => upgrade.encode_static(buffer),
    ```

#### Breaking

- [#714](https://github.com/FuelLabs/fuel-vm/pull/714): Added `max_bytecode_subsections` field to the `TxParameters` to limit the number of subsections that can be uploaded.
- [#707](https://github.com/FuelLabs/fuel-vm/pull/707): Side small breaking for tests changes from the `Upgrade` transaction:
  - Moved `fuel-tx-test-helpers` logic into the `fuel_tx::test_helpers` module.
  - Added a new rule for `Create` transaction: all inputs should use base asset otherwise it returns `TransactionInputContainsNonBaseAssetId` error.
  - Renamed some errors because now they are used for several transactions(`Upgrade` uses some errors from `Create` and some from `Script` transactions):
    - `TransactionScriptOutputContractCreated` -> `TransactionOutputContainsContractCreated`.
    - `TransactionCreateOutputContract` -> `TransactionOutputContainsContract`.
    - `TransactionCreateOutputVariable` -> `TransactionOutputContainsVariable`.
    - `TransactionCreateOutputChangeNotBaseAsset` -> `TransactionChangeChangeUsesNotBaseAsset`.
    - `TransactionCreateInputContract` -> `TransactionInputContainsContract`.
    - `TransactionCreateMessageData` -> `TransactionInputContainsMessageData`.
  - The combination of `serde` and `postcard` is used to serialize and deserialize `ConsensusParameters` during the upgrade. This means the protocol and state transition function requires the `serde` feature by default for `ConsensusParameters` and `fuel-types`.

- [#697](https://github.com/FuelLabs/fuel-vm/pull/697): Changed the VM to internally use separate buffers for the stack and the heap to improve startup time. After this change, memory that was never part of the stack or the heap cannot be accessed, even for reading. Also, even if the whole memory is allocated, accesses spanning from the stack to the heap are not allowed. This PR also fixes a bug that required one-byte gap between the stack and the heap. Multiple errors have been changed to be more sensible ones, and sometimes the order of which error is returned has changed. `ALOC` opcode now zeroes the newly allocated memory.

## [Version 0.48.0]

### Added

- [#705](https://github.com/FuelLabs/fuel-vm/pull/705): Added `privileged_address` to the `ConsensusParameters` for permissioned operations(like upgrade of the network).
- [#648](https://github.com/FuelLabs/fuel-vm/pull/648): Added support for generating proofs for Sparse Merkle Trees (SMTs) and proof verification. Proofs can be used to attest to the inclusion or exclusion of data from the set.

### Changed

#### Breaking

- [#709](https://github.com/FuelLabs/fuel-vm/pull/709): Removed `bytecode_length` from the `Create` transaction.
- [#706](https://github.com/FuelLabs/fuel-vm/pull/706): Unified `Create` and `Script` logic via `ChargeableTransaction`. The change is breaking because affects JSON serialization and deserialization. Now `Script` and `Create` transactions have `body` fields that include unique transactions.
- [#703](https://github.com/FuelLabs/fuel-vm/pull/703): Reshuffled fields `Script` and `Create` transactions to unify part used by all chargeable transactions. It breaks the serialization and deserialization and requires adoption on the SDK side.
- [#708](https://github.com/FuelLabs/fuel-vm/pull/708): Hidden `Default` params under the "test-helper" feature to avoid accidental use in production code. It is a huge breaking change for any code that has used them before in production, and instead, it should be fetched from the network. In the case of tests simply use the "test-helper" feature in your `[dev-dependencies]` section.
- [#702](https://github.com/FuelLabs/fuel-vm/pull/702): Wrapped `FeeParameters`, `PredicateParameters`, `TxParameters`, `ScriptParameters` and `ContractParameters` into an enum to support versioning. 
- [#701](https://github.com/FuelLabs/fuel-vm/pull/701): Wrapped `ConsensusParameters` and `GasCosts` into an enum to support versioning. Moved `block_gas_limit` from `fuel_core_chain_config::ChainConfig` to `ConsensusPataremeters`. Reduced default `MAX_SIZE` to be [110kb](https://github.com/FuelLabs/fuel-core/pull/1761) and `MAX_CONTRACT_SIZE` to be [100kb](https://github.com/FuelLabs/fuel-core/pull/1761).
- [#692](https://github.com/FuelLabs/fuel-vm/pull/692): Add GTF getters for tx size and address.
- [#698](https://github.com/FuelLabs/fuel-vm/pull/698): Store input, output and witness limits to u16, while keeping the values limited to 255.

## [Version 0.47.1]

### Added

- [#689](https://github.com/FuelLabs/fuel-vm/pull/689): Re-add fields to the checked tx `Metadata` for min and max gas.
- [#689](https://github.com/FuelLabs/fuel-vm/pull/689): Add test helpers and additional getters.

## [Version 0.47.0]

### Added

- [#686](https://github.com/FuelLabs/fuel-vm/pull/686): Implement `serde` for `InterpreterError`.

### Changed

#### Breaking

- [#685](https://github.com/FuelLabs/fuel-vm/pull/685):
  The `MaxFee` is a mandatory policy to set. The `MaxFee` policy is used to check that the transaction is valid.
  Added a new stage for the `Checked` transaction - `Ready`. This type can be constructed with the
  `gas_price` before being transacted by the `Interpreter`.
- [#671](https://github.com/FuelLabs/fuel-vm/pull/671): Support dynamically sized values in the ContractsState table by
  using a vector data type (`Vec<u8>`).
- [#682](https://github.com/FuelLabs/fuel-vm/pull/682): Include `Tip` policy in fee calculation
- [#683](https://github.com/FuelLabs/fuel-vm/pull/683): Simplify `InterpreterStorage` by removing dependency
  on `MerkleRootStorage` and removing `merkle_` prefix from method names.
- [#678](https://github.com/FuelLabs/fuel-vm/pull/678): Zero malleable fields before execution. Remove some now-obsolete
  GTF getters. Don't update `tx.receiptsRoot` after pushing receipts, and do it after execution instead.
- [#672](https://github.com/FuelLabs/fuel-vm/pull/672): Remove `GasPrice` policy
- [#672](https://github.com/FuelLabs/fuel-vm/pull/672): Add `gas_price` field to transaction execution
- [#684](https://github.com/FuelLabs/fuel-vm/pull/684): Remove `maturity` field from `Input` coin types. Also remove
  related `GTF` getter.
- [#675](https://github.com/FuelLabs/fuel-vm/pull/675): Add `GTF` access for `asset_id` and `to` fields for `Change`
  outputs.

## [Version 0.46.0]

### Changed

#### Breaking

- [#679](https://github.com/FuelLabs/fuel-vm/pull/679): Require less restricted constraint on `MerkleRootStorage` trait.
  Now it requires `StorageInspect` instead of the `StorageMutate`.
- [#673](https://github.com/FuelLabs/fuel-vm/pull/673): Removed `ContractsInfo` table. Contract salts and roots are no
  longer stored in on-chain data.
- [#673](https://github.com/FuelLabs/fuel-vm/pull/673): Opcode `CROO` now calculates the given contract's root on
  demand. `CROO` has therefore been changed to a `DependentCost` gas cost.

### Changed

- [#672](https://github.com/FuelLabs/fuel-vm/pull/672): Add `Tip` policy

## [Version 0.45.0]

### Changed

#### Breaking

- [#668](https://github.com/FuelLabs/fuel-vm/pull/668): Remove `non_exhaustive` from versionable types for security
  reasons

## [Version 0.44.0]

#### Changed

- [#653](https://github.com/FuelLabs/fuel-vm/pull/653): `ECAL` opcode handler can now hold internal state.
- [#657](https://github.com/FuelLabs/fuel-vm/pull/657): Add debugger methods to remove or replace all breakpoints at
  once.

#### Breaking

- [#654](https://github.com/FuelLabs/fuel-vm/pull/654): Make public types versionable by making non-exhaustive.
- [#658](https://github.com/FuelLabs/fuel-vm/pull/658): Make `key!`-generated types
  like `Address`, `AssetId`, `ContractId` and `Bytes32` consume one less byte when serialized with a binary serde
  serializer like postcard.

## [Version 0.43.2]

### Changed

- [#645](https://github.com/FuelLabs/fuel-vm/pull/645): Add wasm support for `fuel-tx` crate.

## [Version 0.43.1]

### Fixed

- [#643](https://github.com/FuelLabs/fuel-vm/pull/643): Fixed json deserialization of array fuel types from the file.

## [Version 0.43.0]

### Changed

#### Breaking

- [#640](https://github.com/FuelLabs/fuel-vm/pull/640): Update VM initialization cost to dependent cost; this is
  required because the time it takes to initialize the VM depends on the size of the transaction.

## [Version 0.42.1]

### Changed

#### Breaking

- [#637](https://github.com/FuelLabs/fuel-vm/pull/637): Charge for the actual size of the contract in `ccp` opcode.

## [Version 0.42.0]

### Changed

#### Breaking

- [#676](https://github.com/FuelLabs/fuel-vm/pull/676) Add `gas_price` to `Mint` transaction
- [#629](https://github.com/FuelLabs/fuel-vm/pull/629): Charge the user for VM initialization.
- [#628](https://github.com/FuelLabs/fuel-vm/pull/628): Renamed `transaction::CheckError`
  to `transaction::ValidityError`.
  Created a new `checked_transaction::CheckError` that combines `ValidityError`
  and `PredicateVerificationFailed` errors into one. It allows the return of the
  `PredicateVerificationFailed` to the end user instead of losing the reason why predicate verification failed.
- [#625](https://github.com/FuelLabs/fuel-vm/pull/625): Use `ArithmeticError` only for arithmetic operations, and
  introduce new errors like `BalanceOverflow` for others. Whenever an error is internally caused by a type conversion
  to `usize`, so that an overflowing value wouldn't map to a valid index anyway, return the missing item error instead.
- [#623](https://github.com/FuelLabs/fuel-vm/pull/623):
  Added support for transaction policies. The `Script` and `Create`
  transactions received a new field, `policies`. Policies allow the addition
  of some limits to the transaction to protect the user or specify some details regarding execution.
  This change makes the `GasPrice` and `Maturity` fields optional, allowing to save space in the future.
  Also, this will enable us to support multidimensional prices later.
  `GasLimit` was renamed to `ScriptGasLimit`.

  Along with this change, we introduced two new policies:
    - `WitnessLimit` - allows the limitation of the maximum size of witnesses in bytes for the contract. Because of the
      changes in the gas calculation model(the blockchain also charges the user for the witness data), the user should
      protect himself from the block producer or third parties blowing up witness data and draining the user's funds.
    - `MaxFee` - allows the upper bound for the maximum fee that users agree to pay for the transaction.

  This change brings the following modification to the gas model:
    - The `ScriptGasLimit` only limits script execution. Previously, the `ScriptGasLimit` also limited the predicate
      execution time, instead predicate gas is now directly included into `min_fee`. So, it is not possible to use
      the `ScriptGasLimit` for transaction cost limitations. A new `MaxFee` policy is a way to do that. The `GasLimit`
      field was removed from the `Create` transaction because it only relates to the script execution (which
      the `Create` transaction doesn't have).
    - The blockchain charges the user for the size of witness data (before it was free). There is no separate price for
      the storage, so it uses gas to charge the user. This change affects `min_gas` and `min_fee` calculation.
    - A new policy called `WitnessLimit` also impacts the `max_gas` and `max_fee` calculation in addition
      to `ScriptGasLimit`(in the case of `Create` transaction only `WitnessLimit` affects the `max_gas` and `max_fee`).
    - The minimal gas also charges the user for transaction ID calculation.

  The change has the following modification to the transaction layout:
    - The `Create` transaction doesn't have the `ScriptGasLimit` field anymore. Because the `Create` transaction doesn't
      have any script to execute
    - The `Create` and `Script` transactions don't have explicit `maturity` and `gas_price` fields. Instead, these
      fields can be set via a new `policies` field.
    - The `Create` and `Script` transactions have a new `policies` field with a unique canonical serialization and
      deserialization for optimal space consumption.

  Other breaking changes caused by the change:
    - Each transaction requires setting the `GasPrice` policy.
    - Previously, `ScriptGasLimit` should be less than the `MAX_GAS_PER_TX` constant. After removing this field from
      the `Create` transaction, it is impossible to require it. Instead, it requires that `max_gas <= MAX_GAS_PER_TX`
      for any transaction. Consequently, any `Script` transaction that uses `MAX_GAS_PER_TX` as a `ScriptGasLimit` will
      always fail because of a new rule. Setting the estimated gas usage instead solves the problem.
    - If the `max_fee > policies.max_fee`, then transaction will be rejected.
    - If the `witnessses_size > policies.witness_limit`, then transaction will be rejected.
    - GTF opcode changed its hardcoded constants for fields. It should be updated according to the values from the
      specification on the Sway side.
- [#633](https://github.com/FuelLabs/fuel-vm/pull/633): Limit receipt count to `u16::MAX`.
- [#634](https://github.com/FuelLabs/fuel-vm/pull/634): Charge for storage per new byte written. Write opcodes now
  return the number of new storage slots created, instead of just a boolean on whether the value existed before.

### Fixed

- [#627](https://github.com/FuelLabs/fuel-vm/pull/627): Added removal of obsolete SMT nodes along the path
  during `update` and `delete` operations.

## [Version 0.41.0]

#### Breaking

- [#622](https://github.com/FuelLabs/fuel-vm/pull/622): Divide `DependentCost` into "light" and "heavy" operations:
  Light operations consume `0 < x < 1` gas per unit, while heavy operations consume `x` gas per unit. This distinction
  provides more precision when calculating dependent costs.

## [Version 0.40.0]

### Added

- [#607](https://github.com/FuelLabs/fuel-vm/pull/607): Added `ECAL` instruction support.

### Changed

- [#612](https://github.com/FuelLabs/fuel-vm/pull/612): Reduced the memory consumption in all places where we calculate
  BMT root.
- [#615](https://github.com/FuelLabs/fuel-vm/pull/615): Made `ReceiptsCtx` of the VM modifiable with `test-helpers`
  feature.

#### Breaking

- [#618](https://github.com/FuelLabs/fuel-vm/pull/618): Transaction fees for `Create` now include the cost of metadata
  calculations, including: contract root calculation, state root calculation, and contract id calculation.
- [#613](https://github.com/FuelLabs/fuel-vm/pull/613): Transaction fees now include the cost of signature verification
  for each input. For signed inputs, the cost of an EC recovery is charged. For predicate inputs, the cost of a BMT root
  of bytecode is charged.
- [#607](https://github.com/FuelLabs/fuel-vm/pull/607): The `Interpreter` expects the third generic argument during type
  definition that specifies the implementer of the `EcalHandler` trait for `ecal` opcode.
- [#609](https://github.com/FuelLabs/fuel-vm/pull/609): Checked transactions (`Create`, `Script`, and `Mint`) now
  enforce a maximum size. The maximum size is specified by `MAX_TRANSACTION_SIZE` in the transaction parameters, under
  consensus parameters. Checking a transaction above this size raises `CheckError::TransactionSizeLimitExceeded`.
- [#617](https://github.com/FuelLabs/fuel-vm/pull/617): Makes memory outside `$is..$ssp` range not executable.
  Separates `ErrorFlag` into `InvalidFlags`, `MemoryNotExecutable` and `InvalidInstruction`. Fixes related tests.
- [#619](https://github.com/FuelLabs/fuel-vm/pull/619): Avoid possible truncation of higher bits. It may invalidate the
  code that truncated higher bits causing different behavior on 32-bit vs. 64-bit systems.

## [Version 0.39.0]

### Added

- [#603](https://github.com/FuelLabs/fuel-vm/pull/603): Added `MerkleRootCalculator`for efficient in-memory Merkle root
  calculation.
- [#603](https://github.com/FuelLabs/fuel-vm/pull/606): Added Serialization and Deserialization support
  to `MerkleRootCalculator`.

### Changed

- [#595](https://github.com/FuelLabs/fuel-vm/pull/595): Removed `wee_alloc` dependency from `fuel-asm`. It now uses the
  builtin allocator on web targets as well.

#### Breaking

- [#598](https://github.com/FuelLabs/fuel-vm/pull/598): Update cost model for `ldc` opcode to take into account contract
  size.
- [#604](https://github.com/FuelLabs/fuel-vm/pull/604): Removed `ChainId` from `PredicateId` calculation. It changes the
  generated address of the predicates and may break tests or logic that uses hard-coded predicate IDs.
- [#594](https://github.com/FuelLabs/fuel-vm/pull/594): Add new predicate input validation tests. Also improves error
  propagation so that predicate error message better reflects the reason for invalidity.
- [#596](https://github.com/FuelLabs/fuel-vm/pull/596): Remove `core::ops::{Add, Sub}` impls from `BlockHeight`.
  Use `succ` and `pred` to access adjacent blocks, or perform arithmetic directly on the wrapped integer instead.
- [#593](https://github.com/FuelLabs/fuel-vm/pull/593): Reworked `Mint` transaction to work with `Input::Contract`
  and `Output::Contract` instead of `Output::Coin`. It allows account-based fee collection for the block producer.

## [Version 0.38.0]

### Added

- [#586](https://github.com/FuelLabs/fuel-vm/pull/586): Added `default_asset` method to the `ContractIdExt` trait
  implementation, to mirror the `default` method on AssetId in the Sway std lib.

### Changed

#### Breaking

- [#578](https://github.com/FuelLabs/fuel-vm/pull/578): Support `no_std` environments for `fuel-crypto`, falling back to
  a pure-Rust crypto implementation.
- [#582](https://github.com/FuelLabs/fuel-vm/pull/582): Make `fuel-vm` and `fuel-tx` crates compatible
  with `no_std` + `alloc`. This includes reworking all error handling that used `std::io::Error`, replacing
  some `std::collection::{HashMap, HashSet}` with `hashbrown::{HashMap, HashSet}` and many changes to feature-gating of
  APIs.
- [#587](https://github.com/FuelLabs/fuel-vm/pull/587): Replace `thiserror` dependency with `derive_more`, so
  that `core::fmt::Display` is implemented without the `std` feature. Removes `std::io::Error` trait impls from the
  affected types.
- [#588](https://github.com/FuelLabs/fuel-vm/pull/588): Re-worked the size calculation of the canonical
  serialization/deserialization.
- [#700](https://github.com/FuelLabs/fuel-vm/pull/700): Add `BASE_ASSET_ID` to `GM` instruction.


#### Removed

- [#588](https://github.com/FuelLabs/fuel-vm/pull/588): Removed `SerializedSize` and `SerializedFixedSize` traits.
  Removed support for `SIZE_NO_DYNAMIC` and `SIZE_STATIC`. Removed enum attributes from derive macro for `Serialize`
  and `Deserialize` traits.

## [Version 0.37.0]

#### Breaking

- [#573](https://github.com/FuelLabs/fuel-vm/pull/573): Added `base_asset_id` as a required field
  to `FeeParameters`. `base_asset_id` is used to supply the ID of the base asset.
- [#554](https://github.com/FuelLabs/fuel-vm/pull/554): Removed `debug` feature from the `fuel-vm`. The debugger is
  always available and becomes active after calling any `set_*` method.
- [#537](https://github.com/FuelLabs/fuel-vm/pull/537): Use dependent cost for `k256`, `s256`, `mcpi`, `scwq`, `swwq`
  opcodes.
  These opcodes charged inadequately low costs in comparison to the amount of work.
  This change should make all transactions that used these opcodes much more expensive than before.
- [#533](https://github.com/FuelLabs/fuel-vm/pull/533): Use custom serialization for fuel-types to allow no_std
  compilation.

## [Version 0.36.1]

### Changed

- [#546](https://github.com/FuelLabs/fuel-vm/pull/546): Improve debug formatting of instruction in panic receipts.

### Fixed

- [#574](https://github.com/FuelLabs/fuel-vm/pull/574): Enforce fixed 32-byte input length for LHS and RHS inputs to the
  BMT's internal node sum.
- [#547](https://github.com/FuelLabs/fuel-vm/pull/547): Bump `ed25519-dalek` to `2.0.0` to deal with RustSec Advisory.

#### Breaking

- [#524](https://github.com/FuelLabs/fuel-vm/pull/524): Fix a crash in `CCP` instruction when overflowing contract
  bounds. Fix a bug in `CCP` where overflowing contract bounds in a different way would not actually copy the contract
  bytes, but just zeroes out the section. Fix a bug in `LDC` where it would revert the transaction when the contract
  bounds were exceeded, when it's just supposed to fill the rest of the bytes with zeroes.

## [Version 0.36.0]

### Changed

- [#525](https://github.com/FuelLabs/fuel-vm/pull/525): The `$hp` register is no longer restored to it's previous value
  when returning from a call, making it possible to return heap-allocated types from `CALL`.
- [#535](https://github.com/FuelLabs/fuel-vm/pull/535): Add better test coverage for TR and TRO.

#### Breaking

- [#514](https://github.com/FuelLabs/fuel-vm/pull/514/): Add `ChainId` and `GasCosts` to `ConsensusParameters`.
  Break down `ConsensusParameters` into sub-structs to match usage. Change signatures of functions to ask for
  necessary fields only.
- [#532](https://github.com/FuelLabs/fuel-vm/pull/532): The `TRO` instruction now reverts when attempting to send zero
  coins to an output. Panic reason of this `TransferZeroCoins`, and `TR` was changed to use the same panic reason as
  well.

### Fixed

- [#511](https://github.com/FuelLabs/fuel-vm/pull/511): Changes multiple panic reasons to be more accurate, and
  internally refactors instruction fetch logic to be less error-prone.

- [#529](https://github.com/FuelLabs/fuel-vm/pull/529) [#534](https://github.com/FuelLabs/fuel-vm/pull/534): Enforcing
  async WASM initialization for all NPM wrapper packages.

- [#531](https://github.com/FuelLabs/fuel-vm/pull/531): UtxoId::from_str and TxPointer::from_str no longer crash on
  invalid input with multibyte characters. Also adds clippy lints to prevent future issues.

#### Breaking

- [#527](https://github.com/FuelLabs/fuel-vm/pull/527): The balances are empty during predicate estimation/verification.

## [Version 0.35.3]

### Changed

- [#542](https://github.com/FuelLabs/fuel-vm/pull/542/): Make the `fuel-tx` WASM compatible with `serde` feature
  enabled.

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

The release mostly fixes funding during the audit and integration with the bridge. But the release also contains some
new features like:

- Asynchronous predicate estimation/verification.
- Multi-asset support per contract.
- Support Secp256r1 signature recovery and Ed25519 verificaiton.

### Added

- [#486](https://github.com/FuelLabs/fuel-vm/pull/486/): Adds `ed25519` signature verification and `secp256r1` signature
  recovery to `fuel-crypto`, and corresponding opcodes `ED19` and `ECR1` to `fuel-vm`.

- [#486](https://github.com/FuelLabs/fuel-vm/pull/498): Adds `PSHL`, `PSHH`, `POPH` and `POPL` instructions, which allow
  cheap push and pop stack operations with multiple registers.

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

  The conversion like `AssetId::from(*contract_id)` is no longer valid. Instead, the `ContractId` implements
  the `ContractIdExt` trait:
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

- [#486](https://github.com/FuelLabs/fuel-vm/pull/486/): Removes apparently unused `Keystore` and `Signer` traits
  from `fuel-crypto`. Also renames `ECR` opcode to `ECK1`.

### Fixed

- [#500](https://github.com/FuelLabs/fuel-vm/pull/500): Fixed a bug where `MessageCoinPredicate` wasn't checked for
  in `check_predicate_owners`.

#### Breaking

- [#502](https://github.com/FuelLabs/fuel-vm/pull/502): The algorithm used by the
  binary Merkle tree for generating Merkle proofs has been updated to remove
  the leaf data from the proof set. This change allows BMT proofs to conform
  to the format expected by the Solidity contracts used for verifying proofs.

- [#503](https://github.com/FuelLabs/fuel-vm/pull/503): Use correct amount of gas in call
  receipts when limited by cgas. Before this change, the `Receipt::Call` could show an incorrect value for the gas
  limit.

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

- [#476](https://github.com/FuelLabs/fuel-vm/pull/476): The `fuel_vm::Call` supports `From<[u8; Self::LEN]>`
  and `Into<[u8; Self::LEN]>`.

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

- [#456](https://github.com/FuelLabs/fuel-vm/pull/456): Added a new type - `ChainId` to represent the identifier of the
  chain.
  It is a wrapper around the `u64`, so any `u64` can be converted into this type via `.into()` or `ChainId::new(...)`.

- [#459](https://github.com/FuelLabs/fuel-vm/pull/459) Require witness index to be specified when adding an unsigned
  coin to a transaction.
  This allows for better reuse of witness data when using the transaction builder and helper methods to make
  transactions compact.

- [#462](https://github.com/FuelLabs/fuel-vm/pull/462): Adds a `cache` parameter to `Input::check`
  and `Input::check_signature`.
  This is used to avoid redundant signature recovery when multiple inputs share the same witness index.

### Changed

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): Automatically sort storage slots for creation transactions.

#### Breaking

- [#386](https://github.com/FuelLabs/fuel-vm/pull/386): Several methods of the `TransactionFee` are
  renamed `total` -> `max_fee`
  and `bytes` -> `min_fee`. The `TransactionFee::min_fee` take into account the gas used by predicates.

- [#450](https://github.com/FuelLabs/fuel-vm/pull/450): The Merkle root of a contract's code is now calculated by
  partitioning the code into chunks of 16 KiB, instead of 8 bytes. If the last leaf is does not a full 16 KiB, it is
  padded with `0` up to the nearest multiple of 8 bytes. This affects the `ContractId` and `PredicateId` calculations,
  breaking all code that used hardcoded values.

- [#456](https://github.com/FuelLabs/fuel-vm/pull/456): The basic
  methods `UniqueIdentifier::id`, `Signable::sign_inputs`,
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

- [#458](https://github.com/FuelLabs/fuel-vm/pull/458): The storage slots with the same key inside the `Create`
  transaction are forbidden.
