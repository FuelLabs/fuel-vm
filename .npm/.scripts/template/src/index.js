import init from './{{NAME_UNDERSCORED}}.js'
import wasmModule from './{{NAME_UNDERSCORED}}_bg.wasm'

let _initPromise;

export async function initWasm (module_or_path) {
  if (!_initPromise) {
    _initPromise = init({ module_or_path: module_or_path ?? wasmModule() });
  }
  return _initPromise;
}

initWasm();

export * from './{{NAME_UNDERSCORED}}.js'
