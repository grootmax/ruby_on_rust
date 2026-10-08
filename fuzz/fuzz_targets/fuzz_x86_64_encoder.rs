#![no_main]

use libfuzzer_sys::fuzz_target;
use zjit::asm::x86_64::*;
use zjit::asm::CodeBlock;

fuzz_target!(|data: &[u8]| {
    if data.len() < 12 {
        return;
    }

    let mut cb = CodeBlock::new_dummy();

    let reg_no1 = data[0] % 16;
    let reg_no2 = data[1] % 16;
    let bit_mode = match data[2] % 4 {
        0 => 8,
        1 => 16,
        2 => 32,
        _ => 64,
    };

    let r1 = X86Reg { num_bits: bit_mode, reg_type: RegType::GP, reg_no: reg_no1 };
    let r2 = X86Reg { num_bits: bit_mode, reg_type: RegType::GP, reg_no: reg_no2 };

    let op1 = X86Opnd::Reg(r1);
    let op2 = X86Opnd::Reg(r2);

    let disp = i32::from_le_bytes(data[3..7].try_into().unwrap());
    let mem_op = X86Opnd::Mem(X86Mem {
        num_bits: bit_mode,
        base_reg_no: reg_no1,
        idx_reg_no: if data[7] % 2 == 0 { Some(reg_no2) } else { None },
        scale_exp: data[7] % 4,
        disp,
    });

    let imm_val = i64::from_le_bytes(data[4..12].try_into().unwrap());
    let imm_op = X86Opnd::Imm(X86Imm { num_bits: bit_mode, value: imm_val });
    let uimm_op = X86Opnd::UImm(X86UImm { num_bits: bit_mode, value: imm_val as u64 });

    mov(&mut cb, op1, op2);
    mov(&mut cb, op1, mem_op);
    mov(&mut cb, op1, imm_op);
    mov(&mut cb, op1, uimm_op);

    add(&mut cb, op1, op2);
    add(&mut cb, op1, imm_op);

    sub(&mut cb, op1, op2);
    sub(&mut cb, op1, imm_op);

    push(&mut cb, op1);
    push(&mut cb, imm_op);
    pop(&mut cb, op1);

    test(&mut cb, op1, op2);
    test(&mut cb, op1, uimm_op);
});
