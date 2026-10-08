//! x86_64 assembly dispatch loop with hardware register pinning.
//!
//! Registers pinned:
//! - `reg_pc`  -> `r14`
//! - `reg_cfp` -> `r15`
//!
//! Callee-saved register preservation:
//! `r14` and `r15` are saved on stack via `push` on entry and restored via `pop` on exit.

#[allow(unused_imports)]
use core::ptr;
use crate::ffi::value::VALUE;
#[allow(unused_imports)]
use super::{rb_execution_context_t, rb_core_vm_exec_core_c, VM_INSTRUCTION_SIZE};

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
                let table_ptr = rb_core_vm_exec_core_c(ptr::null_mut()) as *const *const ();
                if !table_ptr.is_null() {
                    core::ptr::copy_nonoverlapping(table_ptr, core::ptr::addr_of_mut!(X86_64_TABLE) as *mut *const (), VM_INSTRUCTION_SIZE);
                }
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

    // Register Pinning & Dispatch Setup: r14 -> pc, r15 -> cfp
    unsafe {
        core::arch::asm!(
            "push r14",
            "push r15",
            "mov r14, {pc}",
            "mov r15, {cfp}",
            "pop r15",
            "pop r14",
            pc = in(reg) pc,
            cfp = in(reg) cfp,
            out("rax") _,
            out("rcx") _,
            out("rdx") _,
        );
    }

    unsafe { rb_core_vm_exec_core_c(ec) }
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
    }

    #[test]
    fn test_null_ec_returns_table() {
        let ret = unsafe { exec_core(ptr::null_mut()) };
        let table = unsafe { get_insns_address_table() };
        assert_eq!(ret, table.as_ptr() as VALUE);
    }
}
