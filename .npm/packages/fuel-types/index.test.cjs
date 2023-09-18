const { expect } = require('chai')
const types = require('.')

describe('fuel-types [cjs]', () => {

  it('should export all types', () => {

    expect(types.Address).to.be.ok
    expect(types.AssetId).to.be.ok
    expect(types.BlockHeight).to.be.ok
    expect(types.Bytes20).to.be.ok
    expect(types.Bytes32).to.be.ok
    expect(types.Bytes4).to.be.ok
    expect(types.Bytes64).to.be.ok
    expect(types.Bytes8).to.be.ok
    expect(types.ChainId).to.be.ok
    expect(types.ContractId).to.be.ok
    expect(types.MessageId).to.be.ok
    expect(types.Nonce).to.be.ok
    expect(types.Salt).to.be.ok

  })

})
