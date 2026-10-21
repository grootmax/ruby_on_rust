//! Ports of re.c: byte-string search and ASCII case-insensitive compare,
//! the Unicode range check of the regexp source unescaper (port unit
//! `re-A-02`), and the regexp option letters, the match-offset comparator,
//! MatchData#inspect's named-group collector and the escaped-byte reader
//! (port unit `re-A-01`).
//!
//! Public C API (include/ruby/internal/encoding/string.h,
//! include/ruby/internal/intern/re.h and
//! include/ruby/internal/encoding/encoding.h): `rb_memsearch`, `rb_memcicmp`,
//! `rb_char_to_option_kcode`.
//! `rb_memsearch` is behind String#index, #include?, #start_with?-style
//! scans, #split, #sub/#gsub with string patterns, and more.
//!
//! Faithfulness note.  The C Quick Search loops compute the next shift from
//! the byte *after* the current window even at the last window (reading the
//! string's terminator), and the UTF-8 variant hashes up to three further
//! bytes, which can lie past the end of either string when it ends in a
//! truncated multibyte character.  The ports below never read outside the
//! two strings: they stop at the last window (the shift could not change the
//! result there) and treat bytes past the end as 0, which is what C reads
//! when the strings are NUL-terminated.  Results are identical whenever the C
//! result does not depend on memory beyond the strings.

use core::ffi::{c_char, c_int, c_long, c_ulong, c_void};

/// The leading fields of `OnigEncodingTypeST` (include/ruby/onigmo.h), which
/// is public ABI.  Only `min_enc_len` is read.
#[repr(C)]
struct OnigEncodingHead {
    precise_mbc_enc_len: *const c_void,
    name: *const c_char,
    max_enc_len: c_int,
    min_enc_len: c_int,
}

unsafe extern "C" {
    fn rb_utf8_encoding() -> *const c_void;
    #[cfg(not(target_vendor = "apple"))]
    fn memmem(haystack: *const c_void, hlen: usize, needle: *const c_void, nlen: usize) -> *mut c_void;
}

const VALUE_BYTES: usize = core::mem::size_of::<usize>();

/// `casetable[b]` in re.c: ASCII upper case to lower case, all else
/// unchanged.  The C table is `const char[]`, so entries >= 0x80 are
/// negative where `char` is signed (x86) and positive where it is unsigned
/// (e.g. AArch64 Linux); `c_char` reproduces exactly that.
#[inline]
fn fold(b: u8) -> c_int {
    b.to_ascii_lowercase() as c_char as c_int
}

/// Safe core of `rb_memcicmp`.
pub fn memcicmp(x: &[u8], y: &[u8]) -> c_int {
    for (&a, &b) in x.iter().zip(y) {
        let d = fold(a) - fold(b);
        if d != 0 {
            return d;
        }
    }
    0
}

/// Port of `rb_memcicmp()` (re.c).
///
/// # Safety
/// `x` and `y` are readable for `len` bytes.  (C loops forever on a
/// negative `len`; this returns 0 instead.)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_memcicmp(x: *const c_void, y: *const c_void, len: c_long) -> c_int {
    if len <= 0 {
        return 0;
    }
    let len = len as usize;
    // SAFETY: readable for `len` bytes per the contract.
    let (x, y) = unsafe {
        (core::slice::from_raw_parts(x as *const u8, len), core::slice::from_raw_parts(y as *const u8, len))
    };
    memcicmp(x, y)
}

/// Byte `i` of `s`, or 0 past the end (see the module note).
#[inline]
fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// `rb_memsearch_ss` for platforms without a usable memmem (re.c uses it on
/// Apple): first byte via memchr, then a rolling hash of up to
/// `size_of::<VALUE>()` bytes.  Requires `2 <= x.len() <= VALUE_BYTES` and
/// `x.len() < y.len()`.
pub fn search_ss_rolling(x: &[u8], y: &[u8]) -> c_long {
    let (m, n) = (x.len(), y.len());
    debug_assert!((2..=VALUE_BYTES).contains(&m) && m < n);
    let mask = usize::MAX >> ((VALUE_BYTES - m) * 8);

    let Some(mut yi) = y[..n - m + 1].iter().position(|&b| b == x[0]) else {
        return -1;
    };
    let mut hx: usize = 0;
    let mut hy: usize = 0;
    for k in 0..m {
        hx = (hx << 8) | x[k] as usize;
        hy = (hy << 8) | y[yi + k] as usize;
    }
    yi += m;
    while hx != hy {
        if yi == n {
            return -1;
        }
        hy = ((hy << 8) | y[yi] as usize) & mask;
        yi += 1;
    }
    (yi - m) as c_long
}

/// `rb_memsearch_ss` where re.c uses the C library's memmem.
#[cfg(not(target_vendor = "apple"))]
fn search_ss(x: &[u8], y: &[u8]) -> c_long {
    // SAFETY: both slices are valid for their lengths.
    let p = unsafe { memmem(y.as_ptr() as *const c_void, y.len(), x.as_ptr() as *const c_void, x.len()) };
    if p.is_null() { -1 } else { (p as usize - y.as_ptr() as usize) as c_long }
}

#[cfg(target_vendor = "apple")]
fn search_ss(x: &[u8], y: &[u8]) -> c_long {
    search_ss_rolling(x, y)
}

