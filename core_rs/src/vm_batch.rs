//! Chunked instruction batching kernels for basic-block bytecode execution.

#![allow(non_snake_case, unused_imports)]

use core::ffi::c_int;
use crate::ffi::value::{
    FIXNUM_P, NEGFIXABLE, POSFIXABLE, Qfalse, Qnil, Qtrue, VALUE,
};

/// Control frame mirror matching `rb_control_frame_t`.
#[repr(C)]
pub struct RbControlFrame {
    pub pc: *const VALUE,
    pub sp: *mut VALUE,
    pub iseq: *const usize,
    pub self_val: VALUE,
    pub ep: *const VALUE,
    pub block_code: *const usize,
    pub jit_return: *mut usize,
}

/// Execution context mirror matching `rb_execution_context_t`.
#[repr(C)]
pub struct RbExecutionContext {
    pub vm_stack: *mut VALUE,
    pub vm_stack_size: usize,
    pub cfp: *mut RbControlFrame,
}

/// Instruction opcode table mapping instruction IDs from C.
#[repr(C)]
pub struct RbVmInsnOpcodes {
    pub nop: c_int,
    pub putnil: c_int,
    pub putself: c_int,
    pub putobject: c_int,
    pub putobject_INT2FIX_0_: c_int,
    pub putobject_INT2FIX_1_: c_int,
    pub pop: c_int,
    pub dup: c_int,
    pub dupn: c_int,
    pub swap: c_int,
    pub topn: c_int,
    pub getlocal: c_int,
    pub setlocal: c_int,
    pub getlocal_WC_0: c_int,
    pub setlocal_WC_0: c_int,
    pub getlocal_WC_1: c_int,
    pub setlocal_WC_1: c_int,
    pub opt_plus: c_int,
    pub opt_minus: c_int,
    pub opt_mult: c_int,
    pub opt_div: c_int,
    pub opt_mod: c_int,
    pub opt_eq: c_int,
    pub opt_neq: c_int,
    pub opt_lt: c_int,
    pub opt_le: c_int,
    pub opt_gt: c_int,
    pub opt_ge: c_int,
}

unsafe extern "C" {
    pub static rb_vm_insn_opcodes: RbVmInsnOpcodes;
    pub fn rb_vm_insn_decode(encoded: VALUE) -> c_int;
}

