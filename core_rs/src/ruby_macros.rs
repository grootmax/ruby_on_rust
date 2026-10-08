//! Pure Rust inline helper functions for CRuby VALUE operations and macro reimplementations.
//!
//! Reimplements CRuby header macros and static inline functions (`include/ruby/internal/`)
//! with pure Rust `#[inline(always)]` functions, struct layouts (`RBasic`, `RString`, `RArray`,
//! `RObject`, `RFloat`, `RComplex`, `RRational`, `RBignum`), bitfield accessors, and
//! the `ValueHelpers` trait for method syntax on `VALUE` (e.g. `val.nil_p()`, `val.fixnum_p()`).

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

use core::ffi::{c_char, c_double, c_int, c_long, c_ulong};
use crate::ffi::value::{
    FIXNUM_FLAG, FIXNUM_MAX, FIXNUM_MIN, FLONUM_FLAG, FLONUM_MASK, IMMEDIATE_MASK, Qfalse,
    Qnil, Qtrue, Qundef, SIGNED_VALUE, SPECIAL_SHIFT, SYMBOL_FLAG, USE_FLONUM, VALUE,
};

/// C-level type of a Ruby object (`enum ruby_value_type`).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ruby_value_type {
    RUBY_T_NONE     = 0x00,
    RUBY_T_OBJECT   = 0x01,
    RUBY_T_CLASS    = 0x02,
    RUBY_T_MODULE   = 0x03,
    RUBY_T_FLOAT    = 0x04,
    RUBY_T_STRING   = 0x05,
    RUBY_T_REGEXP   = 0x06,
    RUBY_T_ARRAY    = 0x07,
    RUBY_T_HASH     = 0x08,
    RUBY_T_STRUCT   = 0x09,
    RUBY_T_BIGNUM   = 0x0a,
    RUBY_T_FILE     = 0x0b,
    RUBY_T_DATA     = 0x0c,
    RUBY_T_MATCH    = 0x0d,
    RUBY_T_COMPLEX  = 0x0e,
    RUBY_T_RATIONAL = 0x0f,

    RUBY_T_NIL      = 0x11,
    RUBY_T_TRUE     = 0x12,
    RUBY_T_FALSE    = 0x13,
    RUBY_T_SYMBOL   = 0x14,
    RUBY_T_FIXNUM   = 0x15,
    RUBY_T_UNDEF    = 0x16,

    RUBY_T_IMEMO    = 0x1a,
    RUBY_T_NODE     = 0x1b,
    RUBY_T_ICLASS   = 0x1c,
    RUBY_T_ZOMBIE   = 0x1d,
    RUBY_T_MOVED    = 0x1e,

    RUBY_T_MASK     = 0x1f,
}

// Bitfield & Flag Constants matching CRuby fl_type.h, rstring.h, rarray.h, robject.h, bignum.h
pub const RUBY_FL_USHIFT: u32 = 12;

pub const RUBY_FL_WB_PROTECTED: VALUE = 1 << 5;
pub const RUBY_FL_PROMOTED: VALUE = 1 << 5;
pub const RUBY_FL_FINALIZE: VALUE = 1 << 7;
pub const RUBY_FL_SHAREABLE: VALUE = 1 << 8;
pub const RUBY_FL_FREEZE: VALUE = 1 << 11;

pub const RUBY_FL_USER0: VALUE = 1 << (RUBY_FL_USHIFT + 0);
pub const RUBY_FL_USER1: VALUE = 1 << (RUBY_FL_USHIFT + 1);
pub const RUBY_FL_USER2: VALUE = 1 << (RUBY_FL_USHIFT + 2);
pub const RUBY_FL_USER3: VALUE = 1 << (RUBY_FL_USHIFT + 3);
pub const RUBY_FL_USER4: VALUE = 1 << (RUBY_FL_USHIFT + 4);
pub const RUBY_FL_USER5: VALUE = 1 << (RUBY_FL_USHIFT + 5);
pub const RUBY_FL_USER6: VALUE = 1 << (RUBY_FL_USHIFT + 6);
pub const RUBY_FL_USER7: VALUE = 1 << (RUBY_FL_USHIFT + 7);
pub const RUBY_FL_USER8: VALUE = 1 << (RUBY_FL_USHIFT + 8);
pub const RUBY_FL_USER9: VALUE = 1 << (RUBY_FL_USHIFT + 9);
pub const RUBY_FL_USER10: VALUE = 1 << (RUBY_FL_USHIFT + 10);
pub const RUBY_FL_USER11: VALUE = 1 << (RUBY_FL_USHIFT + 11);
pub const RUBY_FL_USER12: VALUE = 1 << (RUBY_FL_USHIFT + 12);
pub const RUBY_FL_USER13: VALUE = 1 << (RUBY_FL_USHIFT + 13);
pub const RUBY_FL_USER14: VALUE = 1 << (RUBY_FL_USHIFT + 14);
pub const RUBY_FL_USER15: VALUE = 1 << (RUBY_FL_USHIFT + 15);
pub const RUBY_FL_USER16: VALUE = 1 << (RUBY_FL_USHIFT + 16);
pub const RUBY_FL_USER17: VALUE = 1 << (RUBY_FL_USHIFT + 17);
pub const RUBY_FL_USER18: VALUE = 1 << (RUBY_FL_USHIFT + 18);
pub const RUBY_FL_USER19: VALUE = 1 << (RUBY_FL_USHIFT + 19);

