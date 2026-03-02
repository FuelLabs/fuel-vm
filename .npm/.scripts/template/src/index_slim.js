import init from './{{NAME_UNDERSCORED}}.js'

let _initPromise;

export async function initWasm (module_or_path) {
  if (!_initPromise) {
    if (module_or_path == null) {
      throw new Error(
        '@fuels/vm-{{PKG_NAME}}/slim: initWasm() requires a WebAssembly.Module or BufferSource. ' +
        'Use "@fuels/vm-{{PKG_NAME}}" for automatic WASM loading.'
      );
    }
    _initPromise = init({ module_or_path });
  }
  return _initPromise;
}

export * from './{{NAME_UNDERSCORED}}.js'
