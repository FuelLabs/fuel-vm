use fuel_tx::bytes;
use fuel_tx::crypto::Hasher;
use fuel_vm::consts::*;
use fuel_vm::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[test]
fn state_read_write() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;

    let salt: Salt = rng.gen();

    #[rustfmt::skip]
    let program: Witness = [
        Opcode::ADDI(0x30, REG_ZERO, 0),
        Opcode::ADDI(0x31, REG_ONE, 0),

        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),

        Opcode::JNEI(0x10, 0x30, 13),       // (0, b) Add word to state
        Opcode::LW(0x20, 0x11, 4),          // r[0x20]      := m[b+32, 8]
        Opcode::SRW(0x21, 0x11),            // r[0x21]      := s[m[b, 32], 8]
        Opcode::ADD(0x20, 0x20, 0x21),      // r[0x20]      += r[0x21]
        Opcode::SWW(0x11, 0x20),            // s[m[b,32]]   := r[0x20]
        Opcode::LOG(0x20, 0x21, 0x00, 0x00),
        Opcode::RET(REG_ONE),

        Opcode::JNEI(0x10, 0x31, 45),       // (1, b) Unpack arg into 4x16 and xor into state
        Opcode::ADDI(0x20, REG_ZERO, 32),   // r[0x20]      := 32
        Opcode::ALOC(0x20),                 // aloc            0x20
        Opcode::ADDI(0x20, REG_HP, 1),      // r[0x20]      := $hp+1
        Opcode::SRWQ(0x20, 0x11),           // m[0x20,32]   := s[m[b, 32], 32]
        Opcode::LW(0x21, 0x11, 4),          // r[0x21]      := m[b+32, 8]
        Opcode::LOG(0x21, 0x00, 0x00, 0x00),
        Opcode::SRLI(0x22, 0x21, 48),       // r[0x22]      := r[0x21] >> 48
        Opcode::SRLI(0x23, 0x21, 32),       // r[0x23]      := r[0x21] >> 32
        Opcode::ANDI(0x23, 0x23, 0xff),     // r[0x23]      &= 0xffff
        Opcode::SRLI(0x24, 0x21, 16),       // r[0x24]      := r[0x21] >> 16
        Opcode::ANDI(0x24, 0x24, 0xff),     // r[0x24]      &= 0xffff
        Opcode::ANDI(0x25, 0x21, 0xff),     // r[0x25]      := r[0x21] & 0xffff
        Opcode::LOG(0x22, 0x23, 0x24, 0x25),
        Opcode::LW(0x26, 0x20, 0),          // r[0x26]      := m[$fp, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 0),          // m[0x20,8]    := r[0x26]
        Opcode::LW(0x26, 0x20, 1),          // r[0x26]      := m[$fp+8, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 1),          // m[0x20+8,8]  := r[0x26]
        Opcode::LW(0x26, 0x20, 2),          // r[0x26]      := m[$fp+16, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 2),          // m[0x20+16,8] := r[0x26]
        Opcode::LW(0x26, 0x20, 3),          // r[0x26]      := m[$fp+24, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 3),          // m[0x20+24,8] := r[0x26]
        Opcode::SWWQ(0x11, 0x20),           // s[m[b,32],32]:= m[0x20, 32]
        Opcode::RET(REG_ONE),

        Opcode::RET(REG_ZERO),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>()
    .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let contract = contract.id(&salt, &contract_root);

    let output = Output::contract_created(contract);

    let bytecode_witness = 0;
    let tx = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    );

    Interpreter::transition(&mut storage, tx).expect("Failed to transact");

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let script_len = 16;

    let script_data_offset = VM_TX_MEMORY + Transaction::script_offset() + script_len;
    let script_data_offset = script_data_offset as Immediate12;

    let script = vec![
        Opcode::ADDI(0x10, REG_ZERO, script_data_offset),
        Opcode::ADDI(0x11, REG_ZERO, gas_limit as Immediate12),
        Opcode::CALL(0x10, REG_ZERO, 0x10, 0x11),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>();

    let offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script.as_slice());

    let script: Vec<u8> = script.into();

    assert_eq!(script_data_offset, offset as Immediate12);

    let mut script_data = vec![];

    let routine: Word = 0;

    let call_data_offset = script_data_offset as usize + ContractId::size_of() + 2 * WORD_SIZE;
    let call_data_offset = call_data_offset as Word;

    let key = Hasher::hash(b"some key");
    let val: Word = 150;

    script_data.extend(contract.as_ref());
    script_data.extend(&routine.to_be_bytes());
    script_data.extend(&call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(&val.to_be_bytes());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script.clone(),
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    );

    let state = storage.contract_state(&contract, &key);
    assert_eq!(Bytes32::default(), state);

    let transition = Interpreter::transition(&mut storage, tx).expect("Failed to transact");

    let state = storage.contract_state(&contract, &key);
    assert_eq!(&val.to_be_bytes()[..], &state.as_ref()[..WORD_SIZE]);

    let log = transition.log();
    assert!(matches!(log[0], LogEvent::Register { register, value, .. } if register == 0x20 && value == val));
    assert!(matches!(log[1], LogEvent::Register { register, value, .. } if register == 0x21 && value == 0));

    let mut script_data = vec![];

    let routine: Word = 1;

    let a = 0x25;
    let b = 0xc1;
    let c = 0xd3;
    let d = 0xaa;

    let val: Word = (a << 48) | (b << 32) | (c << 16) | d;

    script_data.extend(contract.as_ref());
    script_data.extend(&routine.to_be_bytes());
    script_data.extend(&call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(&val.to_be_bytes());

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    );

    let transition = Interpreter::transition(&mut storage, tx).expect("Failed to transact");

    let log = transition.log();

    assert!(matches!(log[0], LogEvent::Register { register, value, .. } if register == 0x21 && value == val));
    assert!(matches!(log[1], LogEvent::Register { register, value, .. } if register == 0x22 && value == a));
    assert!(matches!(log[2], LogEvent::Register { register, value, .. } if register == 0x23 && value == b));
    assert!(matches!(log[3], LogEvent::Register { register, value, .. } if register == 0x24 && value == c));
    assert!(matches!(log[4], LogEvent::Register { register, value, .. } if register == 0x25 && value == d));

    let m = a ^ 0x96;
    let n = a ^ 0x00;
    let o = a ^ 0x00;
    let p = a ^ 0x00;

    assert!(matches!(log[5], LogEvent::Register { register, value, .. } if register == 0x26 && value == m));
    assert!(matches!(log[6], LogEvent::Register { register, value, .. } if register == 0x26 && value == n));
    assert!(matches!(log[7], LogEvent::Register { register, value, .. } if register == 0x26 && value == o));
    assert!(matches!(log[8], LogEvent::Register { register, value, .. } if register == 0x26 && value == p));

    let mut bytes = [0u8; 32];

    (&mut bytes[..8]).copy_from_slice(&m.to_be_bytes());
    (&mut bytes[8..16]).copy_from_slice(&n.to_be_bytes());
    (&mut bytes[16..24]).copy_from_slice(&o.to_be_bytes());
    (&mut bytes[24..]).copy_from_slice(&p.to_be_bytes());

    let bytes = Bytes32::from(bytes);
    let state = storage.contract_state(&contract, &key);
    assert_eq!(bytes, state);
}