/// Processes a contiguous sequence of basic-block instructions in Rust
/// without returning to C for every instruction dispatch.
/// Synchronizes `pc` and `sp` back to `cfp` before returning.
/// Returns the number of instructions executed in the batch (0 on fallback or side exit).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_vm_exec_batch_rs(ec: *mut RbExecutionContext) -> usize {
    if ec.is_null() {
        return 0;
    }
    let cfp = unsafe { (*ec).cfp };
    if cfp.is_null() {
        return 0;
    }

    let mut pc = unsafe { (*cfp).pc };
    let mut sp = unsafe { (*cfp).sp };
    if pc.is_null() || sp.is_null() {
        return 0;
    }

    let mut executed_count: usize = 0;

    unsafe {
        let op = &rb_vm_insn_opcodes;

        loop {
            let encoded_insn = *pc;
            let insn_id = rb_vm_insn_decode(encoded_insn);

            if insn_id == op.nop {
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.putnil {
                *sp = Qnil;
                sp = sp.add(1);
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.putself {
                let self_val = (*cfp).self_val;
                *sp = self_val;
                sp = sp.add(1);
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.putobject {
                let val = *pc.add(1);
                *sp = val;
                sp = sp.add(1);
                pc = pc.add(2);
                executed_count += 1;
            } else if insn_id == op.putobject_INT2FIX_0_ {
                *sp = 1; // FIXNUM(0)
                sp = sp.add(1);
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.putobject_INT2FIX_1_ {
                *sp = 3; // FIXNUM(1)
                sp = sp.add(1);
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.pop {
                sp = sp.sub(1);
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.dup {
                let val = *sp.sub(1);
                *sp = val;
                sp = sp.add(1);
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.swap {
                let top = *sp.sub(1);
                let sub = *sp.sub(2);
                *sp.sub(1) = sub;
                *sp.sub(2) = top;
                pc = pc.add(1);
                executed_count += 1;
            } else if insn_id == op.getlocal_WC_0 {
                let idx = *pc.add(1);
                let ep = (*cfp).ep;
                if ep.is_null() {
                    break;
                }
                let val = *ep.sub(idx as usize);
                *sp = val;
                sp = sp.add(1);
                pc = pc.add(2);
                executed_count += 1;
            } else if insn_id == op.setlocal_WC_0 {
                let idx = *pc.add(1);
                let ep = (*cfp).ep;
                if ep.is_null() {
                    break;
                }
                let val = *sp.sub(1);
                sp = sp.sub(1);
                let slot = (ep as *mut VALUE).sub(idx as usize);
                *slot = val;
                pc = pc.add(2);
                executed_count += 1;
            } else if insn_id == op.opt_plus {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let r_val = (recv as isize) >> 1;
                    let o_val = (obj as isize) >> 1;
                    if let Some(res) = r_val.checked_add(o_val) {
                        if POSFIXABLE(res as _) && NEGFIXABLE(res as _) {
                            let val = ((res as usize) << 1) | 1;
                            *sp.sub(2) = val;
                            sp = sp.sub(1);
                            pc = pc.add(2);
                            executed_count += 1;
                            continue;
                        }
                    }
                }
                break;
            } else if insn_id == op.opt_minus {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let r_val = (recv as isize) >> 1;
                    let o_val = (obj as isize) >> 1;
                    if let Some(res) = r_val.checked_sub(o_val) {
                        if POSFIXABLE(res as _) && NEGFIXABLE(res as _) {
                            let val = ((res as usize) << 1) | 1;
                            *sp.sub(2) = val;
                            sp = sp.sub(1);
                            pc = pc.add(2);
                            executed_count += 1;
                            continue;
                        }
                    }
                }
                break;
            } else if insn_id == op.opt_mult {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let r_val = (recv as isize) >> 1;
                    let o_val = (obj as isize) >> 1;
                    if let Some(res) = r_val.checked_mul(o_val) {
                        if POSFIXABLE(res as _) && NEGFIXABLE(res as _) {
                            let val = ((res as usize) << 1) | 1;
                            *sp.sub(2) = val;
                            sp = sp.sub(1);
                            pc = pc.add(2);
                            executed_count += 1;
                            continue;
                        }
                    }
                }
                break;
            } else if insn_id == op.opt_eq {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let res = recv == obj;
                    *sp.sub(2) = if res { Qtrue } else { Qfalse };
                    sp = sp.sub(1);
                    pc = pc.add(2);
                    executed_count += 1;
                } else if recv == obj {
                    *sp.sub(2) = Qtrue;
                    sp = sp.sub(1);
                    pc = pc.add(2);
                    executed_count += 1;
                } else {
                    break;
                }
            } else if insn_id == op.opt_lt {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let res = (recv as isize) < (obj as isize);
                    *sp.sub(2) = if res { Qtrue } else { Qfalse };
                    sp = sp.sub(1);
                    pc = pc.add(2);
                    executed_count += 1;
                } else {
                    break;
                }
            } else if insn_id == op.opt_le {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let res = (recv as isize) <= (obj as isize);
                    *sp.sub(2) = if res { Qtrue } else { Qfalse };
                    sp = sp.sub(1);
                    pc = pc.add(2);
                    executed_count += 1;
                } else {
                    break;
                }
            } else if insn_id == op.opt_gt {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let res = (recv as isize) > (obj as isize);
                    *sp.sub(2) = if res { Qtrue } else { Qfalse };
                    sp = sp.sub(1);
                    pc = pc.add(2);
                    executed_count += 1;
                } else {
                    break;
                }
            } else if insn_id == op.opt_ge {
                let recv = *sp.sub(2);
                let obj = *sp.sub(1);
                if FIXNUM_P(recv) && FIXNUM_P(obj) {
                    let res = (recv as isize) >= (obj as isize);
                    *sp.sub(2) = if res { Qtrue } else { Qfalse };
                    sp = sp.sub(1);
                    pc = pc.add(2);
                    executed_count += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if executed_count > 0 {
            (*cfp).pc = pc;
            (*cfp).sp = sp;
        }
    }

    executed_count
}