/// `rb_memsearch_qs`: Quick Search (Sunday).  Requires `1 <= m < n`.
pub fn search_qs(x: &[u8], y: &[u8]) -> c_long {
    let (m, n) = (x.len(), y.len());
    let mut table = [m + 1; 256];
    for (i, &b) in x.iter().enumerate() {
        table[b as usize] = m - i;
    }
    let mut pos = 0;
    while pos + m <= n {
        if x[0] == y[pos] && x == &y[pos..pos + m] {
            return pos as c_long;
        }
        if pos + m == n {
            break;
        }
        pos += table[y[pos + m] as usize];
    }
    -1
}

/// `rb_memsearch_qs_utf8_hash`, reading bytes at `s[i..i+4]`.
pub fn qs_utf8_hash(s: &[u8], i: usize) -> usize {
    const MIX: u32 = 8353;
    let mut h = at(s, i) as u32;
    if h < 0xC0 {
        return h as usize + 256;
    } else if h < 0xE0 {
        h = h.wrapping_mul(MIX).wrapping_add(at(s, i + 1) as u32);
    } else if h < 0xF0 {
        h = h.wrapping_mul(MIX).wrapping_add(at(s, i + 1) as u32);
        h = h.wrapping_mul(MIX).wrapping_add(at(s, i + 2) as u32);
    } else if h < 0xF5 {
        h = h.wrapping_mul(MIX).wrapping_add(at(s, i + 1) as u32);
        h = h.wrapping_mul(MIX).wrapping_add(at(s, i + 2) as u32);
        h = h.wrapping_mul(MIX).wrapping_add(at(s, i + 3) as u32);
    } else {
        return h as usize + 256;
    }
    (h as u8) as usize
}

/// `rb_memsearch_qs_utf8`: Quick Search with a UTF-8 character hash.
/// Requires `1 <= m < n`.
pub fn search_qs_utf8(x: &[u8], y: &[u8]) -> c_long {
    let (m, n) = (x.len(), y.len());
    let mut table = [m + 1; 512];
    for i in 0..m {
        table[qs_utf8_hash(x, i)] = m - i;
    }
    let mut pos = 0;
    while pos + m <= n {
        if x[0] == y[pos] && x == &y[pos..pos + m] {
            return pos as c_long;
        }
        if pos + m == n {
            break;
        }
        pos += table[qs_utf8_hash(y, pos + m)];
    }
    -1
}

/// `rb_memsearch_with_char_size`: candidate positions are multiples of
/// `char_size` (UTF-16/UTF-32 code units).  Requires `1 <= m < n`.
pub fn search_char_size(x: &[u8], y: &[u8], char_size: usize) -> c_long {
    let (m, n) = (x.len(), y.len());
    let mut pos = 0;
    while pos + m <= n {
        if x[0] == y[pos] && x[1..] == y[pos + 1..pos + m] {
            return pos as c_long;
        }
        pos += char_size;
    }
    -1
}

/// Port of `rb_memsearch()` (re.c).
///
/// # Safety
/// `x0` is readable for `m` bytes and `y0` for `n` bytes; `enc` is a valid
/// encoding whenever `1 < m < n` (it is not read otherwise, as in C).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_memsearch(
    x0: *const c_void,
    m: c_long,
    y0: *const c_void,
    n: c_long,
    enc: *const c_void,
) -> c_long {
    if m > n {
        return -1;
    }
    if m == n {
        if m <= 0 {
            return 0; // memcmp of zero bytes
        }
        // SAFETY: both readable for m == n bytes.
        let (x, y) = unsafe {
            (
                core::slice::from_raw_parts(x0 as *const u8, m as usize),
                core::slice::from_raw_parts(y0 as *const u8, n as usize),
            )
        };
        return if x == y { 0 } else { -1 };
    }
    if m < 1 {
        return 0;
    }
    // Here 1 <= m < n.
    // SAFETY: readable for m and n bytes per the contract.
    let (x, y) = unsafe {
        (
            core::slice::from_raw_parts(x0 as *const u8, m as usize),
            core::slice::from_raw_parts(y0 as *const u8, n as usize),
        )
    };
    if m == 1 {
        return match y.iter().position(|&b| b == x[0]) {
            Some(i) => i as c_long,
            None => -1,
        };
    }
    // SAFETY: `enc` points to an encoding; only its public head is read.
    let mbminlen = unsafe { (*(enc as *const OnigEncodingHead)).min_enc_len };
    match mbminlen {
        1 => {
            if x.len() <= VALUE_BYTES {
                return search_ss(x, y);
            }
            // SAFETY: plain function call returning a static encoding.
            if enc == unsafe { rb_utf8_encoding() } {
                return search_qs_utf8(x, y);
            }
        }
        2 => return search_char_size(x, y, 2),
        4 => return search_char_size(x, y, 4),
        _ => {}
    }
    search_qs(x, y)
}

/// `ONIG_MAX_ERROR_MESSAGE_LEN` (include/ruby/onigmo.h): the size of re.c's
/// `onig_errmsg_buffer`.
pub const ONIG_MAX_ERROR_MESSAGE_LEN: usize = 90;

/// The message `check_unicode_range` copies with `errcpy`, NUL included.
const INVALID_UNICODE_RANGE: &[u8] = b"invalid Unicode range\0";

