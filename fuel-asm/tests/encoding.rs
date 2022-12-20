#![allow(clippy::iter_cloned_collect)] // https://github.com/rust-lang/rust-clippy/issues/9119

use fuel_asm::*;
use std::io::{Read, Write};

#[test]
fn opcode() {
    // values picked to test edge cases
    let r = 0x2d;
    let imm12 = 0x0bfd;
    let imm18 = 0x02fffd;
    let imm24 = 0xbffffd;

    let mut data = vec![
        Opcode::ADD(r, r, r),
        Opcode::ADDI(r, r, imm12),
        Opcode::AND(r, r, r),
        Opcode::ANDI(r, r, imm12),
        Opcode::DIV(r, r, r),
        Opcode::DIVI(r, r, imm12),
        Opcode::EQ(r, r, r),
        Opcode::EXP(r, r, r),
        Opcode::EXPI(r, r, imm12),
        Opcode::GT(r, r, r),
        Opcode::LT(r, r, r),
        Opcode::MLOG(r, r, r),
        Opcode::MROO(r, r, r),
        Opcode::MOD(r, r, r),
        Opcode::MODI(r, r, imm12),
        Opcode::MOVE(r, r),
        Opcode::MOVI(r, imm18),
        Opcode::MUL(r, r, r),
        Opcode::MULI(r, r, imm12),
        Opcode::NOT(r, r),
        Opcode::OR(r, r, r),
        Opcode::ORI(r, r, imm12),
        Opcode::SLL(r, r, r),
        Opcode::SLLI(r, r, imm12),
        Opcode::SRL(r, r, r),
        Opcode::SRLI(r, r, imm12),
        Opcode::SUB(r, r, r),
        Opcode::SUBI(r, r, imm12),
        Opcode::XOR(r, r, r),
        Opcode::XORI(r, r, imm12),
        Opcode::JI(imm24),
        Opcode::JNEI(r, r, imm12),
        Opcode::JNZI(r, imm18),
        Opcode::JMP(r),
        Opcode::JNE(r, r, r),
        Opcode::RET(r),
        Opcode::RETD(r, r),
        Opcode::CFEI(imm24),
        Opcode::CFSI(imm24),
        Opcode::LB(r, r, imm12),
        Opcode::LW(r, r, imm12),
        Opcode::ALOC(r),
        Opcode::MCL(r, r),
        Opcode::MCLI(r, imm18),
        Opcode::MCP(r, r, r),
        Opcode::MCPI(r, r, imm12),
        Opcode::MEQ(r, r, r, r),
        Opcode::SB(r, r, imm12),
        Opcode::SW(r, r, imm12),
        Opcode::BAL(r, r, r),
        Opcode::BHSH(r, r),
        Opcode::BHEI(r),
        Opcode::BURN(r),
        Opcode::CALL(r, r, r, r),
        Opcode::CCP(r, r, r, r),
        Opcode::CROO(r, r),
        Opcode::CSIZ(r, r),
        Opcode::CB(r),
        Opcode::LDC(r, r, r),
        Opcode::LOG(r, r, r, r),
        Opcode::LOGD(r, r, r, r),
        Opcode::MINT(r),
        Opcode::RVRT(r),
        Opcode::SMO(r, r, r, r),
        Opcode::SCWQ(r, r, r),
        Opcode::SRW(r, r, r),
        Opcode::SRWQ(r, r, r, r),
        Opcode::SWW(r, r, r),
        Opcode::SWWQ(r, r, r, r),
        Opcode::TIME(r, r),
        Opcode::TR(r, r, r),
        Opcode::TRO(r, r, r, r),
        Opcode::ECR(r, r, r),
        Opcode::K256(r, r, r),
        Opcode::S256(r, r, r),
        Opcode::NOOP,
        Opcode::FLAG(r),
        Opcode::GM(r, imm18),
        Opcode::gm(r, GMArgs::IsCallerExternal),
        Opcode::gm(r, GMArgs::GetCaller),
        Opcode::gm(r, GMArgs::GetVerifyingPredicate),
        Opcode::GTF(r, r, imm12),
        Opcode::gtf(r, r, GTFArgs::Type),
        Opcode::gtf(r, r, GTFArgs::ScriptGasPrice),
        Opcode::gtf(r, r, GTFArgs::ScriptGasLimit),
        Opcode::gtf(r, r, GTFArgs::ScriptMaturity),
        Opcode::gtf(r, r, GTFArgs::ScriptLength),
        Opcode::gtf(r, r, GTFArgs::ScriptDataLength),
        Opcode::gtf(r, r, GTFArgs::ScriptInputsCount),
        Opcode::gtf(r, r, GTFArgs::ScriptOutputsCount),
        Opcode::gtf(r, r, GTFArgs::ScriptWitnessesCound),
        Opcode::gtf(r, r, GTFArgs::ScriptReceiptsRoot),
        Opcode::gtf(r, r, GTFArgs::Script),
        Opcode::gtf(r, r, GTFArgs::ScriptData),
        Opcode::gtf(r, r, GTFArgs::ScriptInputAtIndex),
        Opcode::gtf(r, r, GTFArgs::ScriptOutputAtIndex),
        Opcode::gtf(r, r, GTFArgs::ScriptWitnessAtIndex),
        Opcode::gtf(r, r, GTFArgs::CreateGasPrice),
        Opcode::gtf(r, r, GTFArgs::CreateGasLimit),
        Opcode::gtf(r, r, GTFArgs::CreateMaturity),
        Opcode::gtf(r, r, GTFArgs::CreateBytecodeLength),
        Opcode::gtf(r, r, GTFArgs::CreateBytecodeWitnessIndex),
        Opcode::gtf(r, r, GTFArgs::CreateStorageSlotsCount),
        Opcode::gtf(r, r, GTFArgs::CreateInputsCount),
        Opcode::gtf(r, r, GTFArgs::CreateOutputsCount),
        Opcode::gtf(r, r, GTFArgs::CreateWitnessesCount),
        Opcode::gtf(r, r, GTFArgs::CreateSalt),
        Opcode::gtf(r, r, GTFArgs::CreateStorageSlotAtIndex),
        Opcode::gtf(r, r, GTFArgs::CreateInputAtIndex),
        Opcode::gtf(r, r, GTFArgs::CreateOutputAtIndex),
        Opcode::gtf(r, r, GTFArgs::CreateWitnessAtIndex),
        Opcode::gtf(r, r, GTFArgs::InputType),
        Opcode::gtf(r, r, GTFArgs::InputCoinTxId),
        Opcode::gtf(r, r, GTFArgs::InputCoinOutputIndex),
        Opcode::gtf(r, r, GTFArgs::InputCoinOwner),
        Opcode::gtf(r, r, GTFArgs::InputCoinAmount),
        Opcode::gtf(r, r, GTFArgs::InputCoinAssetId),
        Opcode::gtf(r, r, GTFArgs::InputCoinTxPointer),
        Opcode::gtf(r, r, GTFArgs::InputCoinWitnessIndex),
        Opcode::gtf(r, r, GTFArgs::InputCoinMaturity),
        Opcode::gtf(r, r, GTFArgs::InputCoinPredicateLength),
        Opcode::gtf(r, r, GTFArgs::InputCoinPredicateDataLength),
        Opcode::gtf(r, r, GTFArgs::InputCoinPredicate),
        Opcode::gtf(r, r, GTFArgs::InputCoinPredicateData),
        Opcode::gtf(r, r, GTFArgs::InputContractTxId),
        Opcode::gtf(r, r, GTFArgs::InputContractOutputIndex),
        Opcode::gtf(r, r, GTFArgs::InputContractBalanceRoot),
        Opcode::gtf(r, r, GTFArgs::InputContractStateRoot),
        Opcode::gtf(r, r, GTFArgs::InputContractTxPointer),
        Opcode::gtf(r, r, GTFArgs::InputContractId),
        Opcode::gtf(r, r, GTFArgs::InputMessageId),
        Opcode::gtf(r, r, GTFArgs::InputMessageSender),
        Opcode::gtf(r, r, GTFArgs::InputMessageRecipient),
        Opcode::gtf(r, r, GTFArgs::InputMessageAmount),
        Opcode::gtf(r, r, GTFArgs::InputMessageNonce),
        Opcode::gtf(r, r, GTFArgs::InputMessageWitnessIndex),
        Opcode::gtf(r, r, GTFArgs::InputMessageDataLength),
        Opcode::gtf(r, r, GTFArgs::InputMessagePredicateLength),
        Opcode::gtf(r, r, GTFArgs::InputMessagePredicateDataLength),
        Opcode::gtf(r, r, GTFArgs::InputMessageData),
        Opcode::gtf(r, r, GTFArgs::InputMessagePredicate),
        Opcode::gtf(r, r, GTFArgs::InputMessagePredicateData),
        Opcode::gtf(r, r, GTFArgs::OutputType),
        Opcode::gtf(r, r, GTFArgs::OutputCoinTo),
        Opcode::gtf(r, r, GTFArgs::OutputCoinAmount),
        Opcode::gtf(r, r, GTFArgs::OutputCoinAssetId),
        Opcode::gtf(r, r, GTFArgs::OutputContractInputIndex),
        Opcode::gtf(r, r, GTFArgs::OutputContractBalanceRoot),
        Opcode::gtf(r, r, GTFArgs::OutputContractStateRoot),
        Opcode::gtf(r, r, GTFArgs::OutputMessageRecipient),
        Opcode::gtf(r, r, GTFArgs::OutputMessageAmount),
        Opcode::gtf(r, r, GTFArgs::OutputContractCreatedContractId),
        Opcode::gtf(r, r, GTFArgs::OutputContractCreatedStateRoot),
        Opcode::gtf(r, r, GTFArgs::WitnessDataLength),
        Opcode::gtf(r, r, GTFArgs::WitnessData),
        Opcode::Undefined,
    ];

    // Pad to even length
    if data.len() % 2 != 0 {
        data.push(Opcode::Undefined);
    }

    let bytes: Vec<u8> = data.iter().copied().collect();

    let data_p = Opcode::from_bytes_iter(bytes.clone());
    let data_q = Instruction::from_bytes_iter(bytes.clone());
    let data_q: Vec<Opcode> = data_q.into_iter().collect();

    assert_eq!(data, data_p);
    assert_eq!(data, data_q);

    let pairs = bytes.chunks(8).into_iter().map(|chunk| {
        let mut arr = [0; core::mem::size_of::<Word>()];
        arr.copy_from_slice(chunk);
        Instruction::parse_word(Word::from_be_bytes(arr))
    });

    let result: Vec<Opcode> = pairs
        .into_iter()
        .flat_map(|(a, b)| [Opcode::from(a), Opcode::from(b)])
        .collect();

    assert_eq!(data, result);

    let mut bytes: Vec<u8> = vec![];
    let mut buffer = [0u8; Opcode::LEN];

    for mut op in data.clone() {
        let _ = op
            .read(&mut buffer)
            .expect("Failed to write opcode to buffer");
        bytes.extend(&buffer);

        let op_p = u32::from(op);
        let op_bytes = op_p.to_be_bytes().to_vec();

        let ins = Instruction::from(op_p);
        let ins_p = Instruction::from(op);

        assert_eq!(ins, ins_p);

        let (_, _, _, _, _, imm_ins) = ins.into_inner();
        let imm_op = op.immediate().unwrap_or_default();

        assert_eq!(imm_op, imm_ins);

        let op_p = Opcode::from(op_p);
        let op_q = unsafe { Opcode::from_bytes_unchecked(op_bytes.as_slice()) };

        assert_eq!(op, Opcode::from(ins));
        assert_eq!(op, op_p);
        assert_eq!(op, op_q);

        let mut op_bytes = op.to_bytes().to_vec();

        // Assert opcode can be created from big slices
        op_bytes.extend_from_slice(&[0xff; 25]);
        while op_bytes.len() > Opcode::LEN {
            op_bytes.pop();

            let op_r = unsafe { Opcode::from_bytes_unchecked(op_bytes.as_slice()) };
            let op_s = Opcode::from_bytes(op_bytes.as_slice())
                .expect("Failed to safely generate op from bytes!");

            assert_eq!(op, op_r);
            assert_eq!(op, op_s);

            let ins_r = unsafe { Instruction::from_slice_unchecked(op_bytes.as_slice()) };
            let ins_s = Instruction::from_bytes(op_bytes.as_slice())
                .expect("Failed to safely generate op from bytes!");

            assert_eq!(op, Opcode::from(ins_r));
            assert_eq!(op, Opcode::from(ins_s));
        }

        // Assert no panic with checked function
        while !op_bytes.is_empty() {
            op_bytes.pop();

            assert!(Opcode::from_bytes(op_bytes.as_slice()).is_err());
        }

        #[cfg(feature = "serde")]
        {
            let op_s = bincode::serialize(&op).expect("Failed to serialize opcode");
            let op_s: Opcode = bincode::deserialize(&op_s).expect("Failed to deserialize opcode");

            assert_eq!(op_s, op);
        }
    }

    let mut op_p = Opcode::Undefined;
    bytes
        .chunks(Opcode::LEN)
        .zip(data.iter())
        .for_each(|(chunk, op)| {
            let _ = op_p
                .write(chunk)
                .expect("Failed to parse opcode from chunk");

            assert_eq!(op, &op_p);
        });
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

    let pd = InstructionResult::success();

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
        let b = u8::from(r);
        let r_p = PanicReason::from(b);

        let w = Word::from(r);
        let r_q = PanicReason::from(w);

        assert_eq!(r, r_p);
        assert_eq!(r, r_q);

        let op = Opcode::JI(imm24);
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
