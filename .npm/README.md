# WASM version of Rust Crates

In order to test things locally, first install:

```shell
cargo install wasm-bindgen-cli wasm-opt
```

Then ensure you have the needed target:

```shell
rustup target add wasm32-unknown-unknown
```

Finally, run these inside the `.npm` directory:

```
pnpm install
pnpm run wasm
pnpm run build
pnpm run test
```

This should output ready-to-publish packages inside of `.npm/packages`.
