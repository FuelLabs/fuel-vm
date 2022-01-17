# fuel-vm

[![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-vm?label=latest)](https://crates.io/crates/fuel-vm)
[![docs](https://docs.rs/fuel-vm/badge.svg)](https://docs.rs/fuel-vm/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Rust interpreter for the Fuel Virtual Machine


## Introduction on functionalities of VM

...

### Bundler (BundleTransactor)

It has ability to bundle execution of multiple transactions. It contains 1. list of transactions 2. their outputs (or spend outputs from DB), 3. updated contract storage and. Internaly it used `Transactor` for transaction execution and depending if execution is successful or not it have function to `commit` pending transaction to the bundle.

Most notably our `SubStorage` that is used inside `Bundler`/`Transactor` contains three slices of storage data:
1. DB connector that need to implemented `InterpreterStorage` trait
2. Commited storage that represent changes done and commited by previous transactions.
3. Pending storage for current execution of transaction, this storage can be discarded if transaction execution fails or needs to be reverted or when we want to abandon transaction for any other reason.

Another idea found inside Bundler is `pre_checked_output`. When we want to transact some transaction it is assumed that we have already prechecked some outputs and for most of them (Only VariableOutput is a problem) we know if it can be used by transaction input. Additionaly, this is a good way to precheck with database UTXO and skip need for Bundler to query database for utxo data.

Bundler contains list of used outputs and have info if outputs are from db or from previously execution transaction from bundle or if they are unspend. With this we can extract full list UTXO that needs to be slashed from DB and UTXO that needs to become available for new state.
