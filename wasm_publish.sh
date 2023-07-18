#!/usr/bin/env bash

write_template ()
{
  FROM=$1
  TO=$2
  NAME_DASHED=$3
  NAME_UNDERSCORED=$4

  echo "$(cat ${FROM} | sed -r "s/{{NAME_DASHED}}/${NAME_DASHED}/g" | sed -r "s/{{NAME_UNDERSCORED}}/${NAME_UNDERSCORED}/g")" > ${TO}

}


build_and_publish_wasm_pkg ()
{
  NAME_DASHED=$1
  NAME_UNDERSCORED=$(echo "${NAME_DASHED}" | sed -r 's/-/_/g')

  rm -rf .npm/${NAME_DASHED}/{src,dist}

  cargo rustc -p ${NAME_DASHED} --target wasm32-unknown-unknown --features typescript --crate-type=cdylib --release
  wasm-bindgen --target web ./target/wasm32-unknown-unknown/release/${NAME_UNDERSCORED}.wasm --out-dir .npm/${NAME_DASHED}/src
  wasm-opt .npm/${NAME_DASHED}/src/${NAME_UNDERSCORED}_bg.wasm -o .npm/${NAME_DASHED}/src/${NAME_UNDERSCORED}_bg.wasm -Oz

  write_template .npm/.templates/README.md .npm/${NAME_DASHED}/README.md ${NAME_DASHED} ${NAME_UNDERSCORED}
  write_template .npm/.templates/package.json .npm/${NAME_DASHED}/package.json ${NAME_DASHED} ${NAME_UNDERSCORED}
  write_template .npm/.templates/pnpm-lock.yaml .npm/${NAME_DASHED}/pnpm-lock.yaml ${NAME_DASHED} ${NAME_UNDERSCORED}
  write_template .npm/.templates/rollup.config.mjs .npm/${NAME_DASHED}/rollup.config.mjs ${NAME_DASHED} ${NAME_UNDERSCORED}
  write_template .npm/.templates/index.js .npm/${NAME_DASHED}/src/index.js ${NAME_DASHED} ${NAME_UNDERSCORED}

  # commenting out all `new URL()` and `fetch()` calls for great compatibility with JS bundlers
  sed -i.bkp -r 's;(input = new URL.+);//\1;g' .npm/${NAME_DASHED}/src/${NAME_UNDERSCORED}.js
  sed -i.bkp -r 's;(input = fetch.+);//\1;g' .npm/${NAME_DASHED}/src/${NAME_UNDERSCORED}.js
  rm .npm/${NAME_DASHED}/src/${NAME_UNDERSCORED}.js.bkp

  pnpm -C .npm/${NAME_DASHED} install
  pnpm -C .npm/${NAME_DASHED} build
  pnpm -C .npm/${NAME_DASHED} test

  # TODO: publish logic will go here
}


build_and_publish_wasm_pkg "fuel-asm"
build_and_publish_wasm_pkg "fuel-types"
