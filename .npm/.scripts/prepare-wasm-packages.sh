#!/usr/bin/env bash

ROOT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/../.." && pwd )"
NPM_DIR=${ROOT_DIR}/.npm
PKGS_DIR=${NPM_DIR}/packages


write_template ()
{
  NAME_DASHED=$1
  NAME_UNDERSCORED=$2
  TEMPLATE=$3

  PKG_TEMPLATE_DIR=${NPM_DIR}/.scripts/template
  PKG_DIR=${PKGS_DIR}/${NAME_DASHED}

  PKG_NAME=$(echo "${NAME_DASHED}" | sed -e 's/fuel-//g')
  PKG_VERSION=$(cat ${ROOT_DIR}/Cargo.toml | sed -nr 's/^version = "([^"]+)"/\1/p')

  echo "$(
    cat ${PKG_TEMPLATE_DIR}/${TEMPLATE} \
    | sed -e "s/{{NAME_DASHED}}/${NAME_DASHED}/g" \
    | sed -e "s/{{NAME_UNDERSCORED}}/${NAME_UNDERSCORED}/g" \
    | sed -e "s/{{PKG_NAME}}/${PKG_NAME}/g" \
    | sed -e "s/{{PKG_VERSION}}/${PKG_VERSION}/g"
  )" > ${PKG_DIR}/${TEMPLATE}

}


build_wasm_npm_pkg_for ()
{
  NAME_DASHED=$1
  NAME_UNDERSCORED=$(echo "${NAME_DASHED}" | sed -e 's/-/_/g')
  MORE_ARGS=$2

  PKG_DIR=${PKGS_DIR}/${NAME_DASHED}

  rm -rf ${PKG_DIR}/{src,dist}

  cd ${ROOT_DIR}
  cargo rustc -p ${NAME_DASHED} --target wasm32-unknown-unknown --features typescript --crate-type=cdylib --profile web-release $MORE_ARGS
  wasm-bindgen --typescript --target web ./target/wasm32-unknown-unknown/web-release/${NAME_UNDERSCORED}.wasm --out-dir ${PKG_DIR}/src
  wasm-opt ${PKG_DIR}/src/${NAME_UNDERSCORED}_bg.wasm -o ${PKG_DIR}/src/${NAME_UNDERSCORED}_bg.wasm -Oz
  cd ~-

  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} README.md
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} package.json
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} rollup.config.mjs
  write_template ${NAME_DASHED} ${NAME_UNDERSCORED} src/index.js

  # commenting out all `new URL()` and `fetch()` calls for great compatibility with JS bundlers
  sed -i.bkp -r 's;(.+= new URL.+);//\1;g' ${PKG_DIR}/src/${NAME_UNDERSCORED}.js
  sed -i.bkp -r 's;(.+= fetch.+);//\1;g' ${PKG_DIR}/src/${NAME_UNDERSCORED}.js

  rm ${PKG_DIR}/src/${NAME_UNDERSCORED}.js.bkp
}


build_wasm_npm_pkg_for "fuel-asm"
build_wasm_npm_pkg_for "fuel-types"
build_wasm_npm_pkg_for "fuel-tx" --no-default-features
