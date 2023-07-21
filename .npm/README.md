# WASM version of Rust Crates

You'll find all the routines to publish selected Rust crates as NPM packages here.

# Testing Locally

To get started and test things locally, you'll need to:

```shell
# install the required crates
cargo install wasm-bindgen-cli wasm-opt

# ensure you have the needed target
rustup target add wasm32-unknown-unknown

# then install deps, generate wasm files, build and test the package
pnpm install
pnpm run wasm
pnpm run build
pnpm run test
```

# Output

The above commands will give ready-to-publish packages inside `.npm/packages`.

```shell
.npm
├── .scripts
│   └── prepare-wasm-packages
├── packages
│   ├── fuel-asm   # <—— package #1
│   └── fuel-types # <—— package #2
└── package.json
```

# How does it work?

For an in-depth understanding, check:
 - `.npm/package.json`
 - `.npm/.scripts/prepare-wasm-packages.sh`
 - `.github/workflows/ci.yml`
    - _Look for the `publish_wasm_packages` job_
