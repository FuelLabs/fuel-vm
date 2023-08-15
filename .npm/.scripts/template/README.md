
# @fuels/vm-{{NAME_DASHED}}

WASM version of `{{NAME_DASHED}}` Rust crate:
 - https://crates.io/crates/{{NAME_DASHED}}
 - https://github.com/FuelLabs/fuel-vm/tree/master/{{NAME_DASHED}}


# Getting Started

Be sure to `await` the WASM async initialization:

```ts
import * as {{NAME_UNDERSCORED}} from '@fuels/vm-{{PKG_NAME}}'

(async function() {
  await {{NAME_UNDERSCORED}}.initWasm();

  // {{NAME_UNDERSCORED}}.<?>();
  // ...
})();

```
