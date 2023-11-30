const { expect } = require('chai')
const types = require('.')

describe('fuel-types [cjs]', () => {

  it('should export all types', () => {

    expect(types.Input2).to.be.ok
    expect(types.Output2).to.be.ok
    expect(types.TxPointer).to.be.ok
    expect(types.UtxoId).to.be.ok
  })

})
