#!/usr/bin/env bash


build_and_publish_wasm_pkg ()
{
  dash_name=$1
  underscore_name=$(echo "${dash_name}" | sed -r 's/-/_/g')

  rm -rf .npm/${dash_name}/{src,dist}

  # building & optimizing wasm
  cargo rustc -p ${dash_name} --target wasm32-unknown-unknown --features typescript --crate-type=cdylib --release
  wasm-bindgen --target web ./target/wasm32-unknown-unknown/release/${underscore_name}.wasm --out-dir .npm/${dash_name}/src
  wasm-opt .npm/${dash_name}/src/${underscore_name}_bg.wasm -o .npm/${dash_name}/src/${underscore_name}_bg.wasm -Oz

  # creating entrypoint for loading everything
cat > .npm/${dash_name}/src/index.js <<EOM
  import init from './${underscore_name}.js'
  import wasm from './${underscore_name}_bg.wasm'

  init(wasm())

  export * from './${underscore_name}.js'
EOM

  # commenting `new URL()` and `fetch()` calls (plays nice with other bundlers)
  sed -i.bkp -r 's;(input = new URL.+);//\1;g' .npm/${dash_name}/src/${underscore_name}.js
  sed -i.bkp -r 's;(input = fetch.+);//\1;g' .npm/${dash_name}/src/${underscore_name}.js

  # removing backup file (after sed replacement)
  rm .npm/${dash_name}/src/${underscore_name}.js.bkp

  # building and testing npm package
  pnpm -C .npm/${dash_name} install
  pnpm -C .npm/${dash_name} build
  pnpm -C .npm/${dash_name} test

  # TODO: publish logic will go here...
}


build_and_publish_wasm_pkg "fuel-asm"
build_and_publish_wasm_pkg "fuel-types"
