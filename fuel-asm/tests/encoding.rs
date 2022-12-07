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
        op::gm_args(r, GMArgs::IsCallerExternal),
        op::gm_args(r, GMArgs::GetCaller),
        op::gm_args(r, GMArgs::GetVerifyingPredicate),
        op::gtf(r, r, imm12),
        op::gtf_args(r, r, GTFArgs::Type),
        op::gtf_args(r, r, GTFArgs::ScriptGasPrice),
        op::gtf_args(r, r, GTFArgs::ScriptGasLimit),
        op::gtf_args(r, r, GTFArgs::ScriptMaturity),
        op::gtf_args(r, r, GTFArgs::ScriptLength),
        op::gtf_args(r, r, GTFArgs::ScriptDataLength),
        op::gtf_args(r, r, GTFArgs::ScriptInputsCount),
        op::gtf_args(r, r, GTFArgs::ScriptOutputsCount),
        op::gtf_args(r, r, GTFArgs::ScriptWitnessesCound),
        op::gtf_args(r, r, GTFArgs::ScriptReceiptsRoot),
        op::gtf_args(r, r, GTFArgs::Script),
        op::gtf_args(r, r, GTFArgs::ScriptData),
        op::gtf_args(r, r, GTFArgs::ScriptInputAtIndex),
        op::gtf_args(r, r, GTFArgs::ScriptOutputAtIndex),
        op::gtf_args(r, r, GTFArgs::ScriptWitnessAtIndex),
        op::gtf_args(r, r, GTFArgs::CreateGasPrice),
        op::gtf_args(r, r, GTFArgs::CreateGasLimit),
        op::gtf_args(r, r, GTFArgs::CreateMaturity),
        op::gtf_args(r, r, GTFArgs::CreateBytecodeLength),
        op::gtf_args(r, r, GTFArgs::CreateBytecodeWitnessIndex),
        op::gtf_args(r, r, GTFArgs::CreateStorageSlotsCount),
        op::gtf_args(r, r, GTFArgs::CreateInputsCount),
        op::gtf_args(r, r, GTFArgs::CreateOutputsCount),
        op::gtf_args(r, r, GTFArgs::CreateWitnessesCount),
        op::gtf_args(r, r, GTFArgs::CreateSalt),
        op::gtf_args(r, r, GTFArgs::CreateStorageSlotAtIndex),
        op::gtf_args(r, r, GTFArgs::CreateInputAtIndex),
        op::gtf_args(r, r, GTFArgs::CreateOutputAtIndex),
        op::gtf_args(r, r, GTFArgs::CreateWitnessAtIndex),
        op::gtf_args(r, r, GTFArgs::InputType),
        op::gtf_args(r, r, GTFArgs::InputCoinTxId),
        op::gtf_args(r, r, GTFArgs::InputCoinOutputIndex),
        op::gtf_args(r, r, GTFArgs::InputCoinOwner),
        op::gtf_args(r, r, GTFArgs::InputCoinAmount),
        op::gtf_args(r, r, GTFArgs::InputCoinAssetId),
        op::gtf_args(r, r, GTFArgs::InputCoinTxPointer),
        op::gtf_args(r, r, GTFArgs::InputCoinWitnessIndex),
        op::gtf_args(r, r, GTFArgs::InputCoinMaturity),
        op::gtf_args(r, r, GTFArgs::InputCoinPredicateLength),
        op::gtf_args(r, r, GTFArgs::InputCoinPredicateDataLength),
        op::gtf_args(r, r, GTFArgs::InputCoinPredicate),
        op::gtf_args(r, r, GTFArgs::InputCoinPredicateData),
        op::gtf_args(r, r, GTFArgs::InputContractTxId),
        op::gtf_args(r, r, GTFArgs::InputContractOutputIndex),
        op::gtf_args(r, r, GTFArgs::InputContractBalanceRoot),
        op::gtf_args(r, r, GTFArgs::InputContractStateRoot),
        op::gtf_args(r, r, GTFArgs::InputContractTxPointer),
        op::gtf_args(r, r, GTFArgs::InputContractId),
        op::gtf_args(r, r, GTFArgs::InputMessageId),
        op::gtf_args(r, r, GTFArgs::InputMessageSender),
        op::gtf_args(r, r, GTFArgs::InputMessageRecipient),
        op::gtf_args(r, r, GTFArgs::InputMessageAmount),
        op::gtf_args(r, r, GTFArgs::InputMessageNonce),
        op::gtf_args(r, r, GTFArgs::InputMessageWitnessIndex),
        op::gtf_args(r, r, GTFArgs::InputMessageDataLength),
        op::gtf_args(r, r, GTFArgs::InputMessagePredicateLength),
        op::gtf_args(r, r, GTFArgs::InputMessagePredicateDataLength),
        op::gtf_args(r, r, GTFArgs::InputMessageData),
        op::gtf_args(r, r, GTFArgs::InputMessagePredicate),
        op::gtf_args(r, r, GTFArgs::InputMessagePredicateData),
        op::gtf_args(r, r, GTFArgs::OutputType),
        op::gtf_args(r, r, GTFArgs::OutputCoinTo),
        op::gtf_args(r, r, GTFArgs::OutputCoinAmount),
        op::gtf_args(r, r, GTFArgs::OutputCoinAssetId),
        op::gtf_args(r, r, GTFArgs::OutputContractInputIndex),
        op::gtf_args(r, r, GTFArgs::OutputContractBalanceRoot),
        op::gtf_args(r, r, GTFArgs::OutputContractStateRoot),
        op::gtf_args(r, r, GTFArgs::OutputMessageRecipient),
        op::gtf_args(r, r, GTFArgs::OutputMessageAmount),
        op::gtf_args(r, r, GTFArgs::OutputContractCreatedContractId),
        op::gtf_args(r, r, GTFArgs::OutputContractCreatedStateRoot),
        op::gtf_args(r, r, GTFArgs::WitnessDataLength),
        op::gtf_args(r, r, GTFArgs::WitnessData),
    ];

    // Pad to even length
    if instructions.len() % 2 != 0 {
        instructions.push(op::noop());
    }

    let bytes: Vec<u8> = instructions.iter().copied().collect();

    let instructions_from_bytes: Result<Vec<Instruction>, _> = fuel_asm::from_bytes(bytes.iter().copied()).collect();

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

    let pd = InstructionResult::error(PanicReason::Success, op::noop().into());
    let w = Word::from(pd);
    let pd_p = InstructionResult::from(w);
    assert_eq!(pd, pd_p);

    #[cfg(feature = "serde")]
    {
        let pd_s = bincode::serialize(&pd).expect("Failed to serialize instruction");
        let pd_s: InstructionResult = bincode::deserialize(&pd_s).expect("Failed to deserialize instruction");

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
            let pd_s: InstructionResult = bincode::deserialize(&pd_s).expect("Failed to deserialize instruction");

            assert_eq!(pd_s, pd);
        }
    }
}