pub const RUBY_ELTS_SHARED: VALUE = RUBY_FL_USER0;
pub const RSTRING_NOEMBED: VALUE = RUBY_FL_USER1;
pub const RSTRING_FSTR: VALUE = RUBY_FL_USER17;

pub const RARRAY_EMBED_FLAG: VALUE = RUBY_FL_USER1;
pub const RARRAY_EMBED_LEN_MASK: VALUE = RUBY_FL_USER9
    | RUBY_FL_USER8
    | RUBY_FL_USER7
    | RUBY_FL_USER6
    | RUBY_FL_USER5
    | RUBY_FL_USER4
    | RUBY_FL_USER3;
pub const RARRAY_EMBED_LEN_SHIFT: u32 = RUBY_FL_USHIFT + 3;

pub const ROBJECT_EMBED: VALUE = RUBY_FL_USER1;

pub const BIGNUM_SIGN_BIT: VALUE = RUBY_FL_USER1;
pub const BIGNUM_EMBED_FLAG: VALUE = RUBY_FL_USER2;
pub const BIGNUM_EMBED_LEN_MASK: VALUE = RUBY_FL_USER11
    | RUBY_FL_USER10
    | RUBY_FL_USER9
    | RUBY_FL_USER8
    | RUBY_FL_USER7
    | RUBY_FL_USER6
    | RUBY_FL_USER5
    | RUBY_FL_USER4
    | RUBY_FL_USER3;
pub const BIGNUM_EMBED_LEN_SHIFT: u32 = RUBY_FL_USHIFT + 3;

// -----------------------------------------------------------------------------
// Struct Layout Definitions (`#[repr(C)]`) matching C header definitions
// -----------------------------------------------------------------------------

/// Base header shared by all heap-allocated Ruby objects (`struct RBasic`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RBasic {
    pub flags: VALUE,
    pub klass: VALUE,
}

/// Heap storage structure for `RString`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RStringHeap {
    pub ptr: *mut c_char,
    pub aux: RStringHeapAux,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union RStringHeapAux {
    pub capa: c_long,
    pub shared: VALUE,
}

/// Union holding heap vs embedded content for `RString`.
#[repr(C)]
#[derive(Copy, Clone)]
pub union RStringAs {
    pub heap: RStringHeap,
    pub ary: [c_char; 1],
}

/// CRuby's `RString` layout (`struct RString`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RString {
    pub basic: RBasic,
    pub len: c_long,
    pub as_: RStringAs,
}

/// Heap storage structure for `RArray`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RArrayHeap {
    pub len: c_long,
    pub aux: RArrayHeapAux,
    pub ptr: *const VALUE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union RArrayHeapAux {
    pub capa: c_long,
    pub shared_root: VALUE,
}

/// Union holding heap vs embedded elements for `RArray`.
#[repr(C)]
#[derive(Copy, Clone)]
pub union RArrayAs {
    pub heap: RArrayHeap,
    pub ary: [VALUE; 1],
}

/// CRuby's `RArray` layout (`struct RArray`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RArray {
    pub basic: RBasic,
    pub as_: RArrayAs,
}

/// Union for `RObject`.
#[repr(C)]
#[derive(Copy, Clone)]
pub union RObjectAs {
    pub ary: [VALUE; 1],
    pub extended: VALUE,
}

/// CRuby's `RObject` layout (`struct RObject`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RObject {
    pub basic: RBasic,
    pub as_: RObjectAs,
}

/// CRuby's `RFloat` layout (`struct RFloat`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RFloat {
    pub basic: RBasic,
    pub float_value: c_double,
}

/// CRuby's `RComplex` layout (`struct RComplex`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RComplex {
    pub basic: RBasic,
    pub real: VALUE,
    pub imag: VALUE,
}

/// CRuby's `RRational` layout (`struct RRational`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RRational {
    pub basic: RBasic,
    pub num: VALUE,
    pub den: VALUE,
}

/// Heap storage for `RBignum`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RBignumHeap {
    pub len: usize,
    pub digits: *mut u32,
}

/// Union for `RBignum`.
#[repr(C)]
#[derive(Copy, Clone)]
pub union RBignumAs {
    pub heap: RBignumHeap,
    pub ary: [u32; 1],
}

/// CRuby's `RBignum` layout (`struct RBignum`).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RBignum {
    pub basic: RBasic,
    pub as_: RBignumAs,
}

// -----------------------------------------------------------------------------
// Pure Rust `#[inline(always)]` Predicates and Helper Functions
// -----------------------------------------------------------------------------

/// `RB_TEST()` / `RTEST()`: false only for `Qfalse` and `Qnil`.
#[inline(always)]
pub const fn rtest(obj: VALUE) -> bool {
    obj & !Qnil != 0
}

/// `RB_NIL_P()` / `NIL_P()`.
#[inline(always)]
pub const fn nil_p(obj: VALUE) -> bool {
    obj == Qnil
}

/// `RB_UNDEF_P()` / `UNDEF_P()`.
#[inline(always)]
pub const fn undef_p(obj: VALUE) -> bool {
    obj == Qundef
}

/// `RB_NIL_OR_UNDEF_P()` / `NIL_OR_UNDEF_P()`.
#[inline(always)]
pub const fn nil_or_undef_p(obj: VALUE) -> bool {
    let mask: VALUE = !(Qundef ^ Qnil);
    let common_bits: VALUE = Qundef & Qnil;
    (obj & mask) == common_bits
}

