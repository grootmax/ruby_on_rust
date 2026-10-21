//! Ports of imemo.c: the imemo type names used by GC debug output,
//! `ObjectSpace.dump` and VM assertions (port unit `imemo-A-01` of
//! `tool/core_rs/inventory.rb`).
//!
//! `rb_imemo_name` is exported (`RUBY_SYMBOL_EXPORT_BEGIN` in
//! internal/imemo.h; ext/objspace calls it), so the Rust function keeps its C
//! name and the export set is unchanged.

use core::ffi::{CStr, c_uint};

/// The `enum imemo_type` values of internal/imemo.h.  imemo.c asserts each
/// of them at compile time (`STATIC_ASSERT` in its `#if USE_RUST_PORTS`
/// branch), so a renumbering there fails the build instead of mislabelling.
const IMEMO_ENV: c_uint = 0;
const IMEMO_CREF: c_uint = 1;
const IMEMO_SVAR: c_uint = 2;
const IMEMO_THROW_DATA: c_uint = 3;
const IMEMO_IFUNC: c_uint = 4;
const IMEMO_MEMO: c_uint = 5;
const IMEMO_MENT: c_uint = 6;
const IMEMO_ISEQ: c_uint = 7;
const IMEMO_TMPBUF: c_uint = 8;
const IMEMO_CVAR_ENTRY: c_uint = 9;
const IMEMO_CALLINFO: c_uint = 10;
const IMEMO_CALLCACHE: c_uint = 11;
const IMEMO_CONSTCACHE: c_uint = 12;
const IMEMO_FIELDS: c_uint = 13;
const IMEMO_SUBCLASSES: c_uint = 14;
const IMEMO_CDHASH: c_uint = 15;

/// Safe core of `rb_imemo_name()`: the name of each imemo type, spelled as
/// the C `IMEMO_NAME(x)` macro stringifies it (`#x`), or `None` for a value
/// that is not an `enum imemo_type`.
pub fn imemo_name(t: c_uint) -> Option<&'static CStr> {
    Some(match t {
        IMEMO_CALLCACHE => c"callcache",
        IMEMO_CALLINFO => c"callinfo",
        IMEMO_CONSTCACHE => c"constcache",
        IMEMO_CREF => c"cref",
        IMEMO_ENV => c"env",
        IMEMO_IFUNC => c"ifunc",
        IMEMO_ISEQ => c"iseq",
        IMEMO_MEMO => c"memo",
        IMEMO_MENT => c"ment",
        IMEMO_SVAR => c"svar",
        IMEMO_THROW_DATA => c"throw_data",
        IMEMO_TMPBUF => c"tmpbuf",
        IMEMO_CVAR_ENTRY => c"cvar_entry",
        IMEMO_FIELDS => c"fields",
        IMEMO_SUBCLASSES => c"subclasses",
        IMEMO_CDHASH => c"cdhash",
        _ => return None,
    })
}

/// C boundary for `const char *rb_imemo_name(enum imemo_type type)`.  As in
/// C, a value outside the enum is a bug: it calls `rb_bug("unreachable")`,
/// which does not return.  The returned string is static and NUL-terminated,
/// like the C string literals; callers only read it.
///
/// # Safety
///
/// No pointers are read.  Any `type` is accepted; non-enum values end the
/// process through `rb_bug`, exactly as in C.
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_imemo_name(t: c_uint) -> *const core::ffi::c_char {
    match imemo_name(t) {
        Some(name) => name.as_ptr(),
        // SAFETY: rb_bug takes a printf format; "unreachable" has no
        // conversions, so no variadic arguments are read.
        None => unsafe { crate::ffi::api::rb_bug(c"unreachable".as_ptr()) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_enum_value_has_its_c_name() {
        let names = [
            "env", "cref", "svar", "throw_data", "ifunc", "memo", "ment", "iseq", "tmpbuf",
            "cvar_entry", "callinfo", "callcache", "constcache", "fields", "subclasses", "cdhash",
        ];
        for (t, name) in names.iter().enumerate() {
            assert_eq!(imemo_name(t as c_uint).map(CStr::to_bytes), Some(name.as_bytes()), "type {t}");
        }
    }

    #[test]
    fn values_outside_the_enum_have_no_name() {
        for t in [16, 17, 255, 0x8000_0000, c_uint::MAX] {
            assert_eq!(imemo_name(t), None, "type {t}");
        }
    }
}