/// Safe core of re.c's `check_unicode_range`: -1 for a surrogate
/// (U+D800..U+DFFF) or a value above U+10FFFF, after copying
/// "invalid Unicode range" into `err` exactly as `errcpy` (`strlcpy` with
/// the buffer size) does: the message and its NUL, nothing after it.
/// Otherwise 0, and `err` is untouched.
pub fn check_unicode_range(code: c_ulong, err: &mut [u8; ONIG_MAX_ERROR_MESSAGE_LEN]) -> c_int {
    if (0xd800..=0xdfff).contains(&code) || 0x10ffff < code {
        err[..INVALID_UNICODE_RANGE.len()].copy_from_slice(INVALID_UNICODE_RANGE);
        return -1;
    }
    0
}

/// C boundary for re.c's `static int check_unicode_range(unsigned long code,
/// onig_errmsg_buffer err)`, called by `append_utf8` and
/// `unescape_unicode_list` through `#define check_unicode_range
/// rb_core_re_check_unicode_range` (declared hidden in internal/core_rs.h).
///
/// # Safety
///
/// `err` must point to a writable `onig_errmsg_buffer`
/// (`char[ONIG_MAX_ERROR_MESSAGE_LEN]`), as every caller in re.c passes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_re_check_unicode_range(code: c_ulong, err: *mut c_char) -> c_int {
    // SAFETY: per the contract `err` is a writable buffer of
    // ONIG_MAX_ERROR_MESSAGE_LEN bytes, so it can be viewed as that array.
    let err = unsafe { &mut *(err as *mut [u8; ONIG_MAX_ERROR_MESSAGE_LEN]) };
    check_unicode_range(code, err)
}

// ---------------------------------------------------------------------------
// Port unit re-A-01: regexp option letters, the match-offset comparator, the
// named-group collector of MatchData#inspect, and the escaped-byte reader of
// the regexp source unescaper.
//
// The constants below are copied from the C headers; the `#if USE_RUST_PORTS`
// branch of re.c checks every one of them with STATIC_ASSERT.

/// `ONIG_OPTION_IGNORECASE` (include/ruby/onigmo.h).
const ONIG_OPTION_IGNORECASE: c_int = 1;
/// `ONIG_OPTION_EXTEND` (include/ruby/onigmo.h).
const ONIG_OPTION_EXTEND: c_int = 2;
/// `ONIG_OPTION_MULTILINE` (include/ruby/onigmo.h).
const ONIG_OPTION_MULTILINE: c_int = 4;
/// `ARG_ENCODING_FIXED` (internal/re.h).
const ARG_ENCODING_FIXED: c_int = 16;
/// `ARG_ENCODING_NONE` (internal/re.h).
const ARG_ENCODING_NONE: c_int = 32;
/// `rb_ascii8bit_encindex()`, a macro for `RUBY_ENCINDEX_ASCII_8BIT` in
/// encindex.h, which re.c includes.
const ENCINDEX_ASCII_8BIT: c_int = 0;
/// `rb_utf8_encindex()`, a macro for `RUBY_ENCINDEX_UTF_8` in encindex.h.
const ENCINDEX_UTF_8: c_int = 1;
/// `ENCINDEX_EUC_JP` (encindex.h).
const ENCINDEX_EUC_JP: c_int = 10;
/// `ENCINDEX_Windows_31J` (encindex.h).
const ENCINDEX_WINDOWS_31J: c_int = 11;

/// `OPTBUF_SIZE` in re.c: the size of the buffer `option_to_str` fills.
pub const OPTBUF_SIZE: usize = 4;

/// `rb_len_t` (include/ruby/internal/value.h): `long` on every platform
/// core_rs builds for (it is `int64_t` only on 64-bit mswin).  re.c asserts
/// `sizeof(rb_len_t) == sizeof(long)` in its `#if USE_RUST_PORTS` branch.
type RbLenT = c_long;

/// re.c's `char_to_option`: the Onigmo option of a regexp flag letter
/// (`i`, `x`, `m`), or 0 for any other value.
pub fn char_to_option(c: c_int) -> c_int {
    if c == b'i' as c_int {
        ONIG_OPTION_IGNORECASE
    } else if c == b'x' as c_int {
        ONIG_OPTION_EXTEND
    } else if c == b'm' as c_int {
        ONIG_OPTION_MULTILINE
    } else {
        0
    }
}

/// C boundary for re.c's `static int char_to_option(int c)`, called by
/// `rb_reg_str_with_term` and `str_to_option` through
/// `#define char_to_option rb_core_re_char_to_option`.
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_re_char_to_option(c: c_int) -> c_int {
    char_to_option(c)
}

/// re.c's `option_to_str`: writes the letters of the `m`, `i` and `x`
/// options set in `options`, in that order, then a NUL.  Bytes after the
/// NUL are not touched.
pub fn option_to_str(s: &mut [u8; OPTBUF_SIZE], options: c_int) {
    let mut p = 0;
    if options & ONIG_OPTION_MULTILINE != 0 {
        s[p] = b'm';
        p += 1;
    }
    if options & ONIG_OPTION_IGNORECASE != 0 {
        s[p] = b'i';
        p += 1;
    }
    if options & ONIG_OPTION_EXTEND != 0 {
        s[p] = b'x';
        p += 1;
    }
    s[p] = 0;
}