/// `RB_FIXNUM_P()` / `FIXNUM_P()`.
#[inline(always)]
pub const fn fixnum_p(obj: VALUE) -> bool {
    obj & FIXNUM_FLAG != 0
}

/// `RB_STATIC_SYM_P()` / `STATIC_SYM_P()`.
#[inline(always)]
pub const fn static_sym_p(obj: VALUE) -> bool {
    let mask: VALUE = !(VALUE::MAX << SPECIAL_SHIFT);
    (obj & mask) == SYMBOL_FLAG
}

/// `RB_FLONUM_P()` / `FLONUM_P()`.
#[inline(always)]
pub const fn flonum_p(obj: VALUE) -> bool {
    USE_FLONUM && (obj & FLONUM_MASK) == FLONUM_FLAG
}

/// `RB_IMMEDIATE_P()` / `IMMEDIATE_P()`.
#[inline(always)]
pub const fn immediate_p(obj: VALUE) -> bool {
    obj & IMMEDIATE_MASK != 0
}

/// `RB_SPECIAL_CONST_P()` / `SPECIAL_CONST_P()`.
#[inline(always)]
pub const fn special_const_p(obj: VALUE) -> bool {
    obj == Qfalse || immediate_p(obj)
}

/// `RB_BUILTIN_TYPE()`.
#[inline(always)]
pub unsafe fn builtin_type(obj: VALUE) -> ruby_value_type {
    let basic = rbasic(obj);
    let flags = unsafe { (*basic).flags };
    let t = (flags & (ruby_value_type::RUBY_T_MASK as VALUE)) as u32;
    unsafe { core::mem::transmute(t) }
}

/// `rb_type()`.
#[inline(always)]
pub unsafe fn rb_type(obj: VALUE) -> ruby_value_type {
    if !special_const_p(obj) {
        unsafe { builtin_type(obj) }
    } else if obj == Qfalse {
        ruby_value_type::RUBY_T_FALSE
    } else if obj == Qnil {
        ruby_value_type::RUBY_T_NIL
    } else if obj == Qtrue {
        ruby_value_type::RUBY_T_TRUE
    } else if obj == Qundef {
        ruby_value_type::RUBY_T_UNDEF
    } else if fixnum_p(obj) {
        ruby_value_type::RUBY_T_FIXNUM
    } else if static_sym_p(obj) {
        ruby_value_type::RUBY_T_SYMBOL
    } else {
        ruby_value_type::RUBY_T_FLOAT
    }
}

/// `RB_FLOAT_TYPE_P()`.
#[inline(always)]
pub unsafe fn float_type_p(obj: VALUE) -> bool {
    if flonum_p(obj) {
        true
    } else if special_const_p(obj) {
        false
    } else {
        unsafe { builtin_type(obj) == ruby_value_type::RUBY_T_FLOAT }
    }
}

/// `RB_DYNAMIC_SYM_P()`.
#[inline(always)]
pub unsafe fn dynamic_sym_p(obj: VALUE) -> bool {
    if special_const_p(obj) {
        false
    } else {
        unsafe { builtin_type(obj) == ruby_value_type::RUBY_T_SYMBOL }
    }
}

/// `RB_SYMBOL_P()`.
#[inline(always)]
pub unsafe fn symbol_p(obj: VALUE) -> bool {
    static_sym_p(obj) || unsafe { dynamic_sym_p(obj) }
}

/// `RB_INTEGER_TYPE_P()` / `rb_integer_type_p()`.
#[inline(always)]
pub unsafe fn integer_type_p(obj: VALUE) -> bool {
    if fixnum_p(obj) {
        true
    } else if special_const_p(obj) {
        false
    } else {
        unsafe { builtin_type(obj) == ruby_value_type::RUBY_T_BIGNUM }
    }
}

