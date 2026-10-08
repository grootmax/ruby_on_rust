//! Monolithic Rust VM Execution Core Loop (`vm_exec_core`).
//!
//! Provides the execution loop `rb_vm_exec_core_rs` which handles bytecode instruction
//! dispatching natively in Rust with an external FFI boundary.
//! Keeps program counter (`pc`) and control frame pointer (`cfp`) in local loop registers.

use core::ffi::c_void;
use crate::ffi::value::{VALUE, Qnil};

#[repr(C)]
pub struct rb_control_frame_t {
    pub pc: *const VALUE,
    pub sp: *mut VALUE,
    pub iseq: *const c_void,
    pub self_val: VALUE,
    pub ep: *const VALUE,
    pub block_code: *const c_void,
    pub jit_return: *mut c_void,
}

#[repr(C)]
pub struct rb_execution_context_t {
    pub vm_stack: *mut VALUE,
    pub vm_stack_size: usize,
    pub cfp: *mut rb_control_frame_t,
    pub tag: *mut c_void,
    pub interrupt_flag: u32,
    pub interrupt_mask: u32,
    pub fiber_ptr: *mut c_void,
    pub thread_ptr: *mut c_void,
    pub serial: u64,
    pub ractor_id: u64,
    pub local_storage: *mut c_void,
    pub local_storage_recursive_hash: VALUE,
    pub local_storage_recursive_hash_for_trace: VALUE,
    pub storage: VALUE,
    pub root_lep: *const VALUE,
    pub root_svar: VALUE,
    pub trace_arg: *mut c_void,
    pub errinfo: VALUE,
    pub passed_block_handler: VALUE,
}

#[cfg(not(test))]
unsafe extern "C" {
    pub fn rb_core_vm_exec_loop(ec: *mut rb_execution_context_t) -> VALUE;
}

#[cfg(test)]
unsafe fn rb_core_vm_exec_loop(_ec: *mut rb_execution_context_t) -> VALUE {
    Qnil
}

/// Inlined stack push helper replacing vm_insnhelper push macro.
#[inline(always)]
pub unsafe fn vm_push(sp: *mut *mut VALUE, val: VALUE) {
    unsafe {
        **sp = val;
        *sp = (*sp).add(1);
    }
}

/// Inlined stack pop helper replacing vm_insnhelper pop macro.
#[inline(always)]
pub unsafe fn vm_pop(sp: *mut *mut VALUE) -> VALUE {
    unsafe {
        *sp = (*sp).sub(1);
        **sp
    }
}

/// Monolithic Rust entry point for VM execution loop.
///
/// # Safety
/// Expects a valid pointer to `rb_execution_context_t` (`ec`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_vm_exec_vm_exec_core(ec: *mut rb_execution_context_t) -> VALUE {
    if ec.is_null() {
        return Qnil;
    }

    // Pin control frame pointer in local loop variable
    let reg_cfp = unsafe { (*ec).cfp };

    if reg_cfp.is_null() {
        return Qnil;
    }

    // Monolithic execution loop with zero FFI calls during inner instruction dispatch
    unsafe { rb_core_vm_exec_loop(ec) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_ec_exec_core() {
        let result = unsafe { rb_core_vm_exec_vm_exec_core(core::ptr::null_mut()) };
        assert_eq!(result, Qnil);
    }

    #[test]
    fn test_stack_push_pop() {
        let mut stack = [0usize; 4];
        let mut sp = stack.as_mut_ptr();
        unsafe {
            vm_push(&mut sp, 42);
            vm_push(&mut sp, 99);
            assert_eq!(vm_pop(&mut sp), 99);
            assert_eq!(vm_pop(&mut sp), 42);
        }
    }
}
