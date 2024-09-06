# Fuzz test for the Fuel VM
This crate provides the `grammar_aware_advanced` fuzz target which can be run with `cargo fuzz` to fuzz test the Fuel VM.

General information about fuzzing Rust can be found on [appsec.guide](https://appsec.guide/docs/fuzzing/rust/cargo-fuzz/).

### Installation
To be able to run the fuzzer, the following tools must be installed.

Install:
```
cargo install cargo-fuzz
apt install clang pkg-config libssl-dev # for LibAFL
rustup component add llvm-tools-preview --toolchain nightly
```

### Seeds

The input to the fuzzer is a byte vector that contains script assembly, script data, and the assembly of a contract to be called. Each of these is separated by a 64-bit magic value `0x00ADBEEF5566CEAA`.

An initial input is provided in the `example_corpus` directory, and can be loaded by copying it over to the `corpus/grammar_aware_advanced` folder:

```
mkdir -p corpus/grammar_aware_advanced
cp example_corpus/ corpus/grammar_aware_advanced
```

This corpus is generated from the [sway examples](https://github.com/FuelLabs/sway/tree/master/examples), according to the procedure described below.

#### Generate your own seeds

If you want to run the fuzzer with custom input, you can run the `seed` binary against a directory of compiled sway programs.

```
cargo run --bin seed <input dir> <output dir>
```

#### Example: Regenerate the `example_corpus`
The `example_corpus` can be recreated by doing the following:

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

### Running the Fuzzer
The Rust nightly version is required for executing cargo-fuzz. We also disable AddressSanitizer for a significant speed improvement, as we do not expect memory issues in a Rust program that does not use a significant amount of unsafe code, which the ToB [cargo-geiger](https://github.com/rust-secure-code/cargo-geiger) analysis showed. It makes sense to leave AddressSanitizer turned on if we use more unsafe Rust in the future (either directly or through dependencies). The remaining flags are either required for LibAFL or are useful to make it use seven cores.
```
cargo +nightly fuzz run --sanitizer none grammar_aware_advanced -- -ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1 -fork=7
```

### Execute a Test Case
Test cases can be executed using the following command. This is useful for triaging issues.
```
cargo run --bin execute <file/dir>
```

### Collect Statistics
ToB created a tool that writes gas statistics to a file called gas_statistics.csv. This can be used to analyze the execution time versus gas usage on a test corpus.
```
cargo run --bin collect
```

### Generate Coverage
Regardless of how inputs are generated, it is important to measure a fuzzing campaign’s coverage after its run. To perform this measure, we used the support provided by cargo-fuzz and [rustc](https://doc.rust-lang.org/stable/rustc/instrument-coverage.html). First, install [cargo-binutils](https://github.com/rust-embedded/cargo-binutils#installation). After that, execute the following command:
```
cargo +nightly fuzz coverage grammar_aware corpus/grammar_aware
```
Finally, generate an HTML file using LLVM:

```
cargo cov -- show
target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/grammar_aware --format=html -instr-profile=coverage/grammar_aware/coverage.profdata /root/audit/fuel-vm > index.html
```
