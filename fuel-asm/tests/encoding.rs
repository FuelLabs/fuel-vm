#[test]
fn opcode() {
    use fuel_asm::*;
    use std::io::{Read, Write};

    let r = 0x3f;
    let imm12 = 0xbff;
    let imm18 = 0x2ffff;
    let imm24 = 0xbfffff;

    let data = vec![
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
        Opcode::CIMV(r, r, r),
        Opcode::CTMV(r, r),
        Opcode::JI(imm24),
        Opcode::JNEI(r, r, imm12),
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
        Opcode::MEQ(r, r, r, r),
        Opcode::SB(r, r, imm12),
        Opcode::SW(r, r, imm12),
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
        Opcode::SLDC(r, r, r),
        Opcode::SRW(r, r),
        Opcode::SRWQ(r, r),
        Opcode::SWW(r, r),
        Opcode::SWWQ(r, r),
        Opcode::TR(r, r, r),
        Opcode::TRO(r, r, r, r),
        Opcode::ECR(r, r, r),
        Opcode::K256(r, r, r),
        Opcode::S256(r, r, r),
        Opcode::XIL(r, r),
        Opcode::XIS(r, r),
        Opcode::XOL(r, r),
        Opcode::XOS(r, r),
        Opcode::XWL(r, r),
        Opcode::XWS(r, r),
        Opcode::NOOP,
        Opcode::FLAG(r),
        Opcode::Undefined,
    ];

    let mut bytes: Vec<u8> = vec![];
    let mut buffer = [0u8; 4];

    for mut op in data.clone() {
        op.read(&mut buffer)
            .expect("Failed to write opcode to buffer");
        bytes.extend(&buffer);

        let op_p = u32::from(op);
        let op_bytes = op_p.to_be_bytes().to_vec();

        let op_p = Opcode::from(op_p);
        let op_q = Opcode::from_bytes_unchecked(op_bytes.as_slice());

        assert_eq!(op, op_p);
        assert_eq!(op, op_q);

        let mut op_bytes = op.to_bytes().to_vec();

        // Assert opcode can be created from big slices
        op_bytes.extend_from_slice(&[0xff; 25]);
        while op_bytes.len() > Opcode::BYTES_SIZE {
            op_bytes.pop();

            let op_r = Opcode::from_bytes_unchecked(op_bytes.as_slice());
            let op_s = Opcode::from_bytes(op_bytes.as_slice())
                .expect("Failed to safely generate op from bytes!");

            assert_eq!(op, op_r);
            assert_eq!(op, op_s);
        }

        // Assert no panic with checked function
        while !op_bytes.is_empty() {
            op_bytes.pop();

            assert!(Opcode::from_bytes(op_bytes.as_slice()).is_err());
        }
    }

    let mut op_p = Opcode::Undefined;
    bytes.chunks(4).zip(data.iter()).for_each(|(chunk, op)| {
        op_p.write(chunk)
            .expect("Failed to parse opcode from chunk");

        assert_eq!(op, &op_p);
    });
}
