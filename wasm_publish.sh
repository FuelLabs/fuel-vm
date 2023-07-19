#!/usr/bin/env bash

write_template ()
{
  NAME_DASHED=$1
  NAME_UNDERSCORED=$2
  TEMPLATE=$3

  PKG_TEMPLATE_DIR=.npm/.pkg-template
  PACKAGE_DIR=.npm/${NAME_DASHED}

  VERSION=$(cat ./Cargo.toml | sed -nr 's/^version = "([^"]+)"/\1/p')

  echo "$(
    cat ${PKG_TEMPLATE_DIR}/${TEMPLATE} \
    | sed -r "s/{{NAME_DASHED}}/${NAME_DASHED}/g" \
    | sed -r "s/{{NAME_UNDERSCORED}}/${NAME_UNDERSCORED}/g" \
    | sed -r "s/{{VERSION}}/${VERSION}/g"
  )" > ${PACKAGE_DIR}/${TEMPLATE}

}


build_and_publish_wasm_pkg ()
{
  NAME_DASHED=$1
  NAME_UNDERSCORED=$(echo "${NAME_DASHED}" | sed -r 's/-/_/g')

  PACKAGE_DIR=.npm/${NAME_DASHED}

  rm -rf ${PACKAGE_DIR}/{src,dist}

  cargo rustc -p ${NAME_DASHED} --target wasm32-unknown-unknown --features typescript --crate-type=cdylib --release
  wasm-bindgen --target web ./target/wasm32-unknown-unknown/release/${NAME_UNDERSCORED}.wasm --out-dir ${PACKAGE_DIR}/src
  wasm-opt ${PACKAGE_DIR}/src/${NAME_UNDERSCORED}_bg.wasm -o ${PACKAGE_DIR}/src/${NAME_UNDERSCORED}_bg.wasm -Oz

  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} README.md
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} package.json
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} pnpm-lock.yaml
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} rollup.config.mjs
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} src/index.js

  # commenting out all `new URL()` and `fetch()` calls for great compatibility with JS bundlers
  sed -i.bkp -r 's;(input = new URL.+);//\1;g' ${PACKAGE_DIR}/src/${NAME_UNDERSCORED}.js
  sed -i.bkp -r 's;(input = fetch.+);//\1;g' ${PACKAGE_DIR}/src/${NAME_UNDERSCORED}.js
  rm ${PACKAGE_DIR}/src/${NAME_UNDERSCORED}.js.bkp

  pnpm -C ${PACKAGE_DIR} install
  pnpm -C ${PACKAGE_DIR} build
  pnpm -C ${PACKAGE_DIR} test

  # TODO: publish logic will go here
}


build_and_publish_wasm_pkg "fuel-asm"
build_and_publish_wasm_pkg "fuel-types"
