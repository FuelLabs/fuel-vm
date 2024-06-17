
## Manual for grammar_aware_advanced fuzzer


Install:
```
cargo install cargo-fuzz
apt install clang pkg-config libssl-dev # for LibAFL
rustup component add llvm-tools-preview --toolchain nightly
```

General information about fuzzing Rust might be found on [appsec.guide](https://appsec.guide/docs/fuzzing/rust/cargo-fuzz/).


### Generate Seeds

It is necessary to first convert Sway programs into a suitable format for use as seed input to the fuzzer. This can be done with the following command:
```
cargo run --bin seed <input dir> <output dir>
```

### Running the Fuzzer
The Rust nightly version is required for executing cargo-fuzz. We also disable AddressSanitizer for a significant speed improvement, as we do not expect memory issues in a Rust program that does not use a significant amount of unsafe code, which our [cargo-geiger](https://github.com/rust-secure-code/cargo-geiger) analysis showed. It makes sense to leave AddressSanitizer turned on if the Fuel project uses more unsafe Rust in the future (either directly or through dependencies). The remaining flags are either required for LibAFL or are useful to make it use seven cores.
```
cargo +nightly fuzz run --sanitizer none grammar_aware -- \
	-ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1 -fork=7
```

If you use libfuzzer (default) then the following command is enough:

```
cargo fuzz run --sanitizer none grammar_aware
```

### Execute a Test Case
Test cases can be executed using the following command. This is useful for triaging issues.
```
cd fuzz/
cargo run --bin execute <file/dir>
```

### Collect Statistics
We created a tool that writes gas statistics to a file called gas_statistics.csv. This can be used to analyze the execution time versus gas usage on a test corpus.
```
cargo run --bin collect <file/dir>
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
