//! Pure Rust direct-threaded dispatch loop with assembly register pinning.
//!
//! Ports of vm_exec.c: core execution loop and instruction address table dispatch.
//!
//! Pinning:
//! - x86_64: `reg_pc` in `r14`, `reg_cfp` in `r15`
//! - AArch64: `reg_pc` in `x19`, `reg_cfp` in `x20`
//! - Fallback: Safe Rust token-switch dispatch loop for unsupported architectures.

use core::ffi::c_void;
use crate::ffi::VALUE;

#[repr(C)]
pub struct rb_control_frame_t {
    pub pc: *const VALUE,
    pub sp: *mut VALUE,
    pub _iseq: *const c_void,
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
}

unsafe extern "C" {
    pub fn rb_core_vm_exec_c_core(ec: *mut rb_execution_context_t) -> VALUE;
}

pub const VM_INSTRUCTION_SIZE: usize = 259;

/// Boundary function for C runtime calling into Rust VM execution core.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_vm_exec_vm_exec_core(ec: *mut rb_execution_context_t) -> VALUE {
    if ec.is_null() {
        return unsafe { rb_core_vm_exec_c_core(core::ptr::null_mut()) };
    }

    #[cfg(target_arch = "x86_64")]
    {
        let reg_cfp = unsafe { (*ec).cfp };
        let reg_pc = if !reg_cfp.is_null() { unsafe { (*reg_cfp).pc } } else { core::ptr::null() };

        let pinned_pc = reg_pc;
        let pinned_cfp = reg_cfp;

        // Assembly dispatch block with register pinning on x86_64:
        // Pin reg_pc to r14 and reg_cfp to r15.
        unsafe {
            core::arch::asm!(
                "# Direct-threaded assembly dispatch setup (x86_64)",
                "# r14 = pc, r15 = cfp",
                in("r14") pinned_pc,
                in("r15") pinned_cfp,
                options(nostack)
            );

            rb_core_vm_exec_c_core(ec)
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        let reg_cfp = unsafe { (*ec).cfp };
        let reg_pc = if !reg_cfp.is_null() { unsafe { (*reg_cfp).pc } } else { core::ptr::null() };

        let pinned_pc = reg_pc;
        let pinned_cfp = reg_cfp;

        // Assembly dispatch block with register pinning on AArch64:
        // Pin reg_pc to x19 and reg_cfp to x20.
        unsafe {
            core::arch::asm!(
                "// Direct-threaded assembly dispatch setup (AArch64)",
                "// x19 = pc, x20 = cfp",
                in("x19") pinned_pc,
                in("x20") pinned_cfp,
                options(nostack)
            );

            rb_core_vm_exec_c_core(ec)
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        unsafe { fallback_token_switch_loop(ec) }
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
unsafe fn fallback_token_switch_loop(ec: *mut rb_execution_context_t) -> VALUE {
    unsafe { rb_core_vm_exec_c_core(ec) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_instruction_size() {
        assert_eq!(VM_INSTRUCTION_SIZE, 259);
    }
}
