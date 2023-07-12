import { wasm } from '@rollup/plugin-wasm'
import dts from 'rollup-plugin-dts'

export default [
  {
    plugins: [wasm({ targetEnv: 'auto-inline' })],
    input: 'src/index.js',
    output: [
      { file: 'dist/node/index.cjs', format: 'cjs' },
    ]
  }, {
    plugins: [wasm({ targetEnv: 'auto-inline' })],
    input: 'src/index.js',
    output: [
      { file: 'dist/web/index.mjs', format: 'es' },
    ]
  }, {
    plugins: [dts()],
    input: 'src/fuel_asm.d.ts',
    output: [
      { file: 'dist/web/index.d.ts', format: 'es' },
      { file: 'dist/node/index.d.ts', format: 'es' }
    ],
  }
]
