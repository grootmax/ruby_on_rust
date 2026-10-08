#![cfg(test)]

use crate::asm::{arm64, x86_64, CodeBlock};
use crate::bitset::BitSet;
use crate::backend::parcopy::{self, RegisterCopy};

#[test]
fn test_arm64_encoders_invalid_bit_patterns_graceful() {
    let mut cb = CodeBlock::new_dummy();

    // Test ADD / ADDS / SUB / SUBS with various invalid bit patterns & registers
    arm64::add(&mut cb, arm64::X0, arm64::X1, arm64::X2);
    arm64::add(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(u64::MAX)); // invalid uimm12
    arm64::add(&mut cb, arm64::W0, arm64::W1, arm64::A64Opnd::new_imm(-999999));  // out of range imm

    arm64::adds(&mut cb, arm64::X0, arm64::X1, arm64::X2);
    arm64::adds(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(u64::MAX));

    arm64::sub(&mut cb, arm64::X0, arm64::X1, arm64::X2);
    arm64::sub(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(99999));

    arm64::subs(&mut cb, arm64::X0, arm64::X1, arm64::X2);

    // Logical instructions with arbitrary / invalid bitmask immediates
    arm64::and(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(0)); // 0 is invalid logical bitmask
    arm64::and(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(u64::MAX)); // u64::MAX is invalid bitmask
    arm64::and(&mut cb, arm64::W0, arm64::W1, arm64::A64Opnd::new_uimm(0b101)); // non-replicable bitmask

    arm64::ands(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(0x123456789));

    arm64::eor(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(0));
    arm64::orr(&mut cb, arm64::X0, arm64::X1, arm64::A64Opnd::new_uimm(0));

    // Branch / test bit
    arm64::b(&mut cb, arm64::InstructionOffset::from_bytes(0));
    arm64::bl(&mut cb, arm64::InstructionOffset::from_bytes(4));
    arm64::cbz(&mut cb, arm64::X0, arm64::InstructionOffset::from_bytes(8));
    arm64::cbnz(&mut cb, arm64::W1, arm64::InstructionOffset::from_bytes(-4));

    // Ensure CodeBlock collected encoded bytes without crashing or UB
    assert!(!cb.hexdump().is_empty());
}

#[test]
fn test_x86_64_encoders_pure_rust() {
    let mut cb = CodeBlock::new_dummy();

    // Encode basic x86_64 instructions
    x86_64::mov(&mut cb, x86_64::RAX, x86_64::RBX);
    x86_64::add(&mut cb, x86_64::RAX, x86_64::RCX);
    x86_64::sub(&mut cb, x86_64::RDX, x86_64::RSI);
    x86_64::push(&mut cb, x86_64::RBP);
    x86_64::pop(&mut cb, x86_64::RBP);
    x86_64::ret(&mut cb);

    assert!(!cb.hexdump().is_empty());
}

#[test]
fn test_bitset_pure_rust() {
    let mut bs: BitSet<usize> = BitSet::with_capacity(128);

    assert!(!bs.get(0));
    assert!(bs.insert(5));
    assert!(bs.get(5));
    assert!(!bs.insert(5)); // already inserted

    assert!(bs.remove(5));
    assert!(!bs.get(5));

    // Test out of bounds access gracefully returning false
    assert!(!bs.get(9999));
    assert!(!bs.insert(9999));
    assert!(!bs.remove(9999));

    let mut bs1: BitSet<usize> = BitSet::with_capacity(64);
    let mut bs2: BitSet<usize> = BitSet::with_capacity(64);

    bs1.insert(10);
    bs1.insert(20);
    bs2.insert(20);
    bs2.insert(30);

    bs1.intersect_with(&bs2);
    assert!(!bs1.get(10));
    assert!(bs1.get(20));
    assert!(!bs1.get(30));
}

#[test]
fn test_parcopy_pure_rust() {
    let copies = vec![
        RegisterCopy { source: 1u32, destination: 2u32 },
        RegisterCopy { source: 2u32, destination: 3u32 },
    ];
    let spare = 99u32;

    let seq = parcopy::sequentialize_register(&copies, spare);
    assert!(!seq.is_empty());
}
