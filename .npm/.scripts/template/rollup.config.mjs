import { wasm } from '@rollup/plugin-wasm'
import dts from 'rollup-plugin-dts'
import { copyFileSync } from 'node:fs'

const sync = ['src/{{NAME_UNDERSCORED}}_bg.wasm'];

const copyWasm = {
  name: 'copy-wasm',
  writeBundle() {
    copyFileSync('src/{{NAME_UNDERSCORED}}_bg.wasm', 'dist/{{NAME_UNDERSCORED}}_bg.wasm');
  }
};

export default [
  {
    plugins: [wasm({ sync, targetEnv: 'auto' })],
    input: 'src/index.js',
    output: [
      { file: 'dist/node/index.cjs', format: 'cjs' },
    ]
  }, {
    plugins: [wasm({ sync, targetEnv: 'auto' })],
    input: 'src/index.js',
    output: [
      { file: 'dist/web/index.mjs', format: 'es' },
    ]
  }, {
    input: 'src/index_slim.js',
    output: [
      { file: 'dist/slim/index.cjs', format: 'cjs' },
    ]
  }, {
    plugins: [copyWasm],
    input: 'src/index_slim.js',
    output: [
      { file: 'dist/slim/index.mjs', format: 'es' },
    ]
  }, {
    plugins: [dts()],
    input: 'src/index.d.ts',
    output: [
      { file: 'dist/web/index.d.ts', format: 'es' },
      { file: 'dist/node/index.d.ts', format: 'es' },
    ],
  }, {
    plugins: [dts()],
    input: 'src/index_slim.d.ts',
    output: [
      { file: 'dist/slim/index.d.ts', format: 'es' },
    ],
  }
]
