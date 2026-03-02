export * from './{{NAME_UNDERSCORED}}.js';

export type InitInput =
  | WebAssembly.Module
  | BufferSource
  | Response
  | Promise<Response>;

export function initWasm(module_or_path?: InitInput): Promise<void>;