/// C boundary for re.c's `static char *option_to_str(char str[OPTBUF_SIZE],
/// int options)`, called by `rb_reg_desc`, `rb_reg_str_with_term` and
/// `rb_enc_reg_error_desc` through
/// `#define option_to_str rb_core_re_option_to_str`.  Returns `str`.
///
/// # Safety
///
/// `str` must point to `OPTBUF_SIZE` (4) writable bytes, as every caller in
/// re.c passes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_re_option_to_str(str: *mut c_char, options: c_int) -> *mut c_char {
    // SAFETY: per the contract `str` is a writable buffer of OPTBUF_SIZE
    // bytes, so it can be viewed as that array.
    let buf = unsafe { &mut *(str as *mut [u8; OPTBUF_SIZE]) };
    option_to_str(buf, options);
    str
}

/// Safe core of `rb_char_to_option_kcode`: returns the C return value and
/// the final values of `*option` and `*kcode`.
pub fn char_to_option_kcode(c: c_int) -> (c_int, c_int, c_int) {
    let kcode = if c == b'n' as c_int {
        return (ARG_ENCODING_NONE, ARG_ENCODING_NONE, ENCINDEX_ASCII_8BIT);
    } else if c == b'e' as c_int {
        ENCINDEX_EUC_JP
    } else if c == b's' as c_int {
        ENCINDEX_WINDOWS_31J
    } else if c == b'u' as c_int {
        ENCINDEX_UTF_8
    } else {
        let option = char_to_option(c);
        return (option, option, -1);
    };
    (1, ARG_ENCODING_FIXED, kcode)
}

/// Port of `rb_char_to_option_kcode()` (re.c), public C API
/// (include/ruby/internal/encoding/encoding.h), called by prism_compile.c.
///
/// # Safety
///
/// `option` and `kcode` must be writable.  They are written in the C order
/// (`*option = 0`, then `*kcode`, then `*option` again), so the result is
/// the same as C's even if they alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_char_to_option_kcode(c: c_int, option: *mut c_int, kcode: *mut c_int) -> c_int {
    let (ret, opt, kc) = char_to_option_kcode(c);
    // SAFETY: both out-parameters are writable per the contract.
    unsafe {
        *option = 0;
        *kcode = kc;
        *option = opt;
    }
    ret
}

/// re.c's `pair_byte_cmp` on the `byte_pos` fields of two `pair_t`s: the
/// sign of `pos1 - pos2` where `rb_len_t` is wider than `int`, else the
/// difference itself, as in the `#if SIZEOF_RB_LEN_T > SIZEOF_INT` of the C.
///
/// The subtraction wraps; C's would overflow (undefined behaviour) only for
/// operands of opposite sign near the limits.  The C precondition is that
/// both are byte offsets into a string, so `0 <= pos <= LONG_MAX`, where the
/// difference never overflows.
pub fn pair_byte_cmp(pos1: RbLenT, pos2: RbLenT) -> c_int {
    let diff = pos1.wrapping_sub(pos2);
    if core::mem::size_of::<RbLenT>() > core::mem::size_of::<c_int>() {
        if diff != 0 {
            if diff > 0 { 1 } else { -1 }
        } else {
            0
        }
    } else {
        diff as c_int
    }
}

/// C boundary for re.c's `static int pair_byte_cmp(const void *pair1,
/// const void *pair2)`, the `qsort`/`bsearch` comparator of
/// `update_char_offset`, reached through
/// `#define pair_byte_cmp rb_core_re_pair_byte_cmp`.  It reads only the
/// first field, `rb_len_t byte_pos`, of each `pair_t` (re.c asserts its
/// offset is 0).
///
/// # Safety
///
/// Both pointers must point to readable `pair_t`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_re_pair_byte_cmp(pair1: *const c_void, pair2: *const c_void) -> c_int {
    // SAFETY: per the contract both point to a pair_t, whose first field is
    // an rb_len_t at offset 0, suitably aligned.
    let (a, b) = unsafe { (*(pair1 as *const RbLenT), *(pair2 as *const RbLenT)) };
    pair_byte_cmp(a, b)
}

/// Mirror of re.c's
///
/// ```c
/// struct backref_name_tag {
///     const UChar *name;
///     rb_len_t len;
/// };
/// ```
///
/// The layout is asserted below and, against the C definition, by the
/// `STATIC_ASSERT`s in the `#if USE_RUST_PORTS` branch of re.c.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackrefNameTag {
    pub name: *const u8,
    pub len: RbLenT,
}

const _: () = {
    assert!(core::mem::offset_of!(BackrefNameTag, name) == 0);
    assert!(core::mem::offset_of!(BackrefNameTag, len) == core::mem::size_of::<*const u8>());
    assert!(
        core::mem::size_of::<BackrefNameTag>() == core::mem::size_of::<*const u8>() + core::mem::size_of::<RbLenT>()
    );
};

/// re.c's `match_inspect_name_iter`: for every group number in `back_refs`,
/// records the group name (`name`, and its length `name_end - name`) in
/// `arg[group]`.  The name pointers are only stored, never read.  Returns 0
/// (continue iterating).
pub fn match_inspect_name_iter(
    name: *const u8,
    name_end: *const u8,
    back_refs: &[c_int],
    arg: &mut [BackrefNameTag],
) -> c_int {
    // `name_end - name`: both point into the same name, so the address
    // difference is the C pointer difference (UChar is one byte).
    let len = (name_end as usize).wrapping_sub(name as usize) as isize as RbLenT;
    for &group in back_refs {
        arg[group as usize] = BackrefNameTag { name, len };
    }
    0
}

