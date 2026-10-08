//! AArch64 assembly dispatch loop with hardware register pinning.
//!
//! Registers pinned:
//! - `reg_pc`  -> `x19`
//! - `reg_cfp` -> `x20`
//!
//! Callee-saved register preservation:
//! `x19` and `x20` are saved on stack via `stp` on entry and restored via `ldp` on exit.

use core::ptr;
use crate::ffi::value::VALUE;
use super::{rb_execution_context_t, VM_INSTRUCTION_SIZE};

#[cfg(target_arch = "aarch64")]
static mut AARCH64_TABLE: [*const (); VM_INSTRUCTION_SIZE] = [ptr::null(); VM_INSTRUCTION_SIZE];
#[cfg(target_arch = "aarch64")]
static mut AARCH64_INIT: bool = false;

/// Get the in-memory instruction address table for AArch64.
pub unsafe fn get_insns_address_table() -> &'static [*const (); VM_INSTRUCTION_SIZE] {
    #[cfg(target_arch = "aarch64")]
    {
        if unsafe { !AARCH64_INIT } {
            unsafe {
                init_aarch64_table();
                AARCH64_INIT = true;
            }
        }
        unsafe { &*core::ptr::addr_of!(AARCH64_TABLE) }
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unsafe { super::fallback::get_insns_address_table() }
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn init_aarch64_table() {
    let table_ptr = core::ptr::addr_of_mut!(AARCH64_TABLE) as *mut usize;
    // Get label addresses into table
    unsafe {
        core::arch::asm!(
            "adr {tmp}, 2000f", "str {tmp}, [{tbl}, #0]",
            "adr {tmp}, 2001f", "str {tmp}, [{tbl}, #8]",
            "adr {tmp}, 2002f", "str {tmp}, [{tbl}, #16]",
            "adr {tmp}, 2003f", "str {tmp}, [{tbl}, #24]",
            "adr {tmp}, 2004f", "str {tmp}, [{tbl}, #32]",
            tbl = in(reg) table_ptr,
            tmp = out(reg) _,
        );
    }
    // Fill remaining opcodes with default dispatch label if unpopulated
    unsafe {
        let table_ref = &mut *core::ptr::addr_of_mut!(AARCH64_TABLE);
        let default_label = table_ref[0];
        for i in 5..VM_INSTRUCTION_SIZE {
            if table_ref[i].is_null() {
                table_ref[i] = default_label;
            }
        }
    }
}

/// Execute VM core dispatch loop with assembly register pinning on AArch64.
#[cfg(target_arch = "aarch64")]
pub unsafe fn exec_core(ec: *mut rb_execution_context_t) -> VALUE {
    if ec.is_null() {
        return unsafe { get_insns_address_table().as_ptr() as VALUE };
    }

    let cfp = unsafe { (*ec).cfp };
    if cfp.is_null() {
        return 0; // Qnil
    }

    let pc = unsafe { (*cfp).pc };

    let mut retval: VALUE = 0;

    unsafe {
        core::arch::asm!(
            // 1. Preserve AAPCS64 callee-saved registers x19 and x20
            "stp x19, x20, [sp, #-16]!",

            // 2. Register Pinning:
            // pc  -> x19
            // cfp -> x20
            "mov x19, {pc}",
            "mov x20, {cfp}",

            // Entry dispatch block:
            "2100:", // main loop
            "cbz x19, 2200f", // exit if pc is NULL

            "ldr x16, [x19]",
            "cbz x16, 2200f", // exit on NULL instruction / stop token

            // Direct-threaded computed goto branch
            "br x16",

            // Instruction handlers:
            "2000:", // insn 0: nop / advance
            "add x19, x19, #8",
            "b 2100b",

            "2001:", // insn 1: add
            "add x19, x19, #8",
            "b 2100b",

            "2002:", // insn 2: sub
            "add x19, x19, #8",
            "b 2100b",

            "2003:", // insn 3: putnil
            "add x19, x19, #8",
            "b 2100b",

            "2004:", // insn 4: leave
            "b 2200f",

            // Exit block: restore AAPCS64 callee-saved registers and stack frame
            "2200:",
            "ldp x19, x20, [sp], #16",

            pc = in(reg) pc,
            cfp = in(reg) cfp,
            out("x0") retval,
            clobber_abi("C"),
        );
    }

    retval
}

#[cfg(not(target_arch = "aarch64"))]
pub unsafe fn exec_core(ec: *mut rb_execution_context_t) -> VALUE {
    unsafe { super::fallback::exec_core(ec) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aarch64_table_init() {
        let table = unsafe { get_insns_address_table() };
        assert_eq!(table.len(), VM_INSTRUCTION_SIZE);
        #[cfg(target_arch = "aarch64")]
        assert!(!table[0].is_null());
    }

    #[test]
    fn test_null_ec_returns_table() {
        let ret = unsafe { exec_core(ptr::null_mut()) };
        let table = unsafe { get_insns_address_table() };
        assert_eq!(ret, table.as_ptr() as VALUE);
    }
}
