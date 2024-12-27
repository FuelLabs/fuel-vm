# Fuzz test for the Fuel VM
This crate provides the `grammar_aware_advanced` fuzz target which can be run with `cargo fuzz` to fuzz test the Fuel VM.

General information about fuzzing Rust can be found on [appsec.guide](https://appsec.guide/docs/fuzzing/rust/cargo-fuzz/).

### Installation
The fuzzer requires nightly rust and works with rustc version `1.82.0-nightly`. To be able to run the fuzzer, the following tools must be installed.

Install:
```
cargo install cargo-fuzz
apt install clang pkg-config libssl-dev # for LibAFL
rustup component add llvm-tools-preview --toolchain nightly
```

### Seeds

The input to the fuzzer is a byte vector that contains script assembly, script data, and the assembly of a contract to be called. Each of these is separated by a 64-bit magic value `0x00ADBEEF5566CEAA`.

While the fuzzer can be started without any seeds, it is recommended to generate seeds from compiled sway programs.

#### Generate your own seeds

If you want to run the fuzzer with custom input, you can run the `seed` binary against a directory of compiled sway programs.

```
cargo run --bin seed <input dir> <output dir>
```

#### Example: Generating a corpus from the sway examples
This section explains how to use the [sway examples](https://github.com/FuelLabs/sway/tree/master/examples) to generate an initial corpus.

This can be acieved by doing the following:

1. Compile the sway examples with `forc`.
```
# In sway/examples
forc build
```

2. Gather all the resulting binaries in a temporary directory (for example `/tmp/corpus`).
```
# In sway/examples
for file in $(find . -name "*.bin" | rg debug); do cp $file /tmp/corpus; done
```

3. Run the `seed binary` against the generated binaries
```
# In fuel-vm/fuel-vm/fuzz
mkdir generated_seeds
cargo run --bin seed /tmp/corpus ./generated_seeds
```

Now the directory `./generated_seeds` contains the newly generated seeds. Copy this over to `corpus/grammar_aware_advanced` to run the fuzzer with these seeds.

### Running the Fuzzer
The Rust nightly version is required for executing cargo-fuzz. The simplest way to run the fuzzer is to run the following command:
```
cargo +nightly fuzz run grammar_aware_advanced
```

However, we recommend adding a few flags to the command to improve fuzzing efficiency. First, we can add `--no-default-features --features libafl` to ensure we use the LibAFL fuzzer instead of the default libFuzzer. Secondly, we can set `--sanitizer none` to disable AddressSanitizer for a significant speed improvement, as we do not expect memory issues in a Rust program that does not use a significant amount of unsafe code. This has been confirmed by a ToB [cargo-geiger](https://github.com/rust-secure-code/cargo-geiger) analysis showed. It makes sense to leave AddressSanitizer turned on if we use more unsafe Rust in the future (either directly or through dependencies). Finally, the `-ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1 -fork=7` flags are useful to ensure a smooth LibAFL experience utilizing 7 cores.

Putting this together we arrive at the following command.
```
cargo +nightly fuzz run --no-default-features --features libafl --sanitizer none grammar_aware_advanced -- -ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1 -fork=7
```

### Generate Coverage
It is important to measure a fuzzing campaign’s coverage after its run. To perform this measurement, we can use tools provided by cargo-fuzz and [rustc](https://doc.rust-lang.org/stable/rustc/instrument-coverage.html). First, install [cargo-binutils](https://github.com/rust-embedded/cargo-binutils#installation). After that, execute the following command:
```
cargo +nightly fuzz coverage grammar_aware_advanced corpus/grammar_aware_advanced
```

The code coverage report can now be displayed with the following command:

```
cargo cov -- report target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/grammar_aware_advanced  -instr-profile=coverage/grammar_aware_advanced/coverage.profdata 
```

We can also generate a HTML visualization of the code coverage using the following command:

```
cargo cov -- show target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/grammar_aware_advanced --format=html -instr-profile=coverage/grammar_aware_advanced/coverage.profdata $(pwd | sed "s/fuel-vm\/fuzz//") > index.html
```

### Execute a Test Case
The fuzzing campaign will output any crashes to `artifacts/grammar_aware_advanced`. To further investigate these crashes, the `execute` binary can be used.
```
cargo run --bin execute artifacts/grammar_aware_advanced/<crash file>
```

This is useful for triaging issues.

### Collect Gas Statistics
The `collect` binary writes gas statistics to a file called gas_statistics.csv. This can be used to analyze the execution time versus gas usage on a test corpus.
```
cargo run --bin collect
```