/// C boundary for re.c's `static int match_inspect_name_iter(const OnigUChar
/// *name, const OnigUChar *name_end, int back_num, int *back_refs, OnigRegex
/// regex, void *arg0)`, the `onig_foreach_name` callback of `match_inspect`,
/// reached through
/// `#define match_inspect_name_iter rb_core_re_match_inspect_name_iter`.
/// `regex` is unused, as in C.
///
/// # Safety
///
/// When `back_num > 0`, `back_refs` must be readable for `back_num` ints,
/// each a group number `0 <= g`, and `arg0` must point to an array of
/// `struct backref_name_tag` with more than `g` elements for every such `g`.
/// (Onigmo passes group numbers in `1..num_regs` and `match_inspect`
/// allocates `num_regs` elements.)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_re_match_inspect_name_iter(
    name: *const u8,
    name_end: *const u8,
    back_num: c_int,
    back_refs: *mut c_int,
    _regex: *mut c_void,
    arg0: *mut c_void,
) -> c_int {
    if back_num <= 0 {
        // The C loop body does not run; nothing is read or written.
        return 0;
    }
    // SAFETY: readable for back_num ints per the contract.
    let back_refs = unsafe { core::slice::from_raw_parts(back_refs as *const c_int, back_num as usize) };
    // The C does not know the array's length; view exactly the prefix that
    // the group numbers reach (each is >= 0 per the contract).
    let n = back_refs.iter().fold(0usize, |m, &g| m.max((g as usize).saturating_add(1)));
    // SAFETY: arg0 points to more than every group number elements per the
    // contract, so its first `n` elements are valid and writable.
    let arg = unsafe { core::slice::from_raw_parts_mut(arg0 as *mut BackrefNameTag, n) };
    match_inspect_name_iter(name, name_end, back_refs, arg)
}

/// Copies `msg` (which ends in its NUL) into `err` exactly as re.c's `errcpy`
/// (`strlcpy` with the buffer size) does for messages shorter than the
/// buffer: the message and its NUL, nothing after it.
fn errcpy(err: &mut [u8; ONIG_MAX_ERROR_MESSAGE_LEN], msg: &[u8]) {
    err[..msg.len()].copy_from_slice(msg);
}

/// Safe core of re.c's `read_escaped_byte` over the bytes `[*pp, end)`.
///
/// Reads one escaped byte (`\\`, `\n`, `\t`, `\r`, `\f`, `\v`, `\a`, `\e`,
/// `\OOO`, `\xHH`, `\M-X`, `\C-X`, `\cX` and their combinations) at the start
/// of `s`.  On success returns the byte's value and how many bytes were
/// consumed (the C advances `*pp` by that much).  On failure returns the
/// message the C copies into `err` with `errcpy`, NUL included; the C then
/// returns -1 and leaves `*pp` alone.
///
/// Like the C, it never reads at or past `end`: `\OOO` and `\xHH` scan at
/// most `end - p` digits, `\M-` needs two more bytes, `\C-` and `\c` one.
pub fn read_escaped_byte(s: &[u8]) -> Result<(c_int, usize), &'static [u8]> {
    let n = s.len();
    let mut i = 0;
    let mut meta_prefix = false;
    let mut ctrl_prefix = false;

    if i == n || s[i] != b'\\' {
        return Err(b"too short escaped multibyte character\0");
    }
    i += 1;

    // `again:` in the C.
    let mut code: c_int = loop {
        if i == n {
            return Err(b"too short escape sequence\0");
        }
        let c = s[i];
        i += 1;
        match c {
            b'\\' => break b'\\' as c_int,
            b'n' => break b'\n' as c_int,
            b't' => break b'\t' as c_int,
            b'r' => break b'\r' as c_int,
            b'f' => break 0x0c,
            b'v' => break 0x0b,
            b'a' => break 0x07,
            b'e' => break 0x1b,

            // \OOO
            b'0'..=b'7' => {
                i -= 1;
                let len = if n - i < 3 { n - i } else { 3 };
                // scan_oct(): (int)ruby_scan_oct(p, len, &len).
                let scan = crate::util::scan_digits(s[i..i + len].iter().copied(), 8);
                i += scan.consumed;
                break scan.value as c_int;
            }

            // \xHH
            b'x' => {
                let len = if n - i < 2 { n - i } else { 2 };
                // scan_hex(): (int)ruby_scan_hex(p, len, &len).
                let scan = crate::util::scan_digits(s[i..i + len].iter().copied(), 16);
                if scan.consumed < 1 {
                    return Err(b"invalid hex escape\0");
                }
                i += scan.consumed;
                break scan.value as c_int;
            }

            // \M-X, \M-\C-X, \M-\cX
            b'M' => {
                if meta_prefix {
                    return Err(b"duplicate meta escape\0");
                }
                meta_prefix = true;
                // `p+1 < end && *p++ == '-' && (*p & 0x80) == 0`
                if i + 1 < n {
                    let dash = s[i];
                    i += 1;
                    if dash == b'-' && (s[i] & 0x80) == 0 {
                        if s[i] == b'\\' {
                            i += 1;
                            continue;
                        }
                        // `code = *p++`: the high bit is clear, so the value
                        // is the same whether char is signed or not.
                        let v = s[i] as c_int;
                        i += 1;
                        break v;
                    }
                }
                return Err(b"too short meta escape\0");
            }

            // \C-X, \C-\M-X, and (falling through) \cX, \c\M-X
            b'C' | b'c' => {
                if c == b'C' {
                    // `p == end || *p++ != '-'`
                    if i == n {
                        return Err(b"too short control escape\0");
                    }
                    let dash = s[i];
                    i += 1;
                    if dash != b'-' {
                        return Err(b"too short control escape\0");
                    }
                }
                if ctrl_prefix {
                    return Err(b"duplicate control escape\0");
                }
                ctrl_prefix = true;
                if i < n && (s[i] & 0x80) == 0 {
                    if s[i] == b'\\' {
                        i += 1;
                        continue;
                    }
                    let v = s[i] as c_int;
                    i += 1;
                    break v;
                }
                return Err(b"too short control escape\0");
            }

            _ => return Err(b"unexpected escape sequence\0"),
        }
    };

    if code < 0 || 0xff < code {
        return Err(b"invalid escape code\0");
    }
    if ctrl_prefix {
        code &= 0x1f;
    }
    if meta_prefix {
        code |= 0x80;
    }
    Ok((code, i))
}

