#![allow(clippy::iter_cloned_collect)] // https://github.com/rust-lang/rust-clippy/issues/9119

use fuel_asm::*;

#[test]
fn opcode() {
    // values picked to test edge cases
    let r = 0x2d;
    let imm12 = 0x0bfd;
    let imm18 = 0x02fffd;
    let imm24 = 0xbffffd;

    let mut instructions = vec![
        op::add(r, r, r),
        op::addi(r, r, imm12),
        op::and(r, r, r),
        op::andi(r, r, imm12),
        op::div(r, r, r),
        op::divi(r, r, imm12),
        op::eq(r, r, r),
        op::exp(r, r, r),
        op::expi(r, r, imm12),
        op::gt(r, r, r),
        op::lt(r, r, r),
        op::mlog(r, r, r),
        op::mroo(r, r, r),
        op::mod_(r, r, r),
        op::modi(r, r, imm12),
        op::move_(r, r),
        op::movi(r, imm18),
        op::mul(r, r, r),
        op::muli(r, r, imm12),
        op::not(r, r),
        op::or(r, r, r),
        op::ori(r, r, imm12),
        op::sll(r, r, r),
        op::slli(r, r, imm12),
        op::srl(r, r, r),
        op::srli(r, r, imm12),
        op::sub(r, r, r),
        op::subi(r, r, imm12),
        op::xor(r, r, r),
        op::xori(r, r, imm12),
        op::ji(imm24),
        op::jnei(r, r, imm12),
        op::jnzi(r, imm18),
        op::jmp(r),
        op::jne(r, r, r),
        op::ret(r),
        op::retd(r, r),
        op::cfei(imm24),
        op::cfsi(imm24),
        op::lb(r, r, imm12),
        op::lw(r, r, imm12),
        op::aloc(r),
        op::mcl(r, r),
        op::mcli(r, imm18),
        op::mcp(r, r, r),
        op::mcpi(r, r, imm12),
        op::meq(r, r, r, r),
        op::sb(r, r, imm12),
        op::sw(r, r, imm12),
        op::bal(r, r, r),
        op::bhsh(r, r),
        op::bhei(r),
        op::burn(r),
        op::call(r, r, r, r),
        op::ccp(r, r, r, r),
        op::croo(r, r),
        op::csiz(r, r),
        op::cb(r),
        op::ldc(r, r, r),
        op::log(r, r, r, r),
        op::logd(r, r, r, r),
        op::mint(r),
        op::rvrt(r),
        op::smo(r, r, r, r),
        op::scwq(r, r, r),
        op::srw(r, r, r),
        op::srwq(r, r, r, r),
        op::sww(r, r, r),
        op::swwq(r, r, r, r),
        op::time(r, r),
        op::tr(r, r, r),
        op::tro(r, r, r, r),
        op::ecr(r, r, r),
        op::k256(r, r, r),
        op::s256(r, r, r),
        op::noop(),
        op::flag(r),
        op::gm(r, imm18),
        Instruction::gm(r, GMArgs::IsCallerExternal),
        Instruction::gm(r, GMArgs::GetCaller),
        Instruction::gm(r, GMArgs::GetVerifyingPredicate),
        op::gtf(r, r, imm12),
        Instruction::gtf(r, r, GTFArgs::Type),
        Instruction::gtf(r, r, GTFArgs::ScriptGasPrice),
        Instruction::gtf(r, r, GTFArgs::ScriptGasLimit),
        Instruction::gtf(r, r, GTFArgs::ScriptMaturity),
        Instruction::gtf(r, r, GTFArgs::ScriptLength),
        Instruction::gtf(r, r, GTFArgs::ScriptDataLength),
        Instruction::gtf(r, r, GTFArgs::ScriptInputsCount),
        Instruction::gtf(r, r, GTFArgs::ScriptOutputsCount),
        Instruction::gtf(r, r, GTFArgs::ScriptWitnessesCound),
        Instruction::gtf(r, r, GTFArgs::ScriptReceiptsRoot),
        Instruction::gtf(r, r, GTFArgs::Script),
        Instruction::gtf(r, r, GTFArgs::ScriptData),
        Instruction::gtf(r, r, GTFArgs::ScriptInputAtIndex),
        Instruction::gtf(r, r, GTFArgs::ScriptOutputAtIndex),
        Instruction::gtf(r, r, GTFArgs::ScriptWitnessAtIndex),
        Instruction::gtf(r, r, GTFArgs::CreateGasPrice),
        Instruction::gtf(r, r, GTFArgs::CreateGasLimit),
        Instruction::gtf(r, r, GTFArgs::CreateMaturity),
        Instruction::gtf(r, r, GTFArgs::CreateBytecodeLength),
        Instruction::gtf(r, r, GTFArgs::CreateBytecodeWitnessIndex),
        Instruction::gtf(r, r, GTFArgs::CreateStorageSlotsCount),
        Instruction::gtf(r, r, GTFArgs::CreateInputsCount),
        Instruction::gtf(r, r, GTFArgs::CreateOutputsCount),
        Instruction::gtf(r, r, GTFArgs::CreateWitnessesCount),
        Instruction::gtf(r, r, GTFArgs::CreateSalt),
        Instruction::gtf(r, r, GTFArgs::CreateStorageSlotAtIndex),
        Instruction::gtf(r, r, GTFArgs::CreateInputAtIndex),
        Instruction::gtf(r, r, GTFArgs::CreateOutputAtIndex),
        Instruction::gtf(r, r, GTFArgs::CreateWitnessAtIndex),
        Instruction::gtf(r, r, GTFArgs::InputType),
        Instruction::gtf(r, r, GTFArgs::InputCoinTxId),
        Instruction::gtf(r, r, GTFArgs::InputCoinOutputIndex),
        Instruction::gtf(r, r, GTFArgs::InputCoinOwner),
        Instruction::gtf(r, r, GTFArgs::InputCoinAmount),
        Instruction::gtf(r, r, GTFArgs::InputCoinAssetId),
        Instruction::gtf(r, r, GTFArgs::InputCoinTxPointer),
        Instruction::gtf(r, r, GTFArgs::InputCoinWitnessIndex),
        Instruction::gtf(r, r, GTFArgs::InputCoinMaturity),
        Instruction::gtf(r, r, GTFArgs::InputCoinPredicateLength),
        Instruction::gtf(r, r, GTFArgs::InputCoinPredicateDataLength),
        Instruction::gtf(r, r, GTFArgs::InputCoinPredicate),
        Instruction::gtf(r, r, GTFArgs::InputCoinPredicateData),
        Instruction::gtf(r, r, GTFArgs::InputContractTxId),
        Instruction::gtf(r, r, GTFArgs::InputContractOutputIndex),
        Instruction::gtf(r, r, GTFArgs::InputContractBalanceRoot),
        Instruction::gtf(r, r, GTFArgs::InputContractStateRoot),
        Instruction::gtf(r, r, GTFArgs::InputContractTxPointer),
        Instruction::gtf(r, r, GTFArgs::InputContractId),
        Instruction::gtf(r, r, GTFArgs::InputMessageId),
        Instruction::gtf(r, r, GTFArgs::InputMessageSender),
        Instruction::gtf(r, r, GTFArgs::InputMessageRecipient),
        Instruction::gtf(r, r, GTFArgs::InputMessageAmount),
        Instruction::gtf(r, r, GTFArgs::InputMessageNonce),
        Instruction::gtf(r, r, GTFArgs::InputMessageWitnessIndex),
        Instruction::gtf(r, r, GTFArgs::InputMessageDataLength),
        Instruction::gtf(r, r, GTFArgs::InputMessagePredicateLength),
        Instruction::gtf(r, r, GTFArgs::InputMessagePredicateDataLength),
        Instruction::gtf(r, r, GTFArgs::InputMessageData),
        Instruction::gtf(r, r, GTFArgs::InputMessagePredicate),
        Instruction::gtf(r, r, GTFArgs::InputMessagePredicateData),
        Instruction::gtf(r, r, GTFArgs::OutputType),
        Instruction::gtf(r, r, GTFArgs::OutputCoinTo),
        Instruction::gtf(r, r, GTFArgs::OutputCoinAmount),
        Instruction::gtf(r, r, GTFArgs::OutputCoinAssetId),
        Instruction::gtf(r, r, GTFArgs::OutputContractInputIndex),
        Instruction::gtf(r, r, GTFArgs::OutputContractBalanceRoot),
        Instruction::gtf(r, r, GTFArgs::OutputContractStateRoot),
        Instruction::gtf(r, r, GTFArgs::OutputMessageRecipient),
        Instruction::gtf(r, r, GTFArgs::OutputMessageAmount),
        Instruction::gtf(r, r, GTFArgs::OutputContractCreatedContractId),
        Instruction::gtf(r, r, GTFArgs::OutputContractCreatedStateRoot),
        Instruction::gtf(r, r, GTFArgs::WitnessDataLength),
        Instruction::gtf(r, r, GTFArgs::WitnessData),
    ];

    // Pad to even length
    if instructions.len() % 2 != 0 {
        instructions.push(op::noop());
    }

    let bytes: Vec<u8> = instructions.iter().copied().collect();

    let instructions_from_bytes: Result<Vec<Instruction>, _> =
        fuel_asm::from_bytes(bytes.iter().copied()).collect();

    assert_eq!(instructions, instructions_from_bytes.unwrap());

    let words: Vec<Word> = bytes
        .chunks(core::mem::size_of::<Word>())
        .map(|chunk| {
            let mut arr = [0; core::mem::size_of::<Word>()];
            arr.copy_from_slice(chunk);
            Word::from_be_bytes(arr)
        })
        .collect();

    let instructions_from_words: Vec<Instruction> = words
        .into_iter()
        .flat_map(raw_instructions_from_word)
        .map(|raw| Instruction::try_from(raw).unwrap())
        .collect();

    assert_eq!(instructions, instructions_from_words);
}

