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
    expect(utxo_id.to_string()).to.be.equal(utxo_id2.to_string())
  })


  // it('should serialize and deserialize all types correctly', () => {
  //   tx.input_coin_predicate(
  //     utxo_id: UtxoId,
  //     owner: Address,
  //     amount: Word,
  //     asset_id: AssetId,
  //     tx_pointer: TxPointer,
  //     maturity: BlockHeight,
  //     predicate_gas_used: Word,
  //     predicate: Vec < u8 >,
  //     predicate_data: Vec < u8 >,
  //   )
  //   tx.input_from_bytes()
  // })

})
