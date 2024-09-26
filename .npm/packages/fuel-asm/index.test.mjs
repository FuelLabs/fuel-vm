import { expect } from 'chai'
import { join } from 'path'
import { readFileSync } from 'fs'
import * as asm from './dist/web/index.mjs'

/*
Top-level usage:

  asm.initWasm().then(() => {
    const gtf = asm.gtf(0x10, 0x00, asm.GTFArgs.ScriptData)
    // ...
  });

*/

describe('fuel-asm [esm]', () => {

  it('should ensure URL/fetch patching was succesful', async () => {
    const dist = join(import.meta.dirname, 'dist');
    const cjsContents = readFileSync(join(dist, 'node/index.cjs'), 'utf-8')
    const mjsContents = readFileSync(join(dist, 'web/index.mjs'), 'utf-8')

    const reg = /(new URL|fetch)\(.+\)/
    expect(mjsContents).to.not.match(reg);
    expect(cjsContents).to.not.match(reg);
  })

  it('should compose simple script', async () => {

    await asm.initWasm();

    const gtf = asm.gtf(0x10, 0x00, asm.GTFArgs.ScriptData)
    const addi = asm.addi(0x11, 0x10, 0x20)
    const lw = asm.lw(0x12, 0x11, 0x0)
    const addi2 = asm.addi(0x13, 0x11, 0x8)
    const tr = asm.tr(0x10, 0x12, 0x13)
    const ret = asm.ret(0x1)

    const script = Uint8Array.from([
      ...gtf.to_bytes(),
      ...addi.to_bytes(),
      ...lw.to_bytes(),
      ...addi2.to_bytes(),
      ...tr.to_bytes(),
      ...ret.to_bytes(),
    ])

    const expected = new Uint8Array([
      97, 64,  0,  10, 80, 69,  0, 32,
      93, 73, 16,   0, 80, 77, 16,  8,
      60, 65, 36, 192, 36,  4,  0,  0
    ])

    expect(script).to.deep.equal(expected)
  })

})
