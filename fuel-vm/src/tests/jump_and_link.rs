use alloc::vec;

use crate::{
    prelude::*,
    tests::test_helpers::{
        assert_panics,
        assert_success,
        run_script,
    },
};
use fuel_asm::{
    op,
    RegId,
};

#[test]
fn jump_and_link__allows_discarding_return_address() {
    let script = vec![
        op::jal(RegId::ZERO, RegId::PC, 1), // Just jump to the next instruction
        op::ret(RegId::ONE),
    ];
    let receipts = run_script(script);
    assert_success(&receipts);
}

#[test]
fn jump_and_link__cannot_write_reserved_registers() {
    let script = vec![
        op::jal(RegId::ONE, RegId::PC, 1), // Just jump to the next instruction
        op::ret(RegId::ONE),
    ];
    let receipts = run_script(script);
    assert_panics(&receipts, PanicReason::ReservedRegisterNotWritable);
}

#[test]
fn jump_and_link__subroutine_call_works() {
    let reg_fn_addr = RegId::new(0x10);
    let reg_return_addr = RegId::new(0x11);
    let reg_tmp = RegId::new(0x12);

    let canary = 0x1337;

    let subroutine = vec![
        op::movi(reg_tmp, canary as _),
        op::log(reg_tmp, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::jal(RegId::ZERO, reg_return_addr, 0), // Return from the subroutine
    ];

    const MAIN_LEN: usize = 3; // Use a constant so we don't need to backpatch
    let mut script = vec![
        // Get current address so we know what location to call
        op::addi(reg_fn_addr, RegId::PC, (Instruction::SIZE * MAIN_LEN) as _),
        op::jal(reg_return_addr, reg_fn_addr, 0), // Call subroutine
        op::ret(RegId::ONE),                      // Return from the script
    ];
    assert_eq!(MAIN_LEN, script.len());
    script.extend(subroutine);

    let receipts = run_script(script);
    assert_success(&receipts);

    if let Some(Receipt::Log { ra, .. }) = receipts.first() {
        assert!(*ra == canary, "Expected canary value to be logged");
    } else {
        panic!("Expected a log receipt");
    };
}

#[test]
fn jump_and_link__immediate_count_is_instructions() {
    let reg_return_addr = RegId::new(0x11);
    let reg_tmp = RegId::new(0x12);

    let skip = 3; // Jump over the next 3 instructions

    let script = vec![
        op::movi(reg_tmp, 5),
        op::jal(reg_return_addr, RegId::PC, (skip + 1) as _), /* Zero would mean
                                                               * jumping to this
                                                               * instruction */
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::subi(reg_tmp, reg_tmp, 1),
        op::log(reg_tmp, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ];

    let receipts = run_script(script);
    assert_success(&receipts);

    if let Some(Receipt::Log { ra, .. }) = receipts.first() {
        assert_eq!(*ra, skip, "Expected correct number of skipped instructions");
    } else {
        panic!("Expected a log receipt");
    };
}

#[test]
fn jump_and_link__recursive_fibonacci() {
    /// Rust impl to test against
    fn rust_fibo(n: u64) -> u64 {
        if n <= 1 {
            n
        } else {
            rust_fibo(n - 1) + rust_fibo(n - 2)
        }
    }

    fn fuel_fibo(n: u64) -> u64 {
        // ABI: argument/return in 0x10, return address in 0x11
        let reg_fnarg = RegId::new(0x10); // Function argument and return value
        let reg_return_addr = RegId::new(0x11); // Return address

        // Local registers preserved by the callee
        let reg_local1 = RegId::new(0x12); // Local temp var
        let reg_local2 = RegId::new(0x13); // Local temp var
        let reg_local3 = RegId::new(0x14); // Local temp var, used for self fn pointer

        let script = vec![
            // Set argument
            op::movi(reg_fnarg, n as _),
            // Main function
            op::jal(reg_return_addr, RegId::PC, 3), // <- offset to subroutine
            op::log(reg_fnarg, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            op::ret(RegId::ONE),
            // Fibonacci subroutine
            // fibo(0) = 0, fibo(1) = 1, fibo(n) = fibo(n-1) + fibo(n-2)
            op::pshl(0b11110), // Save return_address and local{1,2,3}
            // Compute fn pointer to the current function, and place it in local3
            op::subi(reg_local3, RegId::PC, Instruction::SIZE as _),
            // If n < 2, no computation needed
            op::movi(reg_local1, 2),
            op::lt(reg_local1, reg_fnarg, reg_local1),
            op::jnzf(reg_local1, RegId::ZERO, 8), // Skip over computation
            // Else, call self with n - 1 and n - 2, and sum those
            op::subi(reg_local2, reg_fnarg, 2), // Save n - 2 to local2
            op::subi(reg_fnarg, reg_fnarg, 1),  // n -= 1
            op::jal(reg_return_addr, reg_local3, 0), // Call self
            op::move_(reg_local1, reg_fnarg),   // Copy result to local1
            op::move_(reg_fnarg, reg_local2),   // Restore n - 2 from local2
            op::jal(reg_return_addr, reg_local3, 0), // Call self
            op::move_(reg_local2, reg_fnarg),   // Copy result to local2
            op::add(reg_fnarg, reg_local1, reg_local2), // result = local1 + local2
            // Computation ends here, this is where jnzf jumps to
            op::popl(0b11110), // Restore return_address and local{1,2,3}
            op::jal(RegId::ZERO, reg_return_addr, 0), // Return from subroutine
        ];

        let receipts = run_script(script);
        assert_success(&receipts);
        let Some(Receipt::Log { ra, .. }) = receipts.first() else {
            panic!("Expected a log receipt");
        };
        *ra
    }

    assert_eq!(rust_fibo(10), 55, "Sanity check");

    for n in 0..=10 {
        let f = fuel_fibo(n);
        let r = rust_fibo(n);
        assert_eq!(f, r, "Wrong result for fibo({n}), got {f}, expected {r}");
    }
}
