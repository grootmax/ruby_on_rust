//! `Value`, `VALUE`, `ID` and the special constants of CRuby.
//!
//! Transcribed from `include/ruby/internal/special_consts.h`,
//! `include/ruby/internal/value.h` and
//! `include/ruby/internal/arithmetic/{long,fixnum}.h`.  Each helper uses the
//! formula of the C inline function it is named after.
//!
//! The constants depend on `USE_FLONUM`, which the C headers set to
//! `SIZEOF_VALUE >= SIZEOF_DOUBLE` (1 on 64-bit targets) unless the build
//! overrides it, e.g. with `cppflags=-DUSE_FLONUM=0`.  core_rs/core_rs.mk
//! asks the C preprocessor for the value the C sources see (core_rs/cfg.c)
//! and passes it to rustc as `--cfg core_rs_flonum` or
//! `--cfg core_rs_no_flonum`, so both sides always agree.

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

#[cfg(not(any(core_rs_flonum, core_rs_no_flonum)))]
compile_error!("build core_rs through core_rs/core_rs.mk: it passes --cfg core_rs_flonum or --cfg core_rs_no_flonum");
#[cfg(all(core_rs_flonum, core_rs_no_flonum))]
compile_error!("--cfg core_rs_flonum and --cfg core_rs_no_flonum are exclusive");
#[cfg(all(core_rs_flonum, not(target_pointer_width = "64")))]
compile_error!("USE_FLONUM needs a 64-bit VALUE");

/// `USE_FLONUM`, as the C build sees it.
pub const USE_FLONUM: bool = cfg!(core_rs_flonum);

#[cfg(core_rs_flonum)]
pub mod consts {
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

#[cfg(not(core_rs_flonum))]
pub mod consts {
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

/// A type-safe, transparent wrapper around CRuby's `VALUE` bit pattern (`usize`).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub VALUE);

impl Value {
    /// `Qfalse` constant as a `Value`.
    pub const Qfalse: Value = Value(consts::Qfalse);
    /// `Qnil` constant as a `Value`.
    pub const Qnil: Value = Value(consts::Qnil);
    /// `Qtrue` constant as a `Value`.
    pub const Qtrue: Value = Value(consts::Qtrue);
    /// `Qundef` constant as a `Value`.
    pub const Qundef: Value = Value(consts::Qundef);

    /// Creates a `Value` from a raw `VALUE` (`usize`).
    #[inline]
    pub const fn from_raw(raw: VALUE) -> Self {
        Value(raw)
    }

    /// Returns the underlying raw `VALUE` (`usize`).
    #[inline]
    pub const fn as_raw(self) -> VALUE {
        self.0
    }

    /// `RB_TEST()` / `RTEST()`: false only for `Qfalse` and `Qnil`.
    #[inline]
    pub const fn rtest(self) -> bool {
        RTEST(self.0)
    }

    /// `RB_NIL_P()`.
    #[inline]
    pub const fn nil_p(self) -> bool {
        NIL_P(self.0)
    }

    /// `RB_UNDEF_P()`.
    #[inline]
    pub const fn undef_p(self) -> bool {
        UNDEF_P(self.0)
    }

    /// `RB_NIL_OR_UNDEF_P()`.
    #[inline]
    pub const fn nil_or_undef_p(self) -> bool {
        NIL_OR_UNDEF_P(self.0)
    }

    /// `RB_FIXNUM_P()`.
    #[inline]
    pub const fn fixnum_p(self) -> bool {
        FIXNUM_P(self.0)
    }

    /// `RB_STATIC_SYM_P()` / `SYMBOL_P()`.
    #[inline]
    pub const fn symbol_p(self) -> bool {
        STATIC_SYM_P(self.0)
    }

    /// `RB_FLONUM_P()`.
    #[inline]
    pub const fn flonum_p(self) -> bool {
        FLONUM_P(self.0)
    }

