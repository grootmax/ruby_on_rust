//! `VALUE`, `ID` and the special constants of CRuby.
//!
//! Transcribed from `include/ruby/internal/special_consts.h`,
//! `include/ruby/internal/value.h` and
//! `include/ruby/internal/arithmetic/{long,fixnum}.h`.  Each helper uses the
//! formula of the C inline function it is named after.
//!
//! The constants depend on `USE_FLONUM`, which the C headers set to
//! `SIZEOF_VALUE >= SIZEOF_DOUBLE`, i.e. 1 on 64-bit targets.  core_rs
//! derives it from the pointer width; `internal/core_rs.h` asserts at C
//! compile time that the C build agrees (so a build with `-DUSE_FLONUM=0` on
//! a 64-bit target fails to compile instead of corrupting objects).

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

use core::ffi::c_long;

/// `VALUE`: `uintptr_t` on every target core_rs supports.
pub type VALUE = usize;
/// `SIGNED_VALUE`: `intptr_t`.
pub type SIGNED_VALUE = isize;
/// `ID`: `uintptr_t`.
pub type ID = usize;

// core_rs supports LP64 and ILP32 targets only (Linux, macOS, FreeBSD), where
// `long` has the width of a pointer.  The fixnum helpers rely on it.
const _: () = assert!(core::mem::size_of::<c_long>() == core::mem::size_of::<VALUE>());

/// `USE_FLONUM`.
pub const USE_FLONUM: bool = cfg!(target_pointer_width = "64");

#[cfg(target_pointer_width = "64")]
mod consts {
    use super::VALUE;
    pub const Qfalse: VALUE = 0x00;
    pub const Qnil: VALUE = 0x04;
    pub const Qtrue: VALUE = 0x14;
    pub const Qundef: VALUE = 0x24;
    pub const IMMEDIATE_MASK: VALUE = 0x07;
    pub const FIXNUM_FLAG: VALUE = 0x01;
    pub const FLONUM_MASK: VALUE = 0x03;
    pub const FLONUM_FLAG: VALUE = 0x02;
    pub const SYMBOL_FLAG: VALUE = 0x0c;
}

#[cfg(not(target_pointer_width = "64"))]
mod consts {
    use super::VALUE;
    pub const Qfalse: VALUE = 0x00;
    pub const Qnil: VALUE = 0x02;
    pub const Qtrue: VALUE = 0x06;
    pub const Qundef: VALUE = 0x0a;
    pub const IMMEDIATE_MASK: VALUE = 0x03;
    pub const FIXNUM_FLAG: VALUE = 0x01;
    pub const FLONUM_MASK: VALUE = 0x00;
    pub const FLONUM_FLAG: VALUE = 0x02;
    pub const SYMBOL_FLAG: VALUE = 0x0e;
}

pub use consts::*;

/// `RUBY_SPECIAL_SHIFT`.
pub const SPECIAL_SHIFT: u32 = 8;

/// `RUBY_FIXNUM_MAX`: `LONG_MAX / 2`.
pub const FIXNUM_MAX: c_long = c_long::MAX / 2;
/// `RUBY_FIXNUM_MIN`: `LONG_MIN / 2`.
pub const FIXNUM_MIN: c_long = c_long::MIN / 2;

/// `RB_TEST()` / `RTEST()`: false only for `Qfalse` and `Qnil`.
#[inline]
pub const fn RTEST(obj: VALUE) -> bool {
    obj & !Qnil != 0
}

/// `RB_NIL_P()`.
#[inline]
pub const fn NIL_P(obj: VALUE) -> bool {
    obj == Qnil
}

/// `RB_UNDEF_P()`.
#[inline]
pub const fn UNDEF_P(obj: VALUE) -> bool {
    obj == Qundef
}

/// `RB_NIL_OR_UNDEF_P()`.
#[inline]
pub const fn NIL_OR_UNDEF_P(obj: VALUE) -> bool {
    let mask: VALUE = !(Qundef ^ Qnil);
    let common_bits: VALUE = Qundef & Qnil;
    (obj & mask) == common_bits
}

/// `RB_FIXNUM_P()`.
#[inline]
pub const fn FIXNUM_P(obj: VALUE) -> bool {
    obj & FIXNUM_FLAG != 0
}

/// `RB_STATIC_SYM_P()`.
#[inline]
pub const fn STATIC_SYM_P(obj: VALUE) -> bool {
    let mask: VALUE = !(VALUE::MAX << SPECIAL_SHIFT);
    (obj & mask) == SYMBOL_FLAG
}

/// `RB_FLONUM_P()`.
#[inline]
pub const fn FLONUM_P(obj: VALUE) -> bool {
    USE_FLONUM && (obj & FLONUM_MASK) == FLONUM_FLAG
}

/// `RB_IMMEDIATE_P()`.
#[inline]
pub const fn IMMEDIATE_P(obj: VALUE) -> bool {
    obj & IMMEDIATE_MASK != 0
}

/// `RB_SPECIAL_CONST_P()`.
#[inline]
pub const fn SPECIAL_CONST_P(obj: VALUE) -> bool {
    obj == Qfalse || IMMEDIATE_P(obj)
}

/// `RB_POSFIXABLE()`: `v < RUBY_FIXNUM_MAX + 1`, written so that it cannot
/// overflow.
#[inline]
pub const fn POSFIXABLE(v: c_long) -> bool {
    v <= FIXNUM_MAX
}