/// `RB_TYPE_P()`.
#[inline(always)]
pub unsafe fn type_p(obj: VALUE, t: ruby_value_type) -> bool {
    match t {
        ruby_value_type::RUBY_T_TRUE => obj == Qtrue,
        ruby_value_type::RUBY_T_FALSE => obj == Qfalse,
        ruby_value_type::RUBY_T_NIL => obj == Qnil,
        ruby_value_type::RUBY_T_UNDEF => obj == Qundef,
        ruby_value_type::RUBY_T_FIXNUM => fixnum_p(obj),
        ruby_value_type::RUBY_T_SYMBOL => unsafe { symbol_p(obj) },
        ruby_value_type::RUBY_T_FLOAT => unsafe { float_type_p(obj) },
        _ => {
            if special_const_p(obj) {
                false
            } else {
                unsafe { builtin_type(obj) == t }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Fixnum & Float Conversion Helpers
// -----------------------------------------------------------------------------

/// `RB_POSFIXABLE()`.
#[inline(always)]
pub const fn posfixable(v: c_long) -> bool {
    v <= FIXNUM_MAX
}

/// `RB_NEGFIXABLE()`.
#[inline(always)]
pub const fn negfixable(v: c_long) -> bool {
    v >= FIXNUM_MIN
}

/// `RB_FIXABLE()`.
#[inline(always)]
pub const fn fixable(v: c_long) -> bool {
    posfixable(v) && negfixable(v)
}

/// `INT2FIX()` / `LONG2FIX()`.
#[inline(always)]
pub const fn int2fix(i: c_long) -> VALUE {
    let j = i as usize;
    (j << 1).wrapping_add(FIXNUM_FLAG)
}

/// `FIX2LONG()`.
#[inline(always)]
pub const fn fix2long(x: VALUE) -> c_long {
    ((x as SIGNED_VALUE) >> 1) as c_long
}

/// `FIX2ULONG()`.
#[inline(always)]
pub const fn fix2ulong(x: VALUE) -> c_ulong {
    fix2long(x) as c_ulong
}

/// `RBOOL()`.
#[inline(always)]
pub const fn rbool(b: bool) -> VALUE {
    if b { Qtrue } else { Qfalse }
}

/// `rb_float_flonum_value()`: decodes flonum bit layout into 64-bit float.
#[inline(always)]
pub fn rb_float_flonum_value(v: VALUE) -> c_double {
    if USE_FLONUM {
        if v != 0x8000_0000_0000_0002 {
            let b63 = v >> 63;
            let val = (2 - b63) | (v & !0x03);
            let rot = val.rotate_right(3);
            c_double::from_bits(rot as u64)
        } else {
            0.0
        }
    } else {
        0.0
    }
}

/// `RFLOAT_VALUE()` / `rb_float_value()`.
#[inline(always)]
pub unsafe fn rfloat_value(obj: VALUE) -> c_double {
    if flonum_p(obj) {
        rb_float_flonum_value(obj)
    } else {
        let flt = obj as *const RFloat;
        unsafe { (*flt).float_value }
    }
}

// -----------------------------------------------------------------------------
// Flags Operations (`RB_FL_*`)
// -----------------------------------------------------------------------------

/// `RB_FL_ABLE()` / `FL_ABLE()`.
#[inline(always)]
pub const fn fl_able(obj: VALUE) -> bool {
    !special_const_p(obj)
}

/// `RB_FL_TEST_RAW()`.
#[inline(always)]
pub unsafe fn fl_test_raw(obj: VALUE, flags: VALUE) -> VALUE {
    let basic = rbasic(obj);
    unsafe { (*basic).flags & flags }
}

/// `RB_FL_TEST()` / `FL_TEST()`.
#[inline(always)]
pub unsafe fn fl_test(obj: VALUE, flags: VALUE) -> VALUE {
    if fl_able(obj) {
        unsafe { fl_test_raw(obj, flags) }
    } else {
        0
    }
}

/// `RB_FL_ANY_RAW()`.
#[inline(always)]
pub unsafe fn fl_any_raw(obj: VALUE, flags: VALUE) -> bool {
    unsafe { fl_test_raw(obj, flags) != 0 }
}

/// `RB_FL_ANY()` / `FL_ANY()`.
#[inline(always)]
pub unsafe fn fl_any(obj: VALUE, flags: VALUE) -> bool {
    unsafe { fl_test(obj, flags) != 0 }
}

/// `RB_FL_ALL_RAW()`.
#[inline(always)]
pub unsafe fn fl_all_raw(obj: VALUE, flags: VALUE) -> bool {
    unsafe { fl_test_raw(obj, flags) == flags }
}

/// `RB_FL_ALL()` / `FL_ALL()`.
#[inline(always)]
pub unsafe fn fl_all(obj: VALUE, flags: VALUE) -> bool {
    unsafe { fl_test(obj, flags) == flags }
}

/// `RB_FL_SET_RAW()`.
#[inline(always)]
pub unsafe fn fl_set_raw(obj: VALUE, flags: VALUE) {
    let basic = rbasic_mut(obj);
    unsafe {
        (*basic).flags |= flags;
    }
}

/// `RB_FL_SET()` / `FL_SET()`.
#[inline(always)]
pub unsafe fn fl_set(obj: VALUE, flags: VALUE) {
    if fl_able(obj) {
        unsafe { fl_set_raw(obj, flags) }
    }
}

/// `RB_FL_UNSET_RAW()`.
#[inline(always)]
pub unsafe fn fl_unset_raw(obj: VALUE, flags: VALUE) {
    let basic = rbasic_mut(obj);
    unsafe {
        (*basic).flags &= !flags;
    }
}

/// `RB_FL_UNSET()` / `FL_UNSET()`.
#[inline(always)]
pub unsafe fn fl_unset(obj: VALUE, flags: VALUE) {
    if fl_able(obj) {
        unsafe { fl_unset_raw(obj, flags) }
    }
}

/// `RB_FL_REVERSE_RAW()`.
#[inline(always)]
pub unsafe fn fl_reverse_raw(obj: VALUE, flags: VALUE) {
    let basic = rbasic_mut(obj);
    unsafe {
        (*basic).flags ^= flags;
    }
}

/// `RB_FL_REVERSE()` / `FL_REVERSE()`.
#[inline(always)]
pub unsafe fn fl_reverse(obj: VALUE, flags: VALUE) {
    if fl_able(obj) {
        unsafe { fl_reverse_raw(obj, flags) }
    }
}

/// `RB_OBJ_FROZEN_RAW()`.
#[inline(always)]
pub unsafe fn obj_frozen_raw(obj: VALUE) -> VALUE {
    unsafe { fl_test_raw(obj, RUBY_FL_FREEZE) }
}

/// `RB_OBJ_FROZEN()` / `OBJ_FROZEN()`.
#[inline(always)]
pub unsafe fn obj_frozen(obj: VALUE) -> bool {
    if !fl_able(obj) {
        true
    } else {
        unsafe { obj_frozen_raw(obj) != 0 }
    }
}

// -----------------------------------------------------------------------------
// Object Pointer & Accessor Helpers
// -----------------------------------------------------------------------------

/// Casts `VALUE` to `*const RBasic`.
#[inline(always)]
pub const fn rbasic(obj: VALUE) -> *const RBasic {
    obj as *const RBasic
}

/// Casts `VALUE` to `*mut RBasic`.
#[inline(always)]
pub const fn rbasic_mut(obj: VALUE) -> *mut RBasic {
    obj as *mut RBasic
}

/// `RBASIC_CLASS()`.
#[inline(always)]
pub unsafe fn rbasic_class(obj: VALUE) -> VALUE {
    unsafe { (*rbasic(obj)).klass }
}

/// Casts `VALUE` to `*const RString`.
#[inline(always)]
pub const fn rstring(obj: VALUE) -> *const RString {
    obj as *const RString
}

/// Casts `VALUE` to `*mut RString`.
#[inline(always)]
pub const fn rstring_mut(obj: VALUE) -> *mut RString {
    obj as *mut RString
}

/// `RSTRING_LEN()`.
#[inline(always)]
pub unsafe fn rstring_len(str_val: VALUE) -> c_long {
    let str_ptr = rstring(str_val);
    unsafe { (*str_ptr).len }
}

/// `RSTRING_LENINT()`.
#[inline(always)]
pub unsafe fn rstring_lenint(str_val: VALUE) -> c_int {
    unsafe { rstring_len(str_val) as c_int }
}

/// `RSTRING_PTR()`: returns raw `*mut c_char` pointing to string bytes.
#[inline(always)]
pub unsafe fn rstring_ptr(str_val: VALUE) -> *mut c_char {
    let str_ptr = rstring_mut(str_val);
    if unsafe { fl_test_raw(str_val, RSTRING_NOEMBED) != 0 } {
        unsafe { (*str_ptr).as_.heap.ptr }
    } else {
        unsafe { (*str_ptr).as_.ary.as_mut_ptr() }
    }
}

/// Const variant of `RSTRING_PTR()`.
#[inline(always)]
pub unsafe fn rstring_const_ptr(str_val: VALUE) -> *const c_char {
    unsafe { rstring_ptr(str_val) as *const c_char }
}

/// `RSTRING_END()`.
#[inline(always)]
pub unsafe fn rstring_end(str_val: VALUE) -> *mut c_char {
    let ptr = unsafe { rstring_ptr(str_val) };
    let len = unsafe { rstring_len(str_val) };
    unsafe { ptr.offset(len as isize) }
}

/// Returns Ruby string contents as a safe byte slice `&[u8]`.
#[inline(always)]
pub unsafe fn rstring_as_bytes<'a>(str_val: VALUE) -> &'a [u8] {
    let ptr = unsafe { rstring_const_ptr(str_val) } as *const u8;
    let len = unsafe { rstring_len(str_val) } as usize;
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }
}

/// Casts `VALUE` to `*const RArray`.
#[inline(always)]
pub const fn rarray(obj: VALUE) -> *const RArray {
    obj as *const RArray
}

/// Casts `VALUE` to `*mut RArray`.
#[inline(always)]
pub const fn rarray_mut(obj: VALUE) -> *mut RArray {
    obj as *mut RArray
}

/// `RARRAY_EMBED_LEN()`.
#[inline(always)]
pub unsafe fn rarray_embed_len(ary_val: VALUE) -> c_long {
    let flags = unsafe { (*rbasic(ary_val)).flags };
    let f = (flags & RARRAY_EMBED_LEN_MASK) >> RARRAY_EMBED_LEN_SHIFT;
    f as c_long
}

/// `RARRAY_LEN()` / `rb_array_len()`.
#[inline(always)]
pub unsafe fn rarray_len(ary_val: VALUE) -> c_long {
    if unsafe { fl_test_raw(ary_val, RARRAY_EMBED_FLAG) != 0 } {
        unsafe { rarray_embed_len(ary_val) }
    } else {
        let ary_ptr = rarray(ary_val);
        unsafe { (*ary_ptr).as_.heap.len }
    }
}

/// `RARRAY_LENINT()`.
#[inline(always)]
pub unsafe fn rarray_lenint(ary_val: VALUE) -> c_int {
    unsafe { rarray_len(ary_val) as c_int }
}

/// `RARRAY_CONST_PTR()` / `rb_array_const_ptr()`.
#[inline(always)]
pub unsafe fn rarray_const_ptr(ary_val: VALUE) -> *const VALUE {
    if unsafe { fl_test_raw(ary_val, RARRAY_EMBED_FLAG) != 0 } {
        let ary_ptr = rarray(ary_val);
        unsafe { (*ary_ptr).as_.ary.as_ptr() }
    } else {
        let ary_ptr = rarray(ary_val);
        unsafe { (*ary_ptr).as_.heap.ptr }
    }
}

/// `RARRAY_PTR()`.
#[inline(always)]
pub unsafe fn rarray_ptr(ary_val: VALUE) -> *mut VALUE {
    unsafe { rarray_const_ptr(ary_val) as *mut VALUE }
}

/// `RARRAY_AREF()`.
#[inline(always)]
pub unsafe fn rarray_aref(ary_val: VALUE, i: c_long) -> VALUE {
    let ptr = unsafe { rarray_const_ptr(ary_val) };
    unsafe { *ptr.offset(i as isize) }
}

/// Returns Ruby array elements as a safe slice `&[VALUE]`.
#[inline(always)]
pub unsafe fn rarray_as_slice<'a>(ary_val: VALUE) -> &'a [VALUE] {
    let ptr = unsafe { rarray_const_ptr(ary_val) };
    let len = unsafe { rarray_len(ary_val) } as usize;
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }
}

/// Casts `VALUE` to `*const RObject`.
#[inline(always)]
pub const fn robject(obj: VALUE) -> *const RObject {
    obj as *const RObject
}

/// Casts `VALUE` to `*mut RObject`.
#[inline(always)]
pub const fn robject_mut(obj: VALUE) -> *mut RObject {
    obj as *mut RObject
}

/// Casts `VALUE` to `*const RComplex`.
#[inline(always)]
pub const fn rcomplex(obj: VALUE) -> *const RComplex {
    obj as *const RComplex
}

/// Casts `VALUE` to `*const RRational`.
#[inline(always)]
pub const fn rrational(obj: VALUE) -> *const RRational {
    obj as *const RRational
}

/// `RBIGNUM_SIGN()` / `BIGNUM_SIGN_BIT`.
#[inline(always)]
pub unsafe fn rbignum_sign_p(obj: VALUE) -> bool {
    unsafe { fl_test_raw(obj, BIGNUM_SIGN_BIT) != 0 }
}

/// `RBIGNUM_POSITIVE_P()`.
#[inline(always)]
pub unsafe fn rbignum_positive_p(obj: VALUE) -> bool {
    unsafe { rbignum_sign_p(obj) }
}

/// `RBIGNUM_NEGATIVE_P()`.
#[inline(always)]
pub unsafe fn rbignum_negative_p(obj: VALUE) -> bool {
    unsafe { !rbignum_sign_p(obj) }
}

// -----------------------------------------------------------------------------
// Trait `ValueHelpers` on `VALUE` for Ergonomic Method Syntax
// -----------------------------------------------------------------------------

/// Extension trait providing CRuby object inspection methods directly on `VALUE`.
pub trait ValueHelpers {
    fn rtest(self) -> bool;
    fn nil_p(self) -> bool;
    fn undef_p(self) -> bool;
    fn nil_or_undef_p(self) -> bool;
    fn fixnum_p(self) -> bool;
    fn static_sym_p(self) -> bool;
    fn dynamic_sym_p(self) -> bool;
    fn symbol_p(self) -> bool;
    fn flonum_p(self) -> bool;
    fn immediate_p(self) -> bool;
    fn special_const_p(self) -> bool;
    fn float_type_p(self) -> bool;
    fn integer_type_p(self) -> bool;
    fn builtin_type(self) -> ruby_value_type;
    fn rb_type(self) -> ruby_value_type;
    fn type_p(self, t: ruby_value_type) -> bool;
    fn fix2long(self) -> c_long;
    fn fix2ulong(self) -> c_ulong;
    fn fl_test(self, flags: VALUE) -> VALUE;
    fn fl_any(self, flags: VALUE) -> bool;
    fn fl_all(self, flags: VALUE) -> bool;
    fn frozen_p(self) -> bool;
    fn rbasic(self) -> *const RBasic;
    fn rbasic_mut(self) -> *mut RBasic;
    fn rstring_len(self) -> c_long;
    fn rstring_ptr(self) -> *mut c_char;
    fn rstring_const_ptr(self) -> *const c_char;
    fn rstring_slice<'a>(self) -> &'a [u8];
    fn rarray_len(self) -> c_long;
    fn rarray_const_ptr(self) -> *const VALUE;
    fn rarray_ptr(self) -> *mut VALUE;
    fn rarray_aref(self, i: c_long) -> VALUE;
    fn rarray_slice<'a>(self) -> &'a [VALUE];
    fn rfloat_value(self) -> c_double;
}

impl ValueHelpers for VALUE {
    #[inline(always)]
    fn rtest(self) -> bool {
        rtest(self)
    }

    #[inline(always)]
    fn nil_p(self) -> bool {
        nil_p(self)
    }

    #[inline(always)]
    fn undef_p(self) -> bool {
        undef_p(self)
    }

    #[inline(always)]
    fn nil_or_undef_p(self) -> bool {
        nil_or_undef_p(self)
    }

    #[inline(always)]
    fn fixnum_p(self) -> bool {
        fixnum_p(self)
    }

    #[inline(always)]
    fn static_sym_p(self) -> bool {
        static_sym_p(self)
    }

    #[inline(always)]
    fn dynamic_sym_p(self) -> bool {
        unsafe { dynamic_sym_p(self) }
    }

    #[inline(always)]
    fn symbol_p(self) -> bool {
        unsafe { symbol_p(self) }
    }

    #[inline(always)]
    fn flonum_p(self) -> bool {
        flonum_p(self)
    }

    #[inline(always)]
    fn immediate_p(self) -> bool {
        immediate_p(self)
    }

    #[inline(always)]
    fn special_const_p(self) -> bool {
        special_const_p(self)
    }

    #[inline(always)]
    fn float_type_p(self) -> bool {
        unsafe { float_type_p(self) }
    }

    #[inline(always)]
    fn integer_type_p(self) -> bool {
        unsafe { integer_type_p(self) }
    }

    #[inline(always)]
    fn builtin_type(self) -> ruby_value_type {
        unsafe { builtin_type(self) }
    }

    #[inline(always)]
    fn rb_type(self) -> ruby_value_type {
        unsafe { rb_type(self) }
    }

    #[inline(always)]
    fn type_p(self, t: ruby_value_type) -> bool {
        unsafe { type_p(self, t) }
    }

    #[inline(always)]
    fn fix2long(self) -> c_long {
        fix2long(self)
    }

    #[inline(always)]
    fn fix2ulong(self) -> c_ulong {
        fix2ulong(self)
    }

    #[inline(always)]
    fn fl_test(self, flags: VALUE) -> VALUE {
        unsafe { fl_test(self, flags) }
    }

    #[inline(always)]
    fn fl_any(self, flags: VALUE) -> bool {
        unsafe { fl_any(self, flags) }
    }

    #[inline(always)]
    fn fl_all(self, flags: VALUE) -> bool {
        unsafe { fl_all(self, flags) }
    }

    #[inline(always)]
    fn frozen_p(self) -> bool {
        unsafe { obj_frozen(self) }
    }

    #[inline(always)]
    fn rbasic(self) -> *const RBasic {
        rbasic(self)
    }

    #[inline(always)]
    fn rbasic_mut(self) -> *mut RBasic {
        rbasic_mut(self)
    }

    #[inline(always)]
    fn rstring_len(self) -> c_long {
        unsafe { rstring_len(self) }
    }

    #[inline(always)]
    fn rstring_ptr(self) -> *mut c_char {
        unsafe { rstring_ptr(self) }
    }

    #[inline(always)]
    fn rstring_const_ptr(self) -> *const c_char {
        unsafe { rstring_const_ptr(self) }
    }

    #[inline(always)]
    fn rstring_slice<'a>(self) -> &'a [u8] {
        unsafe { rstring_as_bytes(self) }
    }

    #[inline(always)]
    fn rarray_len(self) -> c_long {
        unsafe { rarray_len(self) }
    }

    #[inline(always)]
    fn rarray_const_ptr(self) -> *const VALUE {
        unsafe { rarray_const_ptr(self) }
    }

    #[inline(always)]
    fn rarray_ptr(self) -> *mut VALUE {
        unsafe { rarray_ptr(self) }
    }

    #[inline(always)]
    fn rarray_aref(self, i: c_long) -> VALUE {
        unsafe { rarray_aref(self, i) }
    }

    #[inline(always)]
    fn rarray_slice<'a>(self) -> &'a [VALUE] {
        unsafe { rarray_as_slice(self) }
    }

    #[inline(always)]
    fn rfloat_value(self) -> c_double {
        unsafe { rfloat_value(self) }
    }
}

// -----------------------------------------------------------------------------
// Requirement 4: Compile-time Assertions for Bitmasks & Layout Alignments
// -----------------------------------------------------------------------------

const _: () = {
    // Check primitive widths
    assert!(core::mem::size_of::<VALUE>() == core::mem::size_of::<c_long>());

    // Special constants
    if USE_FLONUM {
        assert!(Qfalse == 0x00);
        assert!(Qnil == 0x04);
        assert!(Qtrue == 0x14);
        assert!(Qundef == 0x24);
        assert!(IMMEDIATE_MASK == 0x07);
        assert!(FIXNUM_FLAG == 0x01);
        assert!(FLONUM_MASK == 0x03);
        assert!(FLONUM_FLAG == 0x02);
        assert!(SYMBOL_FLAG == 0x0c);
    } else {
        assert!(Qfalse == 0x00);
        assert!(Qnil == 0x02);
        assert!(Qtrue == 0x06);
        assert!(Qundef == 0x0a);
        assert!(IMMEDIATE_MASK == 0x03);
        assert!(FIXNUM_FLAG == 0x01);
        assert!(FLONUM_MASK == 0x00);
        assert!(FLONUM_FLAG == 0x02);
        assert!(SYMBOL_FLAG == 0x0e);
    }

    // ruby_value_type ABI constants
    assert!(ruby_value_type::RUBY_T_NONE as u32 == 0x00);
    assert!(ruby_value_type::RUBY_T_OBJECT as u32 == 0x01);
    assert!(ruby_value_type::RUBY_T_CLASS as u32 == 0x02);
    assert!(ruby_value_type::RUBY_T_MODULE as u32 == 0x03);
    assert!(ruby_value_type::RUBY_T_FLOAT as u32 == 0x04);
    assert!(ruby_value_type::RUBY_T_STRING as u32 == 0x05);
    assert!(ruby_value_type::RUBY_T_REGEXP as u32 == 0x06);
    assert!(ruby_value_type::RUBY_T_ARRAY as u32 == 0x07);
    assert!(ruby_value_type::RUBY_T_HASH as u32 == 0x08);
    assert!(ruby_value_type::RUBY_T_STRUCT as u32 == 0x09);
    assert!(ruby_value_type::RUBY_T_BIGNUM as u32 == 0x0a);
    assert!(ruby_value_type::RUBY_T_FILE as u32 == 0x0b);
    assert!(ruby_value_type::RUBY_T_DATA as u32 == 0x0c);
    assert!(ruby_value_type::RUBY_T_MATCH as u32 == 0x0d);
    assert!(ruby_value_type::RUBY_T_COMPLEX as u32 == 0x0e);
    assert!(ruby_value_type::RUBY_T_RATIONAL as u32 == 0x0f);
    assert!(ruby_value_type::RUBY_T_NIL as u32 == 0x11);
    assert!(ruby_value_type::RUBY_T_TRUE as u32 == 0x12);
    assert!(ruby_value_type::RUBY_T_FALSE as u32 == 0x13);
    assert!(ruby_value_type::RUBY_T_SYMBOL as u32 == 0x14);
    assert!(ruby_value_type::RUBY_T_FIXNUM as u32 == 0x15);
    assert!(ruby_value_type::RUBY_T_UNDEF as u32 == 0x16);
    assert!(ruby_value_type::RUBY_T_MASK as u32 == 0x1f);

    // FL Flags and Shifts
    assert!(RUBY_FL_USHIFT == 12);
    assert!(RUBY_FL_FREEZE == (1 << 11));
    assert!(RUBY_FL_SHAREABLE == (1 << 8));
    assert!(RSTRING_NOEMBED == (1 << 13));
    assert!(RARRAY_EMBED_FLAG == (1 << 13));
    assert!(RARRAY_EMBED_LEN_SHIFT == 15);

    // Struct Size and Field Offset ABI Alignment
    assert!(core::mem::size_of::<RBasic>() == 2 * core::mem::size_of::<VALUE>());
    assert!(core::mem::offset_of!(RBasic, flags) == 0);
    assert!(core::mem::offset_of!(RBasic, klass) == core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RString, basic) == 0);
    assert!(core::mem::offset_of!(RString, len) == 2 * core::mem::size_of::<VALUE>());
    assert!(core::mem::offset_of!(RString, as_) == 3 * core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RArray, basic) == 0);
    assert!(core::mem::offset_of!(RArray, as_) == 2 * core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RObject, basic) == 0);
    assert!(core::mem::offset_of!(RObject, as_) == 2 * core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RFloat, basic) == 0);
    assert!(core::mem::offset_of!(RFloat, float_value) == 2 * core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RComplex, basic) == 0);
    assert!(core::mem::offset_of!(RComplex, real) == 2 * core::mem::size_of::<VALUE>());
    assert!(core::mem::offset_of!(RComplex, imag) == 3 * core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RRational, basic) == 0);
    assert!(core::mem::offset_of!(RRational, num) == 2 * core::mem::size_of::<VALUE>());
    assert!(core::mem::offset_of!(RRational, den) == 3 * core::mem::size_of::<VALUE>());

    assert!(core::mem::offset_of!(RBignum, basic) == 0);
    assert!(core::mem::offset_of!(RBignum, as_) == 2 * core::mem::size_of::<VALUE>());
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_helpers_trait_syntax() {
        let nil_val: VALUE = Qnil;
        assert!(nil_val.nil_p());
        assert!(!nil_val.rtest());
        assert!(nil_val.nil_or_undef_p());
        assert!(!nil_val.fixnum_p());

        let true_val: VALUE = Qtrue;
        assert!(true_val.rtest());
        assert!(!true_val.nil_p());

        let fix_val: VALUE = int2fix(42);
        assert!(fix_val.fixnum_p());
        assert_eq!(fix_val.fix2long(), 42);
        assert!(fix_val.rtest());
    }

    #[test]
    fn test_embedded_string_helpers() {
        let mut rstr = RString {
            basic: RBasic {
                flags: (ruby_value_type::RUBY_T_STRING as VALUE),
                klass: 0x1000,
            },
            len: 5,
            as_: RStringAs { ary: [0; 1] },
        };
        let bytes = b"hello";
        unsafe {
            let ary_ptr = rstr.as_.ary.as_mut_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), ary_ptr, 5);
        }

        let str_val = &mut rstr as *mut RString as VALUE;
        assert!(!str_val.fl_any(RSTRING_NOEMBED));
        assert_eq!(str_val.rstring_len(), 5);
        assert_eq!(str_val.rstring_slice(), b"hello");
    }

    #[test]
    fn test_heap_string_helpers() {
        let mut buf = *b"world!";
        let rstr = RString {
            basic: RBasic {
                flags: (ruby_value_type::RUBY_T_STRING as VALUE) | RSTRING_NOEMBED,
                klass: 0x1000,
            },
            len: 6,
            as_: RStringAs {
                heap: RStringHeap {
                    ptr: buf.as_mut_ptr() as *mut c_char,
                    aux: RStringHeapAux { capa: 10 },
                },
            },
        };

        let str_val = &rstr as *const RString as VALUE;
        assert!(str_val.fl_any(RSTRING_NOEMBED));
        assert_eq!(str_val.rstring_len(), 6);
        assert_eq!(str_val.rstring_slice(), b"world!");
    }

    #[test]
    fn test_embedded_array_helpers() {
        let mut rary = RArray {
            basic: RBasic {
                flags: (ruby_value_type::RUBY_T_ARRAY as VALUE)
                    | RARRAY_EMBED_FLAG
                    | (2 << RARRAY_EMBED_LEN_SHIFT),
                klass: 0x2000,
            },
            as_: RArrayAs { ary: [0; 1] },
        };
        let elem0 = int2fix(100);
        let elem1 = int2fix(200);
        unsafe {
            let ary_ptr = rary.as_.ary.as_mut_ptr();
            *ary_ptr = elem0;
            *ary_ptr.offset(1) = elem1;
        }

        let ary_val = &rary as *const RArray as VALUE;
        assert_eq!(ary_val.rarray_len(), 2);
        assert_eq!(ary_val.rarray_aref(0), elem0);
        assert_eq!(ary_val.rarray_aref(1), elem1);
        assert_eq!(ary_val.rarray_slice(), &[elem0, elem1]);
    }
}
