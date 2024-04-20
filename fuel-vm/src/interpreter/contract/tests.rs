#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use core::convert::Infallible;

use alloc::vec;

use crate::{
    interpreter::memory::Memory,
    storage::MemoryStorage,
};

use super::*;
use crate::interpreter::internal::absolute_output_mem_range;
use fuel_tx::{
    field::{
        Inputs,
        Outputs,
    },
    Input,
    Script,
};
use fuel_types::canonical::Deserialize;
use test_case::test_case;

#[test_case(0, 32 => Ok(()); "Can read contract balance")]
fn test_contract_balance(b: Word, c: Word) -> IoResult<(), Infallible> {
    let mut memory: Memory = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[b as usize..(b as usize + AssetId::LEN)]
        .copy_from_slice(&[2u8; AssetId::LEN][..]);
    memory[c as usize..(c as usize + ContractId::LEN)]
        .copy_from_slice(&[3u8; ContractId::LEN][..]);
    let contract_id = ContractId::from([3u8; 32]);
    let mut storage = MemoryStorage::default();
    let old_balance = storage
        .contract_asset_id_balance_insert(&contract_id, &AssetId::from([2u8; 32]), 33)
        .unwrap();
    assert!(old_balance.is_none());
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

#[test_case(true, 0, 0, 50, 32, 32 => Ok(()); "Can transfer from external balance")]
#[test_case(false, 0, 0, 50, 32, 32 => Ok(()); "Can transfer from internal balance")]
#[test_case(true, 32, 32, 50, 0, 0 => Ok(()); "Can transfer from external balance with flipped offsets")]
#[test_case(false, 32, 32, 50, 0, 0 => Ok(()); "Can transfer from internal balance with flipped offsets")]
#[test_case(true, 0, 0, 70, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Cannot transfer from external balance insufficient funds")]
#[test_case(false, 0, 0, 70, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Cannot transfer from internal balance insufficient funds")]
#[test_case(true, MEM_SIZE as u64 - 1, 0, 50, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "External transfer errors if contract_id lookup overflows")]
#[test_case(false, MEM_SIZE as u64 - 1, 0, 50, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Internal transfer errors if contract_id lookup overflows")]
#[test_case(true, 0, 0, 50, MEM_SIZE as u64 - 1, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "External transfer errors if asset_id lookup overflows")]
#[test_case(false, 0, 0, 50, MEM_SIZE as u64 - 1, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Internal transfer errors if asset_id lookup overflows")]
fn test_transfer(
    external: bool,
    requested_contract_id_offset: Word,
    real_contract_id_offset: Word,
    transfer_amount: Word,
    requested_asset_id_offset: Word,
    real_asset_id_offset: Word,
) -> IoResult<(), Infallible> {
    // Given

    const ASSET_ID: AssetId = AssetId::new([2u8; AssetId::LEN]);
    const RECIPIENT_CONTRACT_ID: ContractId = ContractId::new([3u8; ContractId::LEN]);
    const SOURCE_CONTRACT_ID: ContractId = ContractId::new([5u8; ContractId::LEN]);

    let mut pc = 4;

    // Arbitrary value
    let fp = 2048;
    let is = 0;
    let mut cgas = 10_000;
    let mut ggas = 10_000;

    let mut memory: Memory = vec![1u8; MEM_SIZE].try_into().unwrap();
    memory[real_contract_id_offset as usize
        ..(real_contract_id_offset as usize + ContractId::LEN)]
        .copy_from_slice(RECIPIENT_CONTRACT_ID.as_ref());
    memory[real_asset_id_offset as usize..(real_asset_id_offset as usize + AssetId::LEN)]
        .copy_from_slice(ASSET_ID.as_ref());
    memory[fp as usize..(fp as usize + ContractId::LEN)]
        .copy_from_slice(SOURCE_CONTRACT_ID.as_ref());

    let mut storage = MemoryStorage::default();

    let initial_recipient_contract_balance = 0;
    let initial_source_contract_balance = 60;
    let old_balance = storage
        .contract_asset_id_balance_insert(
            &SOURCE_CONTRACT_ID,
            &ASSET_ID,
            initial_source_contract_balance,
        )
        .unwrap();
    assert!(old_balance.is_none());

    let context = if external {
        Context::Script {
            block_height: Default::default(),
        }
    } else {
        Context::Call {
            block_height: Default::default(),
        }
    };

    let mut balances = RuntimeBalances::try_from_iter([(ASSET_ID, 50)]).unwrap();
    let start_balance = balances.balance(&ASSET_ID).unwrap();

    let mut receipts = Default::default();
    let mut panic_context = PanicContext::None;
    let mut tx = Script::default();
    *tx.inputs_mut() = vec![Input::contract(
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        RECIPIENT_CONTRACT_ID,
    )];

    let transfer_ctx = TransferCtx {
        storage: &mut storage,
        memory: &mut memory,
        pc: RegMut::new(&mut pc),
        context: &context,
        balances: &mut balances,
        receipts: &mut receipts,
        profiler: &mut Default::default(),
        new_storage_gas_per_byte: 1,
        tx: &mut tx,
        tx_offset: 0,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        fp: Reg::new(&fp),
        is: Reg::new(&is),
    };

    // When

    transfer_ctx.transfer(
        &mut panic_context,
        requested_contract_id_offset,
        transfer_amount,
        requested_asset_id_offset,
    )?;

    // Then

    let final_recipient_contract_balance = storage
        .contract_asset_id_balance(&RECIPIENT_CONTRACT_ID, &ASSET_ID)
        .unwrap()
        .unwrap();

    let final_source_contract_balance = storage
        .contract_asset_id_balance(&SOURCE_CONTRACT_ID, &ASSET_ID)
        .unwrap()
        .unwrap();

    assert_eq!(pc, 8);
    assert_eq!(
        final_recipient_contract_balance,
        initial_recipient_contract_balance + transfer_amount
    );
    if external {
        assert_eq!(
            balances.balance(&ASSET_ID).unwrap(),
            start_balance - transfer_amount
        );
        assert_eq!(
            final_source_contract_balance,
            initial_source_contract_balance
        );
    } else {
        assert_eq!(balances.balance(&ASSET_ID).unwrap(), start_balance);
        assert_eq!(
            final_source_contract_balance,
            initial_source_contract_balance - transfer_amount
        );
    }

    Ok(())
}

#[test_case(true, 0, 0, 0, 50, 32, 32 => Ok(()); "Can transfer from external balance")]
#[test_case(false, 0, 0, 0, 50, 32, 32 => Ok(()); "Can transfer from internal balance")]
#[test_case(true, 32, 32, 0, 50, 0, 0 => Ok(()); "Can transfer from external balance with flipped offsets")]
#[test_case(false,32, 32, 0, 50, 0, 0 => Ok(()); "Can transfer from internal balance with flipped offsets")]
#[test_case(false, 0, 0, 0, 70, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Cannot transfer from external balance insufficient funds")]
#[test_case(false, 0, 0, 0, 70, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance)); "Cannot transfer from internal balance insufficient funds")]
#[test_case(true, MEM_SIZE as u64 - 1, 0, 0, 50, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "External transfer errors if contract_id lookup overflows")]
#[test_case(false, MEM_SIZE as u64 - 1, 0, 0, 50, 32, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Internal transfer errors if contract_id lookup overflows")]
#[test_case(true, 0, 0, 0, 50, MEM_SIZE as u64 - 1, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "External transfer errors if asset_id lookup overflows")]
#[test_case(false, 0, 0, 0, 50, MEM_SIZE as u64 - 1, 32 => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow)); "Internal transfer errors if asset_id lookup overflows")]
fn test_transfer_output(
    external: bool,
    requested_recipient_offset: Word,
    real_recipient_offset: Word,
    output_index: Word,
    transfer_amount: Word,
    requested_asset_id_offset: Word,
    real_asset_id_offset: Word,
) -> IoResult<(), Infallible> {
    // Given

    const ASSET_ID: AssetId = AssetId::new([2u8; AssetId::LEN]);
    const SOURCE_CONTRACT_ID: ContractId = ContractId::new([3u8; ContractId::LEN]);
    const RECIPIENT_ADDRESS: Address = Address::new([4u8; Address::LEN]);

    let mut pc = 4;

    // Arbitrary value
    let fp = 2048;
    let is = 0;
    let mut cgas = 10_000;
    let mut ggas = 10_000;

    let mut memory: Memory = vec![1u8; MEM_SIZE].try_into().unwrap();

    memory
        [real_recipient_offset as usize..(real_recipient_offset as usize + Address::LEN)]
        .copy_from_slice(RECIPIENT_ADDRESS.as_ref());
    memory[real_asset_id_offset as usize..(real_asset_id_offset as usize + AssetId::LEN)]
        .copy_from_slice(ASSET_ID.as_ref());
    memory[fp as usize..(fp as usize + ContractId::LEN)]
        .copy_from_slice(SOURCE_CONTRACT_ID.as_ref());

    let mut storage = MemoryStorage::default();

    let initial_contract_balance = 60;

    let old_balance = storage
        .contract_asset_id_balance_insert(
            &SOURCE_CONTRACT_ID,
            &ASSET_ID,
            initial_contract_balance,
        )
        .unwrap();
    assert!(old_balance.is_none());

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
        RuntimeBalances::try_from_iter([(ASSET_ID, balance_of_start)]).unwrap();
    let mut receipts = Default::default();
    let mut tx = Script::default();
    *tx.inputs_mut() = vec![Input::contract(
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        SOURCE_CONTRACT_ID,
    )];

    *tx.outputs_mut() = vec![Output::variable(
        RECIPIENT_ADDRESS,
        Default::default(),
        Default::default(),
    )];

    let tx_offset = 512;

    let output_range =
        absolute_output_mem_range(&tx, tx_offset, output_index as usize).unwrap();

    let transfer_ctx = TransferCtx {
        storage: &mut storage,
        memory: &mut memory,
        pc: RegMut::new(&mut pc),
        context: &context,
        balances: &mut balances,
        receipts: &mut receipts,
        profiler: &mut Default::default(),
        new_storage_gas_per_byte: 1,
        tx: &mut tx,
        tx_offset,
        cgas: RegMut::new(&mut cgas),
        ggas: RegMut::new(&mut ggas),
        fp: Reg::new(&fp),
        is: Reg::new(&is),
    };

    // When

    transfer_ctx.transfer_output(
        requested_recipient_offset,
        output_index,
        transfer_amount,
        requested_asset_id_offset,
    )?;

    // Then

    let final_contract_balance = storage
        .contract_asset_id_balance(&SOURCE_CONTRACT_ID, &ASSET_ID)
        .unwrap()
        .unwrap();

    assert_eq!(pc, 8);

    let output_bytes: &[u8] = &memory[output_range.start..output_range.end];
    let output = Output::from_bytes(output_bytes).unwrap();
    let output_amount = output.amount().unwrap();
    assert_eq!(output_amount, transfer_amount);

    if external {
        // In an external context, decrease MEM[balanceOfStart(MEM[$rD, 32]), 8] by $rC.
        assert_eq!(
            balances.balance(&ASSET_ID).unwrap(),
            balance_of_start - transfer_amount
        );
        assert_eq!(final_contract_balance, initial_contract_balance);
    } else {
        assert_eq!(balances.balance(&ASSET_ID).unwrap(), balance_of_start);
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
#[test_case(1, Word::MAX => Err(RuntimeError::Recoverable(PanicReason::BalanceOverflow)); "Overflowing add")]
#[test_case(Word::MAX, 1 => Err(RuntimeError::Recoverable(PanicReason::BalanceOverflow)); "Overflowing 1 add")]
fn test_balance_increase(
    initial: impl Into<Option<Word>>,
    amount: Word,
) -> IoResult<(), Infallible> {
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::default();
    let initial = initial.into();
    if let Some(initial) = initial {
        let old_balance = storage
            .contract_asset_id_balance_insert(&contract_id, &asset_id, initial)
            .unwrap();
        assert!(old_balance.is_none());
    }

    let (result, created_new_entry) =
        balance_increase(&mut storage, &contract_id, &asset_id, amount)?;

    assert!(initial.is_none() == created_new_entry);
    let initial = initial.unwrap_or(0);
    assert_eq!(result, initial + amount);

    let result = storage
        .contract_asset_id_balance(&contract_id, &asset_id)
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
) -> IoResult<(), Infallible> {
    let contract_id = ContractId::from([3u8; 32]);
    let asset_id = AssetId::from([2u8; 32]);
    let mut storage = MemoryStorage::default();
    let initial = initial.into();
    if let Some(initial) = initial {
        let old_balance = storage
            .contract_asset_id_balance_insert(&contract_id, &asset_id, initial)
            .unwrap();
        assert!(old_balance.is_none());
    }

    let result = balance_decrease(&mut storage, &contract_id, &asset_id, amount)?;
    let initial = initial.unwrap_or(0);

    assert_eq!(result, initial - amount);

    let result = storage
        .contract_asset_id_balance(&contract_id, &asset_id)
        .unwrap()
        .unwrap();

    assert_eq!(result, initial - amount);

    Ok(())
}