/// C boundary for re.c's `static int read_escaped_byte(const char **pp,
/// const char *end, onig_errmsg_buffer err)`, called by
/// `unescape_escaped_nonascii` and `unescape_nonascii0` through
/// `#define read_escaped_byte rb_core_re_read_escaped_byte`.
///
/// Returns the byte and advances `*pp` past the escape, or returns -1 after
/// copying a message into `err` (then `*pp` is unchanged), exactly as C.
///
/// # Safety
///
/// `pp` must be readable and writable, `*pp <= end`, the bytes `[*pp, end)`
/// readable, and `err` must point to a writable `onig_errmsg_buffer`
/// (`char[ONIG_MAX_ERROR_MESSAGE_LEN]`), as every caller in re.c passes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_re_read_escaped_byte(
    pp: *mut *const c_char,
    end: *const c_char,
    err: *mut c_char,
) -> c_int {
    // SAFETY: `pp` is readable per the contract.
    let p = unsafe { *pp };
    // `*pp <= end` per the contract, so this is `end - p`.
    let len = (end as usize).wrapping_sub(p as usize);
    let s: &[u8] = if len == 0 {
        &[]
    } else {
        // SAFETY: `[p, end)` is readable per the contract.
        unsafe { core::slice::from_raw_parts(p as *const u8, len) }
    };
    match read_escaped_byte(s) {
        Ok((code, used)) => {
            // SAFETY: `used <= len`, so `p + used` stays within `[p, end]`;
            // `pp` is writable per the contract.
            unsafe { *pp = p.add(used) };
            code
        }
        Err(msg) => {
            // SAFETY: per the contract `err` is a writable buffer of
            // ONIG_MAX_ERROR_MESSAGE_LEN bytes, so it can be viewed as that
            // array.
            let err = unsafe { &mut *(err as *mut [u8; ONIG_MAX_ERROR_MESSAGE_LEN]) };
            errcpy(err, msg);
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(x: &[u8], y: &[u8]) -> c_long {
        y.windows(x.len()).position(|w| w == x).map_or(-1, |p| p as c_long)
    }

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *seed >> 33
    }

    #[test]
    fn memcicmp_folds_ascii_only() {
        assert_eq!(memcicmp(b"Hello", b"hELLO"), 0);
        assert_eq!(memcicmp(b"a", b"b"), -1);
        assert_eq!(memcicmp(b"[", b"{"), b'[' as c_int - b'{' as c_int);
        // Non-ASCII bytes are not folded; they compare as C `char` values.
        let (c9, e9) = ("\u{c9}".as_bytes()[1], "\u{e9}".as_bytes()[1]);
        assert_eq!(memcicmp("\u{c9}".as_bytes(), "\u{e9}".as_bytes()), c9 as c_char as c_int - e9 as c_char as c_int);
        assert_eq!(memcicmp(b"\xff", b"a"), 0xffu8 as c_char as c_int - b'a' as c_int);
    }

    #[test]
    fn single_byte_searches_find_first_occurrence() {
        let mut seed = 42;
        for _ in 0..20000 {
            let n = 3 + (lcg(&mut seed) % 40) as usize;
            let m = 2 + (lcg(&mut seed) % (n as u64 - 1)) as usize;
            let m = m.min(n - 1);
            let alpha = b"abAB\0\xff";
            let y: Vec<u8> = (0..n).map(|_| alpha[(lcg(&mut seed) % 6) as usize]).collect();
            let x: Vec<u8> = if lcg(&mut seed) % 2 == 0 {
                let s = (lcg(&mut seed) % (n - m + 1) as u64) as usize;
                y[s..s + m].to_vec()
            } else {
                (0..m).map(|_| alpha[(lcg(&mut seed) % 6) as usize]).collect()
            };
            let want = naive(&x, &y);
            assert_eq!(search_qs(&x, &y), want);
            if m <= VALUE_BYTES {
                assert_eq!(search_ss_rolling(&x, &y), want, "rolling {x:?} in {y:?}");
            }
        }
    }

    #[test]
    fn utf8_search_matches_naive_on_valid_utf8() {
        let words = ["a", "\u{e9}", "\u{3042}", "\u{1F600}", "b", "\u{e8}"];
        let mut seed = 7;
        for _ in 0..20000 {
            let mut y = String::new();
            for _ in 0..(1 + lcg(&mut seed) % 12) {
                y.push_str(words[(lcg(&mut seed) % 6) as usize]);
            }
            let mut x = String::new();
            for _ in 0..(1 + lcg(&mut seed) % 4) {
                x.push_str(words[(lcg(&mut seed) % 6) as usize]);
            }
            if x.len() >= y.len() {
                continue;
            }
            assert_eq!(search_qs_utf8(x.as_bytes(), y.as_bytes()), naive(x.as_bytes(), y.as_bytes()));
        }
    }

    #[test]
    fn char_size_search_is_aligned() {
        // UTF-16LE "a b a" = 61 00 62 00 61 00: "b" is found at offset 2.
        assert_eq!(search_char_size(&[0x62, 0x00], &[0x61, 0x00, 0x62, 0x00, 0x61, 0x00], 2), 2);
        // 61 00 occurs only at the odd offset 1, which is not a code unit start.
        assert_eq!(search_char_size(&[0x61, 0x00], &[0x00, 0x61, 0x00, 0x62, 0x00, 0x00], 2), -1);
    }

    #[test]
    fn unicode_range_rejects_surrogates_and_beyond_max() {
        let msg = b"invalid Unicode range\0";
        for &code in &[0xd800, 0xdabc, 0xdfff, 0x110000, 0x7fff_ffff, c_ulong::MAX] {
            let mut err = [0xaau8; ONIG_MAX_ERROR_MESSAGE_LEN];
            assert_eq!(check_unicode_range(code, &mut err), -1, "{code:#x}");
            assert_eq!(&err[..msg.len()], msg);
            assert!(err[msg.len()..].iter().all(|&b| b == 0xaa), "nothing written after the NUL");
        }
        for &code in &[0, 0x41, 0xd7ff, 0xe000, 0xffff, 0x10ffff] {
            let mut err = [0xaau8; ONIG_MAX_ERROR_MESSAGE_LEN];
            assert_eq!(check_unicode_range(code, &mut err), 0, "{code:#x}");
            assert!(err.iter().all(|&b| b == 0xaa), "err untouched");
        }
    }

    #[test]
    fn option_letters() {
        assert_eq!(char_to_option(b'i' as c_int), ONIG_OPTION_IGNORECASE);
        assert_eq!(char_to_option(b'x' as c_int), ONIG_OPTION_EXTEND);
        assert_eq!(char_to_option(b'm' as c_int), ONIG_OPTION_MULTILINE);
        for c in [0, b'I' as c_int, b'n' as c_int, b'o' as c_int, -105, 0x169, -1] {
            assert_eq!(char_to_option(c), 0, "{c}");
        }
        for options in -16..16 {
            let mut buf = [0xaau8; OPTBUF_SIZE];
            option_to_str(&mut buf, options);
            let mut want = [0xaau8; OPTBUF_SIZE];
            let mut p = 0;
            for (bit, ch) in [(4, b'm'), (1, b'i'), (2, b'x')] {
                if options & bit != 0 {
                    want[p] = ch;
                    p += 1;
                }
            }
            want[p] = 0;
            assert_eq!(buf, want, "{options}");
        }
    }

    #[test]
    fn option_kcode() {
        assert_eq!(char_to_option_kcode(b'n' as c_int), (32, 32, 0));
        assert_eq!(char_to_option_kcode(b'e' as c_int), (1, 16, 10));
        assert_eq!(char_to_option_kcode(b's' as c_int), (1, 16, 11));
        assert_eq!(char_to_option_kcode(b'u' as c_int), (1, 16, 1));
        assert_eq!(char_to_option_kcode(b'i' as c_int), (1, 1, -1));
        assert_eq!(char_to_option_kcode(b'x' as c_int), (2, 2, -1));
        assert_eq!(char_to_option_kcode(b'm' as c_int), (4, 4, -1));
        assert_eq!(char_to_option_kcode(b'z' as c_int), (0, 0, -1));
        let (mut opt, mut kc) = (0x55, 0x66);
        // SAFETY: two distinct writable ints.
        let ret = unsafe { rb_char_to_option_kcode(b'e' as c_int, &mut opt, &mut kc) };
        assert_eq!((ret, opt, kc), (1, 16, 10));
        // Aliased out-parameters end with the last C store, *option.
        let mut both = 0x77;
        let p: *mut c_int = &mut both;
        // SAFETY: p is writable; aliasing is allowed through raw pointers.
        let ret = unsafe { rb_char_to_option_kcode(b'n' as c_int, p, p) };
        assert_eq!((ret, both), (32, 32));
    }

    #[test]
    fn pair_cmp_sign() {
        assert_eq!(pair_byte_cmp(0, 0), 0);
        assert_eq!(pair_byte_cmp(5, 3), 1);
        assert_eq!(pair_byte_cmp(3, 5), -1);
        assert_eq!(pair_byte_cmp(c_long::MAX, 0), 1);
        assert_eq!(pair_byte_cmp(0, c_long::MAX), -1);
        // A difference that does not fit in int still has the right sign.
        assert_eq!(pair_byte_cmp(1 << 40, 0), 1);
        let a: [c_long; 2] = [7, 99];
        let b: [c_long; 2] = [9, -1];
        // SAFETY: both point to two readable longs, like a pair_t.
        let r = unsafe { rb_core_re_pair_byte_cmp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void) };
        assert_eq!(r, -1);
    }

    #[test]
    fn name_iter_records_names() {
        let name = b"foobar";
        let start = name.as_ptr();
        // SAFETY: in bounds of `name`.
        let end = unsafe { start.add(3) };
        let blank = BackrefNameTag { name: core::ptr::null(), len: -7 };
        let mut arg = [blank; 5];
        assert_eq!(match_inspect_name_iter(start, end, &[1, 3], &mut arg), 0);
        assert_eq!(arg[0], blank);
        assert_eq!(arg[1], BackrefNameTag { name: start, len: 3 });
        assert_eq!(arg[2], blank);
        assert_eq!(arg[3], BackrefNameTag { name: start, len: 3 });
        assert_eq!(arg[4], blank);
        let mut refs = [4, 2];
        // SAFETY: refs readable for 2 ints, all < arg.len().
        let r = unsafe {
            rb_core_re_match_inspect_name_iter(
                end,
                end,
                2,
                refs.as_mut_ptr(),
                core::ptr::null_mut(),
                arg.as_mut_ptr() as *mut c_void,
            )
        };
        assert_eq!(r, 0);
        assert_eq!(arg[2], BackrefNameTag { name: end, len: 0 });
        assert_eq!(arg[4], BackrefNameTag { name: end, len: 0 });
        assert_eq!(arg[3], BackrefNameTag { name: start, len: 3 });
    }

    #[test]
    fn escaped_bytes() {
        let ok = |s: &[u8]| read_escaped_byte(s).unwrap();
        let err = |s: &[u8]| core::str::from_utf8(read_escaped_byte(s).unwrap_err()).unwrap().trim_end_matches('\0').to_owned();
        assert_eq!(ok(b"\\\\"), (0x5c, 2));
        assert_eq!(ok(b"\\n"), (0x0a, 2));
        assert_eq!(ok(b"\\t"), (0x09, 2));
        assert_eq!(ok(b"\\r"), (0x0d, 2));
        assert_eq!(ok(b"\\f"), (0x0c, 2));
        assert_eq!(ok(b"\\v"), (0x0b, 2));
        assert_eq!(ok(b"\\a"), (0x07, 2));
        assert_eq!(ok(b"\\e"), (0x1b, 2));
        assert_eq!(ok(b"\\101x"), (0x41, 4));
        assert_eq!(ok(b"\\7"), (7, 2));
        assert_eq!(ok(b"\\0778"), (0o77, 4));
        assert_eq!(ok(b"\\377"), (0xff, 4));
        assert_eq!(err(b"\\400"), "invalid escape code");
        assert_eq!(ok(b"\\x4"), (4, 3));
        assert_eq!(ok(b"\\xfF0"), (0xff, 4));
        assert_eq!(err(b"\\xg"), "invalid hex escape");
        assert_eq!(err(b"\\x"), "invalid hex escape");
        assert_eq!(ok(b"\\M-a"), (0xe1, 4));
        assert_eq!(ok(b"\\M-\\C-a"), (0x81, 7));
        assert_eq!(ok(b"\\M-\\ca"), (0x81, 6));
        assert_eq!(ok(b"\\C-a"), (0x01, 4));
        assert_eq!(ok(b"\\ca"), (0x01, 3));
        assert_eq!(ok(b"\\c\\M-a"), (0x81, 6));
        assert_eq!(ok(b"\\M-\\n"), (0x8a, 5));
        assert_eq!(err(b""), "too short escaped multibyte character");
        assert_eq!(err(b"a"), "too short escaped multibyte character");
        assert_eq!(err(b"\\"), "too short escape sequence");
        assert_eq!(err(b"\\M-\\"), "too short escape sequence");
        assert_eq!(err(b"\\M-"), "too short meta escape");
        assert_eq!(err(b"\\Mxa"), "too short meta escape");
        assert_eq!(err(b"\\M-\xe9"), "too short meta escape");
        assert_eq!(err(b"\\M-\\M-a"), "duplicate meta escape");
        assert_eq!(err(b"\\C"), "too short control escape");
        assert_eq!(err(b"\\Cxa"), "too short control escape");
        assert_eq!(err(b"\\c"), "too short control escape");
        assert_eq!(err(b"\\c\xe9"), "too short control escape");
        assert_eq!(err(b"\\c\\ca"), "duplicate control escape");
        assert_eq!(err(b"\\C-\\C-a"), "duplicate control escape");
        assert_eq!(err(b"\\8"), "unexpected escape sequence");
        assert_eq!(err(b"\\\xff"), "unexpected escape sequence");

        // Through the C boundary: *pp advances on success, err on failure.
        let src = b"\\x41rest";
        let mut p = src.as_ptr() as *const c_char;
        let mut buf = [0xaau8; ONIG_MAX_ERROR_MESSAGE_LEN];
        // SAFETY: [p, end) is `src`; buf is an error buffer.
        let r = unsafe {
            rb_core_re_read_escaped_byte(&mut p, src.as_ptr().add(src.len()) as *const c_char, buf.as_mut_ptr() as *mut c_char)
        };
        assert_eq!(r, 0x41);
        assert_eq!(p as usize - src.as_ptr() as usize, 4);
        assert!(buf.iter().all(|&b| b == 0xaa));
        let mut q = src.as_ptr().wrapping_add(4) as *const c_char;
        // SAFETY: as above.
        let r = unsafe {
            rb_core_re_read_escaped_byte(&mut q, src.as_ptr().add(src.len()) as *const c_char, buf.as_mut_ptr() as *mut c_char)
        };
        assert_eq!(r, -1);
        assert_eq!(q as usize - src.as_ptr() as usize, 4);
        let msg = b"too short escaped multibyte character\0";
        assert_eq!(&buf[..msg.len()], msg);
        assert!(buf[msg.len()..].iter().all(|&b| b == 0xaa));
    }
}
