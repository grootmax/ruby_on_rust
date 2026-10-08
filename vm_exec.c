/* -*-c-*- */
/**********************************************************************

  vm_exec.c -

  $Author$

  Copyright (C) 2004-2007 Koichi Sasada

**********************************************************************/

#include <math.h>
#include "internal/core_rs.h"
#include "insns.inc"

#if !USE_RUST_PORTS
size_t
rb_vm_exec_batch_rs(rb_execution_context_t *ec)
{
    return 0;
}
#endif

struct rb_vm_insn_opcodes_struct rb_vm_insn_opcodes = {
    .nop = BIN(nop),
    .putnil = BIN(putnil),
    .putself = BIN(putself),
    .putobject = BIN(putobject),
    .putobject_INT2FIX_0_ = BIN(putobject_INT2FIX_0_),
    .putobject_INT2FIX_1_ = BIN(putobject_INT2FIX_1_),
    .pop = BIN(pop),
    .dup = BIN(dup),
    .dupn = BIN(dupn),
    .swap = BIN(swap),
    .topn = BIN(topn),
    .getlocal = BIN(getlocal),
    .setlocal = BIN(setlocal),
    .getlocal_WC_0 = BIN(getlocal_WC_0),
    .setlocal_WC_0 = BIN(setlocal_WC_0),
    .getlocal_WC_1 = BIN(getlocal_WC_1),
    .setlocal_WC_1 = BIN(setlocal_WC_1),
    .opt_plus = BIN(opt_plus),
    .opt_minus = BIN(opt_minus),
    .opt_mult = BIN(opt_mult),
    .opt_div = BIN(opt_div),
    .opt_mod = BIN(opt_mod),
    .opt_eq = BIN(opt_eq),
    .opt_neq = BIN(opt_neq),
    .opt_lt = BIN(opt_lt),
    .opt_le = BIN(opt_le),
    .opt_gt = BIN(opt_gt),
    .opt_ge = BIN(opt_ge),
};

#if USE_YJIT || USE_ZJIT
// The number of instructions executed on vm_exec_core. --yjit-stats and --zjit-stats use this.
uint64_t rb_vm_insn_count = 0;
#endif

#if VM_COLLECT_USAGE_DETAILS
static void vm_analysis_insn(int insn);
#endif

#if VMDEBUG > 0
#define DECL_SC_REG(type, r, reg) register type reg_##r

#elif defined(__GNUC__) && defined(__x86_64__)
#define DECL_SC_REG(type, r, reg) register type reg_##r __asm__("r" reg)

#elif defined(__GNUC__) && defined(__i386__)
#define DECL_SC_REG(type, r, reg) register type reg_##r __asm__("e" reg)

#elif defined(__GNUC__) && (defined(__powerpc64__) || defined(__POWERPC__))
#define DECL_SC_REG(type, r, reg) register type reg_##r __asm__("r" reg)

#elif defined(__GNUC__) && defined(__aarch64__)
#define DECL_SC_REG(type, r, reg) register type reg_##r __asm__("x" reg)

#else
#define DECL_SC_REG(type, r, reg) register type reg_##r
#endif
/* #define DECL_SC_REG(r, reg) VALUE reg_##r */

#if !OPT_CALL_THREADED_CODE
static VALUE
vm_exec_core(rb_execution_context_t *ec)
{
#if defined(__GNUC__) && defined(__i386__)
    DECL_SC_REG(const VALUE *, pc, "di");
    DECL_SC_REG(rb_control_frame_t *, cfp, "si");

#elif defined(__GNUC__) && defined(__x86_64__)
    DECL_SC_REG(const VALUE *, pc, "14");
    DECL_SC_REG(rb_control_frame_t *, cfp, "15");

#elif defined(__GNUC__) && (defined(__powerpc64__) || defined(__POWERPC__))
    DECL_SC_REG(const VALUE *, pc, "14");
    DECL_SC_REG(rb_control_frame_t *, cfp, "15");

#elif defined(__GNUC__) && defined(__aarch64__)
    DECL_SC_REG(const VALUE *, pc, "19");
    DECL_SC_REG(rb_control_frame_t *, cfp, "20");

#else
    register rb_control_frame_t *reg_cfp;
    const VALUE *reg_pc;

#endif

#undef  RESTORE_REGS
#define RESTORE_REGS() \
{ \
  VM_REG_CFP = ec->cfp; \
  reg_pc  = reg_cfp->pc; \
}

#undef  VM_REG_PC
#define VM_REG_PC reg_pc
#undef  GET_PC
#define GET_PC() (reg_pc)
#undef  SET_PC
#define SET_PC(x) (reg_cfp->pc = VM_REG_PC = (x))

#if OPT_TOKEN_THREADED_CODE || OPT_DIRECT_THREADED_CODE
#include "vmtc.inc"
    if (UNLIKELY(ec == 0)) {
        return (VALUE)insns_address_table;
    }
#endif
    reg_cfp = ec->cfp;
    reg_pc = reg_cfp->pc;

  first:
#if USE_RUST_PORTS
    {
        reg_cfp->pc = reg_pc;
        ec->cfp = reg_cfp;
        size_t batch_count = rb_vm_exec_batch_rs(ec);
        reg_cfp = ec->cfp;
        reg_pc = reg_cfp->pc;
        if (batch_count > 0) {
#if OPT_DIRECT_THREADED_CODE
            goto *(void const *)*reg_pc;
#elif OPT_TOKEN_THREADED_CODE
            goto *insns_address_table[*reg_pc];
#else
            goto first;
#endif
        }
    }
#endif
    INSN_DISPATCH();
/*****************/
 #include "vm.inc"
/*****************/
    END_INSNS_DISPATCH();

    /* unreachable */
    rb_bug("vm_eval: unreachable");
    goto first;
}

const void **
rb_vm_get_insns_address_table(void)
{
    return (const void **)vm_exec_core(0);
}

#else /* OPT_CALL_THREADED_CODE */

#include "vm.inc"
#include "vmtc.inc"

const void **
rb_vm_get_insns_address_table(void)
{
    return (const void **)insns_address_table;
}

static VALUE
vm_exec_core(rb_execution_context_t *ec)
{
    register rb_control_frame_t *reg_cfp = ec->cfp;
    rb_thread_t *th;

    while (1) {
        reg_cfp = ((rb_insn_func_t) (*GET_PC()))(ec, reg_cfp);

        if (UNLIKELY(reg_cfp == 0)) {
            break;
        }
    }

    if (!UNDEF_P((th = rb_ec_thread_ptr(ec))->retval)) {
        VALUE ret = th->retval;
        th->retval = Qundef;
        return ret;
    }
    else {
        VALUE err = ec->errinfo;
        ec->errinfo = Qnil;
        return err;
    }
}
#endif
