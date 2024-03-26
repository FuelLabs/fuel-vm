use core::str::FromStr;
use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    field::{
        Inputs,
        ReceiptsRoot,
    },
    input::coin::CoinSigned,
    ConsensusParameters,
    TransactionBuilder,
};
use fuel_types::BlockHeight;
use fuel_vm::prelude::*;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

#[cfg(feature = "alloc")]
use alloc::vec;

#[test]
fn transaction_can_be_executed_after_maturity() {
    const MATURITY: BlockHeight = BlockHeight::new(1);
    const BLOCK_HEIGHT: BlockHeight = BlockHeight::new(2);

    let arb_max_fee = 1;

    let rng = &mut StdRng::seed_from_u64(2322u64);
    let tx = TransactionBuilder::script(
        Some(op::ret(1)).into_iter().collect(),
        Default::default(),
    )
    .max_fee_limit(arb_max_fee)
    .add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.gen(),
        arb_max_fee,
        Default::default(),
        rng.gen(),
    )
    .script_gas_limit(100)
    .maturity(MATURITY)
    .finalize_checked(BLOCK_HEIGHT);

    let result = TestBuilder::new(2322u64)
        .block_height(BLOCK_HEIGHT)
        .execute_tx(tx);
    assert!(result.is_ok());
}

/// Malleable fields should not affect validity of the block
#[test]
fn malleable_fields_do_not_affect_validity() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let params = ConsensusParameters::default();

    let tx_start_ptr = params.tx_params().tx_offset();
    let tx_size_ptr = tx_start_ptr - 8;

    let tx = TransactionBuilder::script(
        vec![
            // Log tx id (hash)
            op::movi(0x21, 32),
            op::logd(0x00, 0x00, 0x00, 0x21),
            // Load tx size
            op::movi(0x21, tx_size_ptr as u32),
            op::lw(0x21, 0x21, 0),
            // Make heap space for tx bytes, chain_id and computed tx hash
            op::addi(0x22, 0x21, 8 + 32),
            op::aloc(0x22),
            // Construct the chain_id and tx bytes for hashing
            op::addi(0x22, RegId::HP, 32), // Chain id position
            op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
            op::mcpi(0x22, 0x20, 8), // Copy chain id
            op::movi(0x20, tx_start_ptr as u32),
            op::addi(0x23, 0x22, 8),   // Tx bytes position
            op::mcp(0x23, 0x20, 0x21), // Copy tx bytes
            // Assert that there is exactly one witness
            op::gtf_args(0x26, 0x00, GTFArgs::ScriptWitnessesCount),
            op::eq(0x26, 0x26, RegId::ONE),
            op::jnzf(0x26, 0x00, 1),
            op::ret(0),
            // Zero out the only witness (in the heap)
            op::gtf_args(0x26, 0x00, GTFArgs::WitnessData),
            op::gtf_args(0x27, 0x00, GTFArgs::WitnessDataLength),
            op::sub(0x27, 0x26, 0x20), // Offset in relative to the tx bytes
            op::add(0x27, 0x27, RegId::HP), // Redirect the pointer to heap
            op::addi(0x26, 0x27, 32 + 8), // Offset of tx bytes in heap
            op::gtf_args(0x27, 0x00, GTFArgs::WitnessDataLength),
            op::mcl(0x26, 0x27),
            // Zero out witness count
            op::gtf_args(0x26, 0x00, GTFArgs::Script),
            op::subi(0x26, 0x26, 8), // Offset to get the witness count address
            op::sub(0x26, 0x26, 0x20), // Offset in relative to the tx bytes
            op::add(0x26, 0x26, RegId::HP), // Redirect the pointer to heap
            op::addi(0x26, 0x26, 32 + 8), // Offset of tx bytes in heap
            op::sw(0x26, RegId::ZERO, 0), // Zero out the witness count
            // Actually hash
            op::subi(0x24, 0x21, 64 + 8 - 8), // Offset ptr
            op::s256(RegId::HP, 0x22, 0x24),  // Compute tx id hash
            op::movi(0x25, 32),               // Hash size
            op::logd(0x00, 0x00, RegId::HP, 0x25), // Log computed txid
            op::logd(0, 0, 0x22, 0x24),
            // Done
            op::ret(0x00),
        ]
        .into_iter()
        .collect(),
        params.chain_id().to_be_bytes().to_vec(),
    )
    .add_unsigned_coin_input(
        SecretKey::random(rng),
        UtxoId::new([3; 32].into(), 0),
        123456789,
        AssetId::default(),
        Default::default(),
    )
    .script_gas_limit(1_000_000)
    .finalize();

    let run_tx = |tx: Script| {
        let original_id = tx.id(&params.chain_id());

        let vm = Interpreter::<_, Script>::with_memory_storage();
        let mut client = MemoryClient::from_txtor(vm.into());
        let receipts =
            client.transact(tx.into_checked(0u32.into(), &params).expect("valid tx"));

        let start_id = receipts[0].data().unwrap();
        let computed_id = receipts[1].data().unwrap();

        assert_eq!(*original_id, start_id);
        assert_eq!(*original_id, computed_id);

        original_id
    };

    let original = run_tx(tx.clone());

    // Check that modifying the input coin doesn't affect validity
    {
        let mut tx = tx.clone();

        match tx.inputs_mut()[0] {
            Input::CoinSigned(CoinSigned {
                ref mut tx_pointer, ..
            }) => *tx_pointer = TxPointer::from_str("123456780001").unwrap(),
            _ => unreachable!(),
        };
        let result = run_tx(tx);
        assert_eq!(result, original);
    }

    // Check that modifying the receipts root doesn't affect validity
    {
        let mut tx = tx;
        *tx.receipts_root_mut() = [1u8; 32].into();
        let result = run_tx(tx);
        assert_eq!(result, original);
    }
}
