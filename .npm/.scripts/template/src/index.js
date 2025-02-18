import init from './{{NAME_UNDERSCORED}}.js'
import wasm from './{{NAME_UNDERSCORED}}_bg.wasm'

export async function initWasm () {
  return await init({ module_or_path: wasm() });
}

/**
 * calling it right away for pre-caching
 * the wasm async initialization at startup
 */
initWasm();

export * from './{{NAME_UNDERSCORED}}.js'