/// `RB_NEGFIXABLE()`.
#[inline]
pub const fn NEGFIXABLE(v: c_long) -> bool {
    v >= FIXNUM_MIN
}

/// `RB_FIXABLE()`.
#[inline]
pub const fn FIXABLE(v: c_long) -> bool {
    POSFIXABLE(v) && NEGFIXABLE(v)
}

/// `RB_INT2FIX()` / `LONG2FIX()`.
///
/// Precondition (as in C, where it is an assertion): `FIXABLE(i)`.
#[inline]
pub const fn INT2FIX(i: c_long) -> VALUE {
    let j = i as usize;
    (j << 1).wrapping_add(FIXNUM_FLAG)
}

/// `RB_FIX2LONG()` / `rb_fix2long()` (the arithmetic-shift variant, which
/// the C selects on every target with an arithmetic right shift).
///
/// Precondition (as in C): `FIXNUM_P(x)`.
#[inline]
pub const fn FIX2LONG(x: VALUE) -> c_long {
    ((x as SIGNED_VALUE) >> 1) as c_long
}

/// `RB_FIX2ULONG()`.
#[inline]
pub const fn FIX2ULONG(x: VALUE) -> core::ffi::c_ulong {
    FIX2LONG(x) as core::ffi::c_ulong
}

/// `RBOOL()`: `Qtrue` or `Qfalse`.
#[inline]
pub const fn RBOOL(b: bool) -> VALUE {
    if b { Qtrue } else { Qfalse }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_constants() {
        // The values of enum ruby_special_consts for this target.
        if USE_FLONUM {
            assert_eq!((Qfalse, Qnil, Qtrue, Qundef), (0x00, 0x04, 0x14, 0x24));
        } else {
            assert_eq!((Qfalse, Qnil, Qtrue, Qundef), (0x00, 0x02, 0x06, 0x0a));
        }
        assert!(!RTEST(Qfalse));
        assert!(!RTEST(Qnil));
        assert!(RTEST(Qtrue));
        assert!(RTEST(Qundef));
        assert!(RTEST(INT2FIX(0)));
        assert!(NIL_P(Qnil) && !NIL_P(Qfalse) && !NIL_P(Qundef));
        assert!(UNDEF_P(Qundef) && !UNDEF_P(Qnil));
        assert!(NIL_OR_UNDEF_P(Qnil) && NIL_OR_UNDEF_P(Qundef));
        assert!(!NIL_OR_UNDEF_P(Qfalse) && !NIL_OR_UNDEF_P(Qtrue));
        for v in [Qfalse, Qnil, Qtrue, Qundef, INT2FIX(-1)] {
            assert!(SPECIAL_CONST_P(v));
        }
        // An aligned heap pointer is not special.
        assert!(!SPECIAL_CONST_P(0x7f00_0000_1000usize as VALUE & !0x7));
        assert!(!SPECIAL_CONST_P(0x40));
    }

    #[test]
    fn nil_or_undef_matches_its_definition() {
        for obj in (0..4096usize).chain([VALUE::MAX, VALUE::MAX - 4, 1 << 40]) {
            assert_eq!(NIL_OR_UNDEF_P(obj), obj == Qnil || obj == Qundef, "{obj:#x}");
            assert_eq!(RTEST(obj), obj != Qnil && obj != Qfalse, "{obj:#x}");
        }
    }

    #[test]
    fn static_symbols_and_flonums() {
        let sym = (1234usize << SPECIAL_SHIFT) | SYMBOL_FLAG;
        assert!(STATIC_SYM_P(sym));
        assert!(!FIXNUM_P(sym));
        assert!(!STATIC_SYM_P(Qnil));
        assert!(!STATIC_SYM_P(sym | 0x1));
        if USE_FLONUM {
            assert!(FLONUM_P(0x1234_5672));
            assert!(!FLONUM_P(Qnil) && !FLONUM_P(Qtrue) && !FLONUM_P(INT2FIX(3)));
        } else {
            for v in 0..64usize {
                assert!(!FLONUM_P(v));
            }
        }
    }

    #[test]
    fn fixnum_round_trip() {
        let edge = [
            0, 1, -1, 2, -2, 42, -42, 0x3fff_ffff, -0x4000_0000,
            FIXNUM_MAX, FIXNUM_MIN, FIXNUM_MAX - 1, FIXNUM_MIN + 1,
        ];
        for i in edge {
            assert!(FIXABLE(i));
            let v = INT2FIX(i);
            assert!(FIXNUM_P(v), "{i}");
            assert!(SPECIAL_CONST_P(v));
            assert_eq!(FIX2LONG(v), i);
            // The C formula: 2*i + 1, in two's complement.
            assert_eq!(v, (i as usize).wrapping_mul(2).wrapping_add(1));
        }
        assert_eq!(INT2FIX(-1), VALUE::MAX);
        assert_eq!(INT2FIX(0), 1);
        assert_eq!(FIX2ULONG(INT2FIX(-1)), core::ffi::c_ulong::MAX);
        assert!(!FIXABLE(FIXNUM_MAX + 1) && !FIXABLE(FIXNUM_MIN - 1));
        assert!(!FIXABLE(c_long::MAX) && !FIXABLE(c_long::MIN));
        assert!(POSFIXABLE(c_long::MIN) && NEGFIXABLE(c_long::MAX));
        assert_eq!(RBOOL(true), Qtrue);
        assert_eq!(RBOOL(false), Qfalse);
    }
}