    /// `RB_IMMEDIATE_P()`.
    #[inline]
    pub const fn immediate_p(self) -> bool {
        IMMEDIATE_P(self.0)
    }

    /// `RB_SPECIAL_CONST_P()`.
    #[inline]
    pub const fn special_const_p(self) -> bool {
        SPECIAL_CONST_P(self.0)
    }

    /// `RB_FIX2LONG()`.
    #[inline]
    pub const fn fix2long(self) -> c_long {
        FIX2LONG(self.0)
    }

    /// `RB_FIX2ULONG()`.
    #[inline]
    pub const fn fix2ulong(self) -> core::ffi::c_ulong {
        FIX2ULONG(self.0)
    }

    /// `RB_INT2FIX()` / `LONG2FIX()`.
    #[inline]
    pub const fn int2fix(i: c_long) -> Self {
        Value(INT2FIX(i))
    }

    /// `RBOOL()`.
    #[inline]
    pub const fn rbool(b: bool) -> Self {
        Value(RBOOL(b))
    }
}

impl core::fmt::Debug for Value {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.nil_p() {
            write!(f, "Value(Qnil)")
        } else if self.undef_p() {
            write!(f, "Value(Qundef)")
        } else if *self == Value::Qtrue {
            write!(f, "Value(Qtrue)")
        } else if *self == Value::Qfalse {
            write!(f, "Value(Qfalse)")
        } else if self.fixnum_p() {
            write!(f, "Value(Fixnum:{})", self.fix2long())
        } else {
            write!(f, "Value({:#x})", self.0)
        }
    }
}

impl From<VALUE> for Value {
    #[inline]
    fn from(raw: VALUE) -> Self {
        Value(raw)
    }
}

impl From<Value> for VALUE {
    #[inline]
    fn from(val: Value) -> Self {
        val.0
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(b: bool) -> Self {
        Value::rbool(b)
    }
}

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

/// `SYMBOL_P()` alias for `STATIC_SYM_P()`.
#[inline]
pub const fn SYMBOL_P(obj: VALUE) -> bool {
    STATIC_SYM_P(obj)
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
            assert_eq!(
                (Value::Qfalse.0, Value::Qnil.0, Value::Qtrue.0, Value::Qundef.0),
                (0x00, 0x04, 0x14, 0x24)
            );
        } else {
            assert_eq!((Qfalse, Qnil, Qtrue, Qundef), (0x00, 0x02, 0x06, 0x0a));
            assert_eq!(
                (Value::Qfalse.0, Value::Qnil.0, Value::Qtrue.0, Value::Qundef.0),
                (0x00, 0x02, 0x06, 0x0a)
            );
        }
        assert!(!RTEST(Qfalse));
        assert!(!RTEST(Qnil));
        assert!(RTEST(Qtrue));
        assert!(RTEST(Qundef));
        assert!(RTEST(INT2FIX(0)));

        assert!(!Value::Qfalse.rtest());
        assert!(!Value::Qnil.rtest());
        assert!(Value::Qtrue.rtest());
        assert!(Value::Qundef.rtest());
        assert!(Value::int2fix(0).rtest());

        assert!(NIL_P(Qnil) && !NIL_P(Qfalse) && !NIL_P(Qundef));
        assert!(Value::Qnil.nil_p() && !Value::Qfalse.nil_p() && !Value::Qundef.nil_p());

        assert!(UNDEF_P(Qundef) && !UNDEF_P(Qnil));
        assert!(Value::Qundef.undef_p() && !Value::Qnil.undef_p());

        assert!(NIL_OR_UNDEF_P(Qnil) && NIL_OR_UNDEF_P(Qundef));
        assert!(Value::Qnil.nil_or_undef_p() && Value::Qundef.nil_or_undef_p());

        assert!(!NIL_OR_UNDEF_P(Qfalse) && !NIL_OR_UNDEF_P(Qtrue));
        assert!(!Value::Qfalse.nil_or_undef_p() && !Value::Qtrue.nil_or_undef_p());

