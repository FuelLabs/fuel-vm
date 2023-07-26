import { wasm } from '@rollup/plugin-wasm'
import dts from 'rollup-plugin-dts'

const sync = ['src/{{NAME_UNDERSCORED}}_bg.wasm'];

export default [
  {
    plugins: [wasm({ sync, targetEnv: 'auto-inline' })],
    input: 'src/index.js',
    output: [
      { file: 'dist/node/index.cjs', format: 'cjs' },
    ]
  }, {
    plugins: [wasm({ sync, targetEnv: 'auto-inline' })],
    input: 'src/index.js',
    output: [
      { file: 'dist/web/index.mjs', format: 'es' },
    ]
  }, {
    plugins: [dts()],
    input: 'src/{{NAME_UNDERSCORED}}.d.ts',
    output: [
      { file: 'dist/web/index.d.ts', format: 'es' },
      { file: 'dist/node/index.d.ts', format: 'es' }
    ],
  }
]
