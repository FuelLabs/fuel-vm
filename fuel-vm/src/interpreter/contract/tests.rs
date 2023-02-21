use crate::interpreter::InitialBalances;
use crate::storage::MemoryStorage;

use super::*;
use fuel_tx::field::Inputs;
use fuel_tx::Input;
use fuel_tx::Script;
use test_case::test_case;

#[test_case(0, 32 => Ok(()); "Can read contract balance")]
fn test_contract_balance(b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Box<[u8; VM_MEMORY_SIZE]> = vec![1u8; VM_MEMORY_SIZE].try_into().unwrap();
    memory[b as usize..(b as usize + AssetId::LEN)].copy_from_slice(&[2u8; AssetId::LEN][..]);
    memory[c as usize..(c as usize + ContractId::LEN)].copy_from_slice(&[3u8; ContractId::LEN][..]);
    let contract_id = ContractId::from([3u8; 32]);
    let mut storage = MemoryStorage::new(0, Default::default());
    storage
        .merkle_contract_asset_id_balance_insert(&contract_id, &AssetId::from([2u8; 32]), 33)
        .unwrap();
    let mut pc = 4;

    let input = ContractBalanceInput {
        storage: &mut storage,
        memory: &mut memory,
        pc: RegMut::new(&mut pc),
        input_contracts: [&contract_id].into_iter(),
        panic_context: &mut PanicContext::None,
    };
    let mut result = 0;

    input.contract_balance(&mut result, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 33);

    Ok(())
}

#[test_case(true, 0, 50, 32 => Ok(()); "Can transfer from external balance")]
fn test_transfer(external: bool, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Box<[u8; VM_MEMORY_SIZE]> = vec![1u8; VM_MEMORY_SIZE].try_into().unwrap();
    memory[a as usize..(a as usize + ContractId::LEN)].copy_from_slice(&[3u8; ContractId::LEN][..]);
    memory[c as usize..(c as usize + AssetId::LEN)].copy_from_slice(&[2u8; AssetId::LEN][..]);
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::new(0, Default::default());
    storage
        .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, 60)
        .unwrap();
    let mut pc = 4;

    let context = if external {
        Context::Script { block_height: 0 }
    } else {
        Context::Call { block_height: 0 }
    };

    let mut balances = RuntimeBalances::from(InitialBalances::try_from([(AssetId::from([2u8; 32]), 50)]).unwrap());
    let mut receipts = Vec::new();
    let mut panic_context = PanicContext::None;
    let mut tx = Script::default();
    *tx.inputs_mut() = vec![Input::contract(
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        contract_id,
    )];

    let fp = 0;
    let is = 0;

    let input = TransferInput {
        storage: &mut storage,
        memory: &mut memory,
        pc: RegMut::new(&mut pc),
        context: &context,
        balances: &mut balances,
        receipts: &mut receipts,
        tx: &mut tx,
        tx_offset: 0,
        fp: Reg::new(&fp),
        is: Reg::new(&is),
    };

    input.transfer(&mut panic_context, a, b, c)?;

    assert_eq!(pc, 8);
    let amount = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();
    assert_eq!(balances.balance(&asset_id).unwrap(), 0);
    assert_eq!(amount, b);

    Ok(())
}
