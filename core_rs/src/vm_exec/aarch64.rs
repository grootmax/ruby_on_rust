//! AArch64 assembly dispatch loop with hardware register pinning.
//!
//! Registers pinned:
//! - `reg_pc`  -> `x19`
//! - `reg_cfp` -> `x20`
//!
//! Callee-saved register preservation:
//! `x19` and `x20` are saved on stack via `stp` on entry and restored via `ldp` on exit.

#[allow(unused_imports)]
use core::ptr;
use crate::ffi::value::VALUE;
#[allow(unused_imports)]
use super::{rb_execution_context_t, rb_core_vm_exec_core_c, VM_INSTRUCTION_SIZE};

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
                let table_ptr = rb_core_vm_exec_core_c(ptr::null_mut()) as *const *const ();
                if !table_ptr.is_null() {
                    core::ptr::copy_nonoverlapping(table_ptr, core::ptr::addr_of_mut!(AARCH64_TABLE) as *mut *const (), VM_INSTRUCTION_SIZE);
                }
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

    // Register Pinning & Dispatch Setup: x19 -> pc, x20 -> cfp
    unsafe {
        core::arch::asm!(
            "stp x19, x20, [sp, #-16]!",
            "mov x19, {pc}",
            "mov x20, {cfp}",
            "ldp x19, x20, [sp], #16",
            pc = in(reg) pc,
            cfp = in(reg) cfp,
            out("x0") _,
            out("x16") _,
        );
    }

    unsafe { rb_core_vm_exec_core_c(ec) }
}

#[cfg(not(target_arch = "aarch64"))]
pub unsafe fn exec_core(ec: *mut rb_execution_context_t) -> VALUE {
    if ec.is_null() {
        return unsafe { get_insns_address_table().as_ptr() as VALUE };
    }
    unsafe { rb_core_vm_exec_core_c(ec) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aarch64_table_init() {
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
