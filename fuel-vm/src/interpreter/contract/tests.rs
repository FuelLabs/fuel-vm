use crate::{
    interpreter::memory::Memory,
    storage::MemoryStorage,
};

use super::*;
use fuel_tx::{
    field::{
        Inputs,
        Outputs,
    },
    Input,
    Script,
};
use test_case::test_case;

#[test_case(0, 32 => Ok(()); "Can read contract balance")]
fn test_contract_balance(b: Word, c: Word) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[b as usize..(b as usize + AssetId::LEN)]
        .copy_from_slice(&[2u8; AssetId::LEN][..]);
    memory[c as usize..(c as usize + ContractId::LEN)]
        .copy_from_slice(&[3u8; ContractId::LEN][..]);
    let contract_id = ContractId::from([3u8; 32]);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    storage
        .merkle_contract_asset_id_balance_insert(
            &contract_id,
            &AssetId::from([2u8; 32]),
            33,
        )
        .unwrap();
    let mut pc = 4;

    let mut panic_context = PanicContext::None;
    let input = ContractBalanceCtx {
        storage: &mut storage,
        memory: &mut memory,
        pc: RegMut::new(&mut pc),
        input_contracts: InputContracts::new(
            [&contract_id].into_iter(),
            &mut panic_context,
        ),
    };
    let mut result = 0;

    input.contract_balance(&mut result, b, c)?;

    assert_eq!(pc, 8);
    assert_eq!(result, 33);

    Ok(())
}

#[test_case(true, 0, 50, 32 => Ok(()); "Can transfer from external balance")]
fn test_transfer(
    external: bool,
    contract_id_offset: Word,
    transfer_amount: Word,
    asset_id_offset: Word,
) -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[contract_id_offset as usize..(contract_id_offset as usize + ContractId::LEN)]
        .copy_from_slice(&[3u8; ContractId::LEN][..]);
    memory[asset_id_offset as usize..(asset_id_offset as usize + AssetId::LEN)]
        .copy_from_slice(&[2u8; AssetId::LEN][..]);
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    storage
        .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, 60)
        .unwrap();
    let mut pc = 4;

    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };

    let mut balances =
        RuntimeBalances::try_from_iter([(AssetId::from([2u8; 32]), 50)]).unwrap();
    let mut receipts = Default::default();
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

    let input = TransferCtx {
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

    input.transfer(
        &mut panic_context,
        contract_id_offset,
        transfer_amount,
        asset_id_offset,
    )?;

    assert_eq!(pc, 8);
    let amount = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();
    assert_eq!(balances.balance(&asset_id).unwrap(), 0);
    assert_eq!(amount, 60 + transfer_amount);

    Ok(())
}