        for v in [Qfalse, Qnil, Qtrue, Qundef, INT2FIX(-1)] {
            assert!(SPECIAL_CONST_P(v));
            assert!(Value(v).special_const_p());
        }
        // An aligned heap pointer is not special.
        assert!(!SPECIAL_CONST_P(0x7f00_0000_1000usize as VALUE & !0x7));
        assert!(!SPECIAL_CONST_P(0x40));
        assert!(!Value(0x40).special_const_p());
    }

    #[test]
    fn nil_or_undef_matches_its_definition() {
        for obj in (0..4096usize).chain([VALUE::MAX, VALUE::MAX - 4, 1 << 40]) {
            let val = Value(obj);
            assert_eq!(NIL_OR_UNDEF_P(obj), obj == Qnil || obj == Qundef, "{obj:#x}");
            assert_eq!(val.nil_or_undef_p(), obj == Qnil || obj == Qundef, "{obj:#x}");
            assert_eq!(RTEST(obj), obj != Qnil && obj != Qfalse, "{obj:#x}");
            assert_eq!(val.rtest(), obj != Qnil && obj != Qfalse, "{obj:#x}");
        }
    }

    #[test]
    fn static_symbols_and_flonums() {
        let sym = (1234usize << SPECIAL_SHIFT) | SYMBOL_FLAG;
        let val_sym = Value(sym);
        assert!(STATIC_SYM_P(sym));
        assert!(SYMBOL_P(sym));
        assert!(val_sym.symbol_p());
        assert!(!FIXNUM_P(sym));
        assert!(!val_sym.fixnum_p());
        assert!(!STATIC_SYM_P(Qnil));
        assert!(!Value::Qnil.symbol_p());
        assert!(!STATIC_SYM_P(sym | 0x1));
        if USE_FLONUM {
            assert!(FLONUM_P(0x1234_5672));
            assert!(Value(0x1234_5672).flonum_p());
            assert!(!FLONUM_P(Qnil) && !FLONUM_P(Qtrue) && !FLONUM_P(INT2FIX(3)));
            assert!(!Value::Qnil.flonum_p() && !Value::Qtrue.flonum_p() && !Value::int2fix(3).flonum_p());
        } else {
            for v in 0..64usize {
                assert!(!FLONUM_P(v));
                assert!(!Value(v).flonum_p());
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
            let val = Value::int2fix(i);
            assert_eq!(val.0, v);
            assert!(FIXNUM_P(v), "{i}");
            assert!(val.fixnum_p(), "{i}");
            assert!(SPECIAL_CONST_P(v));
            assert!(val.special_const_p());
            assert_eq!(FIX2LONG(v), i);
            assert_eq!(val.fix2long(), i);
            // The C formula: 2*i + 1, in two's complement.
            assert_eq!(v, (i as usize).wrapping_mul(2).wrapping_add(1));
        }
        assert_eq!(INT2FIX(-1), VALUE::MAX);
        assert_eq!(INT2FIX(0), 1);
        assert_eq!(FIX2ULONG(INT2FIX(-1)), core::ffi::c_ulong::MAX);
        assert_eq!(Value::int2fix(-1).fix2ulong(), core::ffi::c_ulong::MAX);
        assert!(!FIXABLE(FIXNUM_MAX + 1) && !FIXABLE(FIXNUM_MIN - 1));
        assert!(!FIXABLE(c_long::MAX) && !FIXABLE(c_long::MIN));
        assert!(POSFIXABLE(c_long::MIN) && NEGFIXABLE(c_long::MAX));
        assert_eq!(RBOOL(true), Qtrue);
        assert_eq!(RBOOL(false), Qfalse);
        assert_eq!(Value::rbool(true), Value::Qtrue);
        assert_eq!(Value::rbool(false), Value::Qfalse);
    }
}
