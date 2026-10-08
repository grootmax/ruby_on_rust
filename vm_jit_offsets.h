#ifndef VM_JIT_OFFSETS_H
#define VM_JIT_OFFSETS_H

#include <stddef.h>

/* Fallback macro for C11 _Static_assert if compiler does not provide it */
#ifndef _Static_assert
# if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
/* Supported natively in C11 */
# elif defined(__GNUC__) && (__GNUC__ > 4 || (__GNUC__ == 4 && __GNUC_MINOR__ >= 6))
/* Supported natively in GCC 4.6+ */
# else
#  define _STATIC_ASSERT_CONCAT_IMPL(a, b) a##b
#  define _STATIC_ASSERT_CONCAT(a, b) _STATIC_ASSERT_CONCAT_IMPL(a, b)
#  define _Static_assert(expr, msg) \
     typedef char _STATIC_ASSERT_CONCAT(static_assert_failed_, __LINE__)[(expr) ? 1 : -1]
# endif
#endif

/* Field offsets for RBasic struct */
#define RUBY_OFFSET_RBASIC_FLAGS offsetof(struct RBasic, flags)
#define RUBY_OFFSET_RBASIC_KLASS offsetof(struct RBasic, klass)

/* Field offsets for RObject struct */
#define ROBJECT_OFFSET_AS_HEAP_FIELDS offsetof(struct RObject, as.extended)
#define ROBJECT_OFFSET_AS_ARY offsetof(struct RObject, as.ary)

/* Field offset for prime classext's fields_obj from a class pointer */
#define RCLASS_OFFSET_PRIME_FIELDS_OBJ offsetof(struct RClass_and_rb_classext_t, classext.fields_obj)

/* Field offset for fields_obj in T_DATA */
#define TDATA_OFFSET_FIELDS_OBJ offsetof(struct RTypedData, fields_obj)

/* Field offsets for RHash struct */
#define RUBY_OFFSET_RHASH_IFNONE offsetof(struct RHash, ifnone)
#define RUBY_OFFSET_RHASH_AR_HINT (sizeof(struct RHash) + offsetof(ar_table, ar_hint))
#define RUBY_OFFSET_RHASH_AR_PAIRS (sizeof(struct RHash) + offsetof(ar_table, pairs))
#define RUBY_RHASH_AR_TABLE_MAX_SIZE RHASH_AR_TABLE_MAX_SIZE

/* Field offsets for RString struct */
#define RUBY_OFFSET_RSTRING_LEN offsetof(struct RString, len)
#define RUBY_OFFSET_RSTRING_AS_HEAP_PTR offsetof(struct RString, as.heap.ptr)
#define RUBY_OFFSET_RSTRING_AS_ARY offsetof(struct RString, as.embed.ary)

/* Shape constant related to RBasic::flags */
#define RB_SHAPE_FLAG_SHIFT SHAPE_FLAG_SHIFT

/* Field offsets for rb_execution_context_t */
#define RUBY_OFFSET_EC_CFP offsetof(rb_execution_context_t, cfp)
#define RUBY_OFFSET_EC_INTERRUPT_FLAG offsetof(rb_execution_context_t, interrupt_flag)
#define RUBY_OFFSET_EC_INTERRUPT_MASK offsetof(rb_execution_context_t, interrupt_mask)
#define RUBY_OFFSET_EC_THREAD_PTR offsetof(rb_execution_context_t, thread_ptr)
#define RUBY_OFFSET_EC_RACTOR_ID offsetof(rb_execution_context_t, ractor_id)

/* Field offsets for RArray struct */
#define RUBY_OFFSET_RARRAY_AS_HEAP_LEN offsetof(struct RArray, as.heap.len)
#define RUBY_OFFSET_RARRAY_AS_HEAP_PTR offsetof(struct RArray, as.heap.ptr)
#define RUBY_OFFSET_RARRAY_AS_ARY offsetof(struct RArray, as.ary)

/* Field offsets for RStruct struct */
#define RUBY_OFFSET_RSTRUCT_AS_HEAP_PTR offsetof(struct RStruct, as.heap.ptr)
#define RUBY_OFFSET_RSTRUCT_FIELDS_OBJ offsetof(struct RStruct, fields_obj)
#define RUBY_OFFSET_RSTRUCT_AS_ARY offsetof(struct RStruct, as.ary)

/* Field offsets for rb_control_frame_t (struct rb_control_frame_struct) */
#define RUBY_OFFSET_CFP_PC offsetof(rb_control_frame_t, pc)
#define RUBY_OFFSET_CFP_SP offsetof(rb_control_frame_t, sp)
#define RUBY_OFFSET_CFP_ISEQ offsetof(rb_control_frame_t, _iseq)
#define RUBY_OFFSET_CFP_SELF offsetof(rb_control_frame_t, self)
#define RUBY_OFFSET_CFP_EP offsetof(rb_control_frame_t, ep)
#define RUBY_OFFSET_CFP_BLOCK_CODE offsetof(rb_control_frame_t, block_code)
#define RUBY_OFFSET_CFP_JIT_RETURN offsetof(rb_control_frame_t, jit_return)
#define RUBY_SIZEOF_CONTROL_FRAME sizeof(rb_control_frame_t)

/* Field offsets for rb_thread_t (struct rb_thread_struct) */
#define RUBY_OFFSET_THREAD_SELF offsetof(rb_thread_t, self)

/* Field offsets for iseq_inline_constant_cache and entry */
#define RUBY_OFFSET_IC_ENTRY offsetof(struct iseq_inline_constant_cache, entry)
#define RUBY_OFFSET_ICE_VALUE offsetof(struct iseq_inline_constant_cache_entry, value)

#endif /* VM_JIT_OFFSETS_H */