#[test]
fn panic_reason_description() {
    let imm24 = 0xbfffff;

    let reasons = vec![
        PanicReason::Revert,
        PanicReason::OutOfGas,
        PanicReason::TransactionValidity,
        PanicReason::MemoryOverflow,
        PanicReason::ArithmeticOverflow,
        PanicReason::ContractNotFound,
        PanicReason::MemoryOwnership,
        PanicReason::NotEnoughBalance,
        PanicReason::ExpectedInternalContext,
        PanicReason::AssetIdNotFound,
        PanicReason::InputNotFound,
        PanicReason::OutputNotFound,
        PanicReason::WitnessNotFound,
        PanicReason::TransactionMaturity,
        PanicReason::InvalidMetadataIdentifier,
        PanicReason::MalformedCallStructure,
        PanicReason::ReservedRegisterNotWritable,
        PanicReason::ErrorFlag,
        PanicReason::InvalidImmediateValue,
        PanicReason::ExpectedCoinInput,
        PanicReason::MaxMemoryAccess,
        PanicReason::MemoryWriteOverlap,
        PanicReason::ContractNotInInputs,
        PanicReason::InternalBalanceOverflow,
        PanicReason::ContractMaxSize,
        PanicReason::ExpectedUnallocatedStack,
        PanicReason::MaxStaticContractsReached,
        PanicReason::TransferAmountCannotBeZero,
        PanicReason::ExpectedOutputVariable,
        PanicReason::ExpectedParentInternalContext,
        PanicReason::IllegalJump,
    ];

    let pd = InstructionResult::error(PanicReason::Success, op::noop());
    let w = Word::from(pd);
    let pd_p = InstructionResult::from(w);
    assert_eq!(pd, pd_p);

    #[cfg(feature = "serde")]
    {
        let pd_s = bincode::serialize(&pd).expect("Failed to serialize instruction");
        let pd_s: InstructionResult =
            bincode::deserialize(&pd_s).expect("Failed to deserialize instruction");

        assert_eq!(pd_s, pd);
    }

    for r in reasons {
        let b = r as u8;
        let r_p = PanicReason::from(b);
        let w = Word::from(r as u8);
        let r_q = PanicReason::from(u8::try_from(w).unwrap());
        assert_eq!(r, r_p);
        assert_eq!(r, r_q);

        let op = op::ji(imm24);
        let pd = InstructionResult::error(r, op.into());
        let w = Word::from(pd);
        let pd_p = InstructionResult::from(w);
        assert_eq!(pd, pd_p);

        #[cfg(feature = "serde")]
        {
            let pd_s = bincode::serialize(&pd).expect("Failed to serialize instruction");
            let pd_s: InstructionResult =
                bincode::deserialize(&pd_s).expect("Failed to deserialize instruction");

            assert_eq!(pd_s, pd);
        }
    }
}
