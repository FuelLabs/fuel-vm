const { expect } = require('chai')
const tx = require('.')

describe('fuel-tx [cjs]', () => {

  it('should export all types', () => {
    expect(tx.Input2).to.be.ok
    expect(tx.Output2).to.be.ok
    expect(tx.TxPointer).to.be.ok
    expect(tx.UtxoId).to.be.ok
  })

  // TODO: copy from .mjs
})
