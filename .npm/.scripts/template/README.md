
# @fuels/vm-{{NAME_DASHED}}

WASM version of `{{NAME_DASHED}}` Rust crate:
 - https://crates.io/crates/{{NAME_DASHED}}
 - https://github.com/FuelLabs/fuel-vm/tree/master/{{NAME_DASHED}}


## Getting Started

### Standard Usage (Browser / Node.js)

The default entrypoint includes WASM inlined as base64:

```ts
import * as {{NAME_UNDERSCORED}} from '@fuels/vm-{{PKG_NAME}}'

await {{NAME_UNDERSCORED}}.initWasm();

// {{NAME_UNDERSCORED}}.<?>();
// ...
```

### Slim Usage (Cloudflare Workers / Custom WASM Loading)

The `/slim` entrypoint omits the inlined WASM, requiring you to supply it.
This is necessary for environments like Cloudflare Workers where runtime WASM
compilation is disallowed.

#### Cloudflare Workers

```ts
import * as {{NAME_UNDERSCORED}} from '@fuels/vm-{{PKG_NAME}}/slim'
import wasm from '@fuels/vm-{{PKG_NAME}}/wasm'

await {{NAME_UNDERSCORED}}.initWasm(wasm);

// {{NAME_UNDERSCORED}}.<?>();
// ...
```
