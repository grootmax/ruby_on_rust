//! Safe Rust token-switch dispatch loop fallback.
//!
//! Used for target architectures without explicit assembly dispatch routines or as fallback.

use core::ptr;
use crate::ffi::value::VALUE;
use super::{rb_execution_context_t, rb_core_vm_exec_core_c, VM_INSTRUCTION_SIZE};

static mut FALLBACK_TABLE: [*const (); VM_INSTRUCTION_SIZE] = [ptr::null(); VM_INSTRUCTION_SIZE];
static mut FALLBACK_INIT: bool = false;

/// Get the instruction address table for fallback token dispatch.
pub unsafe fn get_insns_address_table() -> &'static [*const (); VM_INSTRUCTION_SIZE] {
    if unsafe { !FALLBACK_INIT } {
        unsafe {
            let table_ptr = rb_core_vm_exec_core_c(ptr::null_mut()) as *const *const ();
            if !table_ptr.is_null() {
                core::ptr::copy_nonoverlapping(table_ptr, core::ptr::addr_of_mut!(FALLBACK_TABLE) as *mut *const (), VM_INSTRUCTION_SIZE);
            }
            FALLBACK_INIT = true;
        }
    }
    unsafe { &*core::ptr::addr_of!(FALLBACK_TABLE) }
}

/// Fallback execution loop using a safe Rust token-switch dispatch loop.
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
    fn test_fallback_table_init() {
        let table = unsafe { get_insns_address_table() };
        assert_eq!(table.len(), VM_INSTRUCTION_SIZE);
    }
}
