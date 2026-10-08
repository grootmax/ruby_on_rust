//! Pure Rust direct-threaded VM execution loop with assembly register pinning.
//!
//! Replicates GCC computed-goto and explicit hardware register binding:
//! - On x86_64: `reg_pc` in `r14`, `reg_cfp` in `r15`.
//! - On AArch64: `reg_pc` in `x19`, `reg_cfp` in `x20`.
//! - On other targets: Safe Rust token-switch dispatch loop.

pub mod aarch64;
pub mod fallback;
pub mod x86_64;

use core::ffi::c_void;
use crate::ffi::value::VALUE;

/// Total number of instructions in Ruby VM instruction sequence table.
pub const VM_INSTRUCTION_SIZE: usize = 259;

/// Control frame structure representation matching CRuby layout (`vm_core.h`).
#[repr(C)]
pub struct rb_control_frame_t {
    pub pc: *const VALUE,        // cfp[0]
    pub sp: *mut VALUE,          // cfp[1]
    pub iseq: *const c_void,     // cfp[2]
    pub self_val: VALUE,         // cfp[3]
    pub ep: *const VALUE,        // cfp[4]
    pub block_code: *const c_void, // cfp[5]
    pub jit_return: *mut c_void, // cfp[6]
}

/// Execution context structure representation matching CRuby layout (`vm_core.h`).
#[repr(C)]
pub struct rb_execution_context_t {
    pub vm_stack: *mut VALUE,
    pub vm_stack_size: usize,
    pub cfp: *mut rb_control_frame_t,
}

/// Get the instruction address table for current architecture.
pub unsafe fn get_insns_address_table() -> &'static [*const (); VM_INSTRUCTION_SIZE] {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { x86_64::get_insns_address_table() }
    }
    #[cfg(target_arch = "aarch64")]
    {
        unsafe { aarch64::get_insns_address_table() }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        unsafe { fallback::get_insns_address_table() }
    }
}

/// Boundary C-compatible entrypoint function for VM core execution loop.
///
/// If `ec` is NULL, returns pointer to the instruction address table (`insns_address_table`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_vm_exec_core_rs(ec: *mut rb_execution_context_t) -> VALUE {
    if ec.is_null() {
        return unsafe { get_insns_address_table().as_ptr() as VALUE };
    }

    #[cfg(target_arch = "x86_64")]
    {
        unsafe { x86_64::exec_core(ec) }
    }
    #[cfg(target_arch = "aarch64")]
    {
        unsafe { aarch64::exec_core(ec) }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        unsafe { fallback::exec_core(ec) }
    }
}

/// Boundary C-compatible function returning instruction address table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_vm_get_insns_address_table_rs() -> *const *const () {
    unsafe { get_insns_address_table().as_ptr() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;

    #[test]
    fn test_null_ec_returns_instruction_table() {
        let ret = unsafe { rb_vm_exec_core_rs(ptr::null_mut()) };
        let table = unsafe { rb_vm_get_insns_address_table_rs() };
        assert_eq!(ret, table as VALUE);
    }
}
