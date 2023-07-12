#!/usr/bin/env bash

# cleaning up
rm -rf .npm/fuel-asm/{src,dist}


# building & optimizing wasm
cargo rustc -p fuel-asm --target wasm32-unknown-unknown --features typescript --crate-type=cdylib --release
wasm-bindgen --target web ./target/wasm32-unknown-unknown/release/fuel_asm.wasm --out-dir .npm/fuel-asm/src
wasm-opt .npm/fuel-asm/src/fuel_asm_bg.wasm -o .npm/fuel-asm/src/fuel_asm_bg.wasm -Oz


# creating entrypoint for loading everything
cat >.npm/fuel-asm/src/index.js <<EOL
import init from './fuel_asm.js'
import wasm from './fuel_asm_bg.wasm'

init(wasm())

export * from './fuel_asm.js'
EOL


# commenting `new URL()` and `fetch()` calls (plays nice with other bundlers)
sed -i.bkp -r 's;(input = new URL.+);//\1;g' .npm/fuel-asm/src/fuel_asm.js
sed -i.bkp -r 's;(input = fetch.+);//\1;g' .npm/fuel-asm/src/fuel_asm.js


# removing backup file (after sed replacement)
rm .npm/fuel-asm/src/fuel_asm.js.bkp


# building and testing npm package
pnpm -C .npm/fuel-asm install
pnpm -C .npm/fuel-asm build
pnpm -C .npm/fuel-asm test