#[test_case(true, 0, 0, 50, 32 => Ok(()); "Can transfer from external balance")]
#[test_case(false, 0, 0, 50, 32 => Ok(()); "Can transfer from internal balance")]
fn test_transfer_output(
    external: bool,
    recipient_offset: Word,
    output_index: Word,
    transfer_amount: Word,
    asset_id_offset: Word,
) -> Result<(), RuntimeError> {
    // Given

    const ASSET_ID: [u8; AssetId::LEN] = [2u8; AssetId::LEN];
    const CONTRACT_ID: [u8; ContractId::LEN] = [3u8; ContractId::LEN];
    const RECIPIENT_ADDRESS: [u8; Address::LEN] = [4u8; Address::LEN];

    let mut pc = 4;

    // Arbitrary value
    let fp = 2048;
    let is = 0;

    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();

    memory[recipient_offset as usize..(recipient_offset as usize + Address::LEN)]
        .copy_from_slice(&RECIPIENT_ADDRESS[..]);
    memory[asset_id_offset as usize..(asset_id_offset as usize + AssetId::LEN)]
        .copy_from_slice(&ASSET_ID[..]);
    memory[fp as usize..(fp as usize + ContractId::LEN)]
        .copy_from_slice(&CONTRACT_ID[..]);

    let contract_id = ContractId::from(CONTRACT_ID);
    let asset_id = AssetId::from(ASSET_ID);
    let recipient = Address::from(RECIPIENT_ADDRESS);

    dbg!(contract_id, asset_id);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());

    let initial_contract_balance = 60;

    storage
        .merkle_contract_asset_id_balance_insert(
            &contract_id,
            &asset_id,
            initial_contract_balance,
        )
        .unwrap();

    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };

    let balance_of_start = transfer_amount;

    let mut balances =
        RuntimeBalances::try_from_iter([(asset_id, balance_of_start)]).unwrap();
    let mut receipts = Default::default();
    let mut tx = Script::default();
    *tx.inputs_mut() = vec![Input::contract(
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        contract_id,
    )];

    dbg!(&recipient);
    *tx.outputs_mut() = vec![Output::variable(
        recipient,
        Default::default(),
        Default::default(),
    )];

    let old_memory = memory.clone();

    let transfer_ctx = TransferCtx {
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

    // When

    transfer_ctx.transfer_output(
        recipient_offset,
        output_index,
        transfer_amount,
        asset_id_offset,
    )?;

    let mem_len = old_memory.len();

    for i in 0..mem_len {
        if old_memory[i] != memory[i] {
            println!("{}: {} -> {}", i, old_memory[i], memory[i]);
        }
    }

    // Then

    let final_contract_balance = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();

    dbg!(&final_contract_balance);

    assert_eq!(pc, 8);
    if external {
        // In an external context, decrease MEM[balanceOfStart(MEM[$rD, 32]), 8] by $rC.
        assert_eq!(
            balances.balance(&asset_id).unwrap(),
            balance_of_start - transfer_amount
        );
        assert_eq!(final_contract_balance, initial_contract_balance);
    } else {
        assert_eq!(balances.balance(&asset_id).unwrap(), balance_of_start);
        assert_eq!(
            final_contract_balance,
            initial_contract_balance - transfer_amount
        );
    }

    Ok(())
}

#[test_case(0, 0 => Ok(()); "Can increase balance by zero")]
#[test_case(None, 0 => Ok(()); "Can initialize balance to zero")]
#[test_case(None, Word::MAX => Ok(()); "Can initialize balance to max")]
#[test_case(0, Word::MAX => Ok(()); "Can add max to zero")]
#[test_case(Word::MAX, 0 => Ok(()); "Can add zero to max")]
#[test_case(1, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::ArithmeticOverflow)); "Overflowing add")]
#[test_case(Word::MAX, 1 => Err(RuntimeError::Recoverable(PanicReason::ArithmeticOverflow)); "Overflowing 1 add")]
fn test_balance_increase(
    initial: impl Into<Option<Word>>,
    amount: Word,
) -> Result<(), RuntimeError> {
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let initial = initial.into();
    if let Some(initial) = initial {
        storage
            .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, initial)
            .unwrap();
    }

    let result = balance_increase(&mut storage, &contract_id, &asset_id, amount)?;
    let initial = initial.unwrap_or(0);

    assert_eq!(result, initial + amount);

    let result = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();

    assert_eq!(result, initial + amount);

    Ok(())
}

#[test_case(0, 0 => Ok(()); "Can increase balance by zero")]
#[test_case(None, 0 => Ok(()); "Can initialize balance to zero")]
#[test_case(Word::MAX, 0 => Ok(()); "Can initialize balance to max")]
#[test_case(10, 10 => Ok(()); "Can subtract to zero")]
#[test_case(1, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Overflowing subtract")]
#[test_case(1, 2 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Overflowing 1 subtract")]
fn test_balance_decrease(
    initial: impl Into<Option<Word>>,
    amount: Word,
) -> Result<(), RuntimeError> {
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::new(Default::default(), Default::default());
    let initial = initial.into();
    if let Some(initial) = initial {
        storage
            .merkle_contract_asset_id_balance_insert(&contract_id, &asset_id, initial)
            .unwrap();
    }

    let result = balance_decrease(&mut storage, &contract_id, &asset_id, amount)?;
    let initial = initial.unwrap_or(0);

    assert_eq!(result, initial - amount);

    let result = storage
        .merkle_contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();

    assert_eq!(result, initial - amount);

    Ok(())
}
