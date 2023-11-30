# WASM version of Rust Crates

You'll find all the routines to publish selected Rust crates as NPM packages here.

# Usage

The external usage of WASM packages requires them to be async.

Don't forget to await for the WASM initialization:

```ts
import * as asm from '@fuels/vm-asm'

// alternative 1
(async function() {
  await asm.initWasm();

  asm.movi(0x10, 0);

})();

// alternative 2
import * as asm from '@fuels/vm-asm'

asm.initWasm().then(() => {
    asm.movi(0x10, 0);
})

```


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
│   ├── fuel-types # <—— package #2
│   └── ...
└── package.json
```

# How does it work?

For an in-depth understanding, check:
 - `.npm/package.json`
 - `.npm/.scripts/prepare-wasm-packages.sh`
 - `.github/workflows/ci.yml`
    - _Look for the `publish_wasm_packages` job_

# Using local sym-linked packages

First, we need to link our `packages` package globally in our local `global pnpm store`:

```sh
cd packages/fuel-asm
pnpm link -g
```

Let's check it out:

```sh
pnpm list -g
```

You should see something like:

```
@fuels/vm-asm link:<...>/fuel-vm/.npm/packages/fuel-asm
```

Cool, now on the root directory of your desired project `my-local-project`:

```sh
cd my-local-project
pnpm link -g @fuels/vm-asm
```

Done — you're using the local version of `@fuels/vm-asm`.
