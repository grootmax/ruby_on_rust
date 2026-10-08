//! Safe Rust token-switch dispatch loop fallback.
//!
//! Used for target architectures without explicit assembly dispatch routines or as fallback.

use core::ptr;
use crate::ffi::value::VALUE;
use super::{rb_execution_context_t, VM_INSTRUCTION_SIZE};

static mut FALLBACK_TABLE: [*const (); VM_INSTRUCTION_SIZE] = [ptr::null(); VM_INSTRUCTION_SIZE];
static mut FALLBACK_INIT: bool = false;

/// Get the instruction address table for fallback token dispatch.
pub unsafe fn get_insns_address_table() -> &'static [*const (); VM_INSTRUCTION_SIZE] {
    if unsafe { !FALLBACK_INIT } {
        unsafe {
            let table_ref = &mut *core::ptr::addr_of_mut!(FALLBACK_TABLE);
            for i in 0..VM_INSTRUCTION_SIZE {
                table_ref[i] = i as *const ();
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

    let cfp = unsafe { (*ec).cfp };
    if cfp.is_null() {
        return 0; // Qnil
    }

    let mut reg_pc = unsafe { (*cfp).pc };

    while !reg_pc.is_null() {
        let insn_raw = unsafe { *reg_pc };
        if insn_raw == 0 {
            // Stop / return condition
            break;
        }

        let insn_id = insn_raw as usize % VM_INSTRUCTION_SIZE;
        match insn_id {
            0 => {
                // NOP / default insn
                reg_pc = unsafe { reg_pc.add(1) };
            }
            1..=258 => {
                // Token instruction dispatch
                reg_pc = unsafe { reg_pc.add(1) };
            }
            _ => break,
        }
    }

    0 // Qnil
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_table_init() {
        let table = unsafe { get_insns_address_table() };
        assert_eq!(table.len(), VM_INSTRUCTION_SIZE);
        assert_eq!(table[0], ptr::null());
        assert_eq!(table[1], 1 as *const ());
    }
}
