const { expect } = require('chai')
const asm = require('.')

describe('fuel-asm [cjs]', () => {

  it('should compose simple script', () => {

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
      97, 64,  0,  12, 80, 69,  0, 32,
      93, 73, 16,   0, 80, 77, 16,  8,
      60, 65, 36, 192, 36,  4,  0,  0
    ])

    expect(script).to.deep.equal(expected)
  })

})
