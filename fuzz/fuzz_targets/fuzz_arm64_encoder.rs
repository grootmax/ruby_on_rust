#![no_main]

use libfuzzer_sys::fuzz_target;
use zjit::asm::{arm64, CodeBlock};

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    let mut cb = CodeBlock::new_dummy();

    let r0 = data[0] % 32;
    let r1 = data[1] % 32;
    let r2 = data[2] % 32;
    let num_bits = if data[3] % 2 == 0 { 32 } else { 64 };

    let reg0 = arm64::A64Reg { num_bits, reg_no: r0 };
    let reg1 = arm64::A64Reg { num_bits, reg_no: r1 };
    let reg2 = arm64::A64Reg { num_bits, reg_no: r2 };

    let imm_val = i64::from_le_bytes(data[4..12].try_into().unwrap());
    let uimm_val = u64::from_le_bytes(data[8..16].try_into().unwrap());

    let opnd_reg = arm64::A64Opnd::Reg(reg2);
    let opnd_imm = arm64::A64Opnd::Imm(imm_val);
    let opnd_uimm = arm64::A64Opnd::UImm(uimm_val);

    // Test encoders with arbitrary operands
    arm64::add(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_reg);
    arm64::add(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_imm);
    arm64::add(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_uimm);

    arm64::adds(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_reg);
    arm64::adds(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_uimm);

    arm64::sub(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_reg);
    arm64::sub(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_imm);

    arm64::subs(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_reg);

    arm64::and(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_uimm);
    arm64::ands(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_uimm);
    arm64::eor(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_uimm);
    arm64::orr(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::A64Opnd::Reg(reg1), opnd_uimm);

    let offset_val = imm_val as i32;
    arm64::b(&mut cb, arm64::InstructionOffset::from_bytes(offset_val));
    arm64::bl(&mut cb, arm64::InstructionOffset::from_bytes(offset_val));
    arm64::cbz(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::InstructionOffset::from_bytes(offset_val));
    arm64::cbnz(&mut cb, arm64::A64Opnd::Reg(reg0), arm64::InstructionOffset::from_bytes(offset_val));
});
