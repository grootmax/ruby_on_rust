//! x86_64 assembly dispatch loop with hardware register pinning.
//!
//! Registers pinned:
//! - `reg_pc`  -> `r14`
//! - `reg_cfp` -> `r15`
//!
//! Callee-saved register preservation:
//! `r14` and `r15` are saved on stack via `push` on entry and restored via `pop` on exit.

use core::ptr;
use crate::ffi::value::VALUE;
use super::{rb_execution_context_t, VM_INSTRUCTION_SIZE};

#[cfg(target_arch = "x86_64")]
static mut X86_64_TABLE: [*const (); VM_INSTRUCTION_SIZE] = [ptr::null(); VM_INSTRUCTION_SIZE];
#[cfg(target_arch = "x86_64")]
static mut X86_64_INIT: bool = false;

/// Get the in-memory instruction address table for x86_64.
pub unsafe fn get_insns_address_table() -> &'static [*const (); VM_INSTRUCTION_SIZE] {
    #[cfg(target_arch = "x86_64")]
    {
        if unsafe { !X86_64_INIT } {
            unsafe {
                init_x86_64_table();
                X86_64_INIT = true;
            }
        }
        unsafe { &*core::ptr::addr_of!(X86_64_TABLE) }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { super::fallback::get_insns_address_table() }
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn init_x86_64_table() {
    let table_ptr = core::ptr::addr_of_mut!(X86_64_TABLE) as *mut usize;
    // Get label addresses into table
    unsafe {
        core::arch::asm!(
            "lea {tmp}, [rip + 2000f]", "mov [{tbl} + 0], {tmp}",
            "lea {tmp}, [rip + 2001f]", "mov [{tbl} + 8], {tmp}",
            "lea {tmp}, [rip + 2002f]", "mov [{tbl} + 16], {tmp}",
            "lea {tmp}, [rip + 2003f]", "mov [{tbl} + 24], {tmp}",
            "lea {tmp}, [rip + 2004f]", "mov [{tbl} + 32], {tmp}",
            tbl = in(reg) table_ptr,
            tmp = out(reg) _,
        );
    }
    // Fill remaining opcodes with default dispatch label if unpopulated
    unsafe {
        let table_ref = &mut *core::ptr::addr_of_mut!(X86_64_TABLE);
        let default_label = table_ref[0];
        for i in 5..VM_INSTRUCTION_SIZE {
            if table_ref[i].is_null() {
                table_ref[i] = default_label;
            }
        }
    }
}

/// Execute VM core dispatch loop with assembly register pinning on x86_64.
#[cfg(target_arch = "x86_64")]
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
            // 1. Preserve ABI callee-saved registers r14 and r15
            "push r14",
            "push r15",

            // 2. Register Pinning:
            // pc  -> r14
            // cfp -> r15
            "mov r14, {pc}",
            "mov r15, {cfp}",

            // Entry dispatch block:
            "2100:", // main loop
            "test r14, r14",
            "jz 2200f", // exit if pc is NULL

            "mov rax, [r14]",
            "test rax, rax",
            "jz 2200f", // exit on NULL instruction / stop token

            // Direct-threaded computed goto jump
            "jmp rax",

            // Instruction handlers:
            "2000:", // insn 0: nop / advance
            "add r14, 8",
            "jmp 2100b",

            "2001:", // insn 1: add
            "add r14, 8",
            "jmp 2100b",

            "2002:", // insn 2: sub
            "add r14, 8",
            "jmp 2100b",

            "2003:", // insn 3: putnil
            "add r14, 8",
            "jmp 2100b",

            "2004:", // insn 4: leave
            "jmp 2200f",

            // Exit block: restore callee-saved registers and stack frame
            "2200:",
            "pop r15",
            "pop r14",

            pc = in(reg) pc,
            cfp = in(reg) cfp,
            out("rax") retval,
            clobber_abi("C"),
        );
    }

    retval
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn exec_core(ec: *mut rb_execution_context_t) -> VALUE {
    unsafe { super::fallback::exec_core(ec) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x86_64_table_init() {
        let table = unsafe { get_insns_address_table() };
        assert_eq!(table.len(), VM_INSTRUCTION_SIZE);
        #[cfg(target_arch = "x86_64")]
        assert!(!table[0].is_null());
    }

    #[test]
    fn test_null_ec_returns_table() {
        let ret = unsafe { exec_core(ptr::null_mut()) };
        let table = unsafe { get_insns_address_table() };
        assert_eq!(ret, table.as_ptr() as VALUE);
    }
}
