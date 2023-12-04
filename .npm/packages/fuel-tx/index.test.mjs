import { expect } from 'chai'
import * as tx from './dist/web/index.mjs'

describe('fuel-tx [mjs]', () => {

  it('should export all types', () => {
    expect(tx.UtxoId).to.be.ok
    expect(tx.TxPointer).to.be.ok
    expect(tx.PredicateParameters).to.be.ok
    expect(tx.Input2).to.be.ok
    expect(tx.Output2).to.be.ok
  })

  it('should serialize and deserialize UtxoId correctly', () => {
    let utxo_id = new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b1a");
    let bytes = utxo_id.to_bytes();
    let utxo_id2 = tx.UtxoId.from_bytes(bytes);
    expect(utxo_id.to_string()).to.equal(utxo_id2.to_string())
  })

  it('should serialize and deserialize TxPointer correctly', () => {
    let utxo_id = new tx.TxPointer("0123456789ab");
    let bytes = utxo_id.to_bytes();
    let utxo_id2 = tx.TxPointer.from_bytes(bytes);
    expect(utxo_id.to_string()).to.equal(utxo_id2.to_string())
  })


  it('should serialize and deserialize all input variants correctly', () => {
    [
      tx.Input2.coin_predicate(
        new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b1a"),
        tx.Address.zeroed(),
        BigInt(1234),
        tx.AssetId.zeroed(),
        new tx.TxPointer("0123456789ab"),
        new tx.BlockHeight(5678),
        BigInt(9012),
        [1, 2, 3, 4],
        [5, 6, 7, 8],
      ),
      tx.Input2.coin_signed(
        new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b1a"),
        tx.Address.zeroed(),
        BigInt(1234),
        tx.AssetId.zeroed(),
        new tx.TxPointer("0123456789ab"),
        2,
        new tx.BlockHeight(5678),
      ),
      tx.Input2.contract(
        new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b1a"),
        tx.Bytes32.zeroed(),
        tx.Bytes32.zeroed(),
        new tx.TxPointer("0123456789ab"),
        tx.ContractId.zeroed(),
      ),
      tx.Input2.message_coin_signed(
        tx.Address.zeroed(),
        tx.Address.zeroed(),
        BigInt(1234),
        tx.Nonce.zeroed(),
        2,
      ),
      tx.Input2.message_coin_predicate(
        tx.Address.zeroed(),
        tx.Address.zeroed(),
        BigInt(1234),
        tx.Nonce.zeroed(),
        BigInt(1234),
        [1, 2, 3, 4],
        [5, 6, 7, 8],
      ),
      tx.Input2.message_data_signed(
        tx.Address.zeroed(),
        tx.Address.zeroed(),
        BigInt(1234),
        tx.Nonce.zeroed(),
        2,
        [1, 2, 3, 4],
      ),
      tx.Input2.message_data_predicate(
        tx.Address.zeroed(),
        tx.Address.zeroed(),
        BigInt(1234),
        tx.Nonce.zeroed(),
        BigInt(1234),
        [0, 1, 2, 3],
        [1, 2, 3, 4],
        [5, 6, 7, 8],
      ),
    ].forEach(input => {
      let bytes = input.to_bytes();
      let input2 = tx.Input2.from_bytes(bytes);
      expect(input.toString()).to.equal(input2.toString())
    })
  })


  it('should serialize and deserialize all output variants correctly', () => {
    [
      tx.Output2.coin(
        tx.Address.zeroed(),
        BigInt(1234),
        tx.AssetId.zeroed(),
      ),
      tx.Output2.contract(
        2,
        tx.Bytes32.zeroed(),
        tx.Bytes32.zeroed(),
      ),
      tx.Output2.change(
        tx.Address.zeroed(),
        BigInt(1234),
        tx.AssetId.zeroed(),
      ),
      tx.Output2.variable(
        tx.Address.zeroed(),
        BigInt(1234),
        tx.AssetId.zeroed(),
      ),
      tx.Output2.contract_created(
        tx.ContractId.zeroed(),
        tx.Bytes32.zeroed(),
      ),
    ].forEach(output => {
      let bytes = output.to_bytes();
      let output2 = tx.Output2.from_bytes(bytes);
      expect(output.toString()).to.equal(output2.toString())
    })
  })


  const hexToByte = (hex) => {
    const key = '0123456789abcdef'
    let newBytes = []
    let currentChar = 0
    let currentByte = 0
    for (let i = 0; i < hex.length; i++) {   // Go over two 4-bit hex chars to convert into one 8-bit byte
      currentChar = key.indexOf(hex[i])
      if (i % 2 === 0) { // First hex char
        currentByte = (currentChar << 4) // Get 4-bits from first hex char
      }
      if (i % 2 === 1) { // Second hex char
        currentByte += (currentChar)     // Concat 4-bits from second hex char
        newBytes.push(currentByte)       // Add byte
      }
    }
    return new Uint8Array(newBytes)
  }

  it('should validate input correctly', () => {
    let input = tx.Input2.coin_signed(
      new tx.UtxoId("0xc49d65de61cf04588a764b557d25cc6c6b4bc0d7429227e2a21e61c213b3a3e2:18"),
      tx.Address.from_bytes(hexToByte("f1e92c42b90934aa6372e30bc568a326f6e66a1a0288595e6e3fbd392a4f3e6e")),
      10599410012256088338n,
      tx.AssetId.from_bytes(hexToByte("2cafad611543e0265d89f1c2b60d9ebf5d56ad7e23d9827d6b522fd4d6e44bc3")),
      new tx.TxPointer("000000000000"),
      0,
      new tx.BlockHeight(0),
    );

    tx.check_input(input, 0, tx.Bytes32.from_bytes(hexToByte("108eae4147d2c1c86ef4c2ab7c9fe94126645c8d8737495a0574ef1518ae74d8")), [], [{ data: hexToByte("7ce4de2225f041b7f9fec727343a501d99e5b7b58d33f3d4a2cf218d3489959bdec24d13770b5d3bb084b4dac3474f95153e6ecc98f6f0f8ca37a2897b9562ee") }], new tx.PredicateParameters());
  })
})
