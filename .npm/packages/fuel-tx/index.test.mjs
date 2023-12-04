import { expect } from 'chai'
import * as tx from './dist/web/index.mjs'

describe('fuel-tx [mjs]', () => {

  it('should export all types', () => {
    expect(tx.Input2).to.be.ok
    expect(tx.Output2).to.be.ok
    expect(tx.TxPointer).to.be.ok
    expect(tx.UtxoId).to.be.ok
  })

  it('should serialize and deserialize UtxoId correctly', () => {
    let utxo_id = new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b1a");
    let bytes = utxo_id.to_bytes();
    let utxo_id2 = tx.UtxoId.from_bytes(bytes);
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

})
