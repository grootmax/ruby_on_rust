//! Rust ports of digit-array primitives of bignum.c (port unit
//! `bignum-A-04`): addition (`bary_addc`, `bary_add`, `bary_add_one`),
//! multiply-and-subtract (`bigdivrem_mulsub`, `bary_mulsub_1xN`), squaring
//! (`bary_sq_fast`), `max_size`, and the schoolbook division loop
//! `bigdivrem1`.
//!
//! All of them except `bigdivrem_mulsub` are `static` in bignum.c and still
//! called from its unported C, so they are exported as
//! `rb_core_bignum_<name>` (hidden, see internal/core_rs.h).
//! `bigdivrem_mulsub` is only called by `bary_mulsub_1xN` and `bigdivrem1`
//! and stays private.
//!
//! Digit types: `BDIGIT` is `unsigned int`, `BDIGIT_DBL` is
//! `unsigned long long` and `BDIGIT_DBL_SIGNED` is `long long`, the first
//! configuration of internal/bignum.h, which every target core_rs supports
//! selects (`SIZEOF_INT*2 <= SIZEOF_LONG_LONG`).  bignum.c asserts this in
//! its `#if USE_RUST_PORTS` branch.
//!
//! Aliasing: the C callers pass overlapping arrays (e.g.
//! `bary_add(z, n, z, n, w, n)` adds in place).  The safe cores therefore
//! take `&[Cell<BDIGIT>]`, which may alias freely, and read and write the
//! elements in the same order as the C, so every overlap behaves exactly as
//! it does in C.  Unsigned C arithmetic wraps, so the ports use `wrapping_*`
//! throughout; where the C uses signed arithmetic (`BDIGIT_DBL_SIGNED`) the
//! values are bounded far below overflow, and `wrapping_*` only keeps an
//! impossible overflow from panicking.

use core::cell::Cell;
use core::ffi::{c_int, c_void};

use crate::ffi::value::VALUE;

/// `BDIGIT`.
#[allow(non_camel_case_types)]
pub type BDIGIT = u32;
/// `BDIGIT_DBL`.
type BdigitDbl = u64;
/// `BDIGIT_DBL_SIGNED`.
type BdigitDblSigned = i64;

/// `BITSPERDIG` (`SIZEOF_BDIGIT*CHAR_BIT`).
const BITSPERDIG: u32 = 32;
/// `BDIGMAX`.
const BDIGMAX: BDIGIT = BDIGIT::MAX;

/// `BIGLO(x)`: `(BDIGIT)((x) & BDIGMAX)`.
#[inline(always)]
fn biglo(x: BdigitDbl) -> BDIGIT {
    x as BDIGIT
}

/// `BIGLO(x)` for a `BDIGIT_DBL_SIGNED`: the low 32 bits.
#[inline(always)]
fn biglo_signed(x: BdigitDblSigned) -> BDIGIT {
    x as BDIGIT
}

/// `BIGDN(x)`: `RSHIFT(x, BITSPERDIG)`, a logical shift for `BDIGIT_DBL`.
#[inline(always)]
fn bigdn(x: BdigitDbl) -> BdigitDbl {
    x >> BITSPERDIG
}

/// `BIGDN(x)` for a `BDIGIT_DBL_SIGNED`: `RSHIFT` is `>>`, an arithmetic
/// shift of a signed value with the compilers CRuby supports.
#[inline(always)]
fn bigdn_signed(x: BdigitDblSigned) -> BdigitDblSigned {
    x >> BITSPERDIG
}

/// `bary_addc`: `z = x + y + (carry ? 1 : 0)`, with `zn = z.len()`,
/// `xn = x.len()`, `yn = y.len()`.  Returns the carry out (0 or 1).
///
/// Precondition (C `RUBY_ASSERT`s): `x.len() <= z.len()` and
/// `y.len() <= z.len()`.  The slices may overlap.
pub fn bary_addc_impl(z: &[Cell<BDIGIT>], x: &[Cell<BDIGIT>], y: &[Cell<BDIGIT>], carry: c_int) -> c_int {
    let zn = z.len();
    // if (xn > yn) swap x and y, so that x is the shorter one.
    let (x, y) = if x.len() > y.len() { (y, x) } else { (x, y) };
    let xn = x.len();
    let yn = y.len();

    // The C writes z[0..yn) first; yn <= zn is a precondition.  Slicing
    // up front lets the loops below run without bounds checks.
    let zy = &z[..yn];

    let mut num: BdigitDbl = if carry != 0 { 1 } else { 0 };
    for ((zd, xd), yd) in zy[..xn].iter().zip(x).zip(&y[..xn]) {
        num = num.wrapping_add((xd.get() as BdigitDbl).wrapping_add(yd.get() as BdigitDbl));
        zd.set(biglo(num));
        num = bigdn(num);
    }
    let mut i = xn;
    'num_is_zero: {
        while i < yn {
            if num == 0 {
                break 'num_is_zero;
            }
            num = num.wrapping_add(y[i].get() as BdigitDbl);
            zy[i].set(biglo(num));
            num = bigdn(num);
            i += 1;
        }
        while i < zn {
            if num == 0 {
                break 'num_is_zero;
            }
            z[i].set(biglo(num));
            num = bigdn(num);
            i += 1;
        }
        return (num != 0) as c_int;
    }

    // num_is_zero:
    if core::ptr::eq(y.as_ptr(), z.as_ptr()) && yn == zn {
        return 0;
    }
    while i < yn {
        zy[i].set(y[i].get());
        i += 1;
    }
    while i < zn {
        z[i].set(0);
        i += 1;
    }
    0
}

/// `bary_add`: `bary_addc` without a carry in.
pub fn bary_add_impl(z: &[Cell<BDIGIT>], x: &[Cell<BDIGIT>], y: &[Cell<BDIGIT>]) -> c_int {
    bary_addc_impl(z, x, y, 0)
}

/// `bary_add_one`: increments `ds` in place.  Returns 1 if the increment
/// carried out of the last digit (every digit was `BDIGMAX`, and all are now
/// 0), else 0.
pub fn bary_add_one_impl(ds: &mut [BDIGIT]) -> c_int {
    for d in ds.iter_mut() {
        let n: BdigitDbl = (*d as BdigitDbl) + 1;
        *d = biglo(n);
        if *d != 0 {
            return 0;
        }
    }
    1
}

/// `bigdivrem_mulsub`: `z[0..yn) -= x * y`, propagating the borrow, where
/// `yn = y.len()`.  `z[yn]` is read but not written.  Returns the final
/// borrow plus `z[yn]` (a `BDIGIT_DBL_SIGNED`).
///
/// Preconditions: `z.len() >= y.len() + 1` (C `RUBY_ASSERT(zn == yn + 1)`)
/// and `y.len() >= 1`: the C loop is a `do`/`while` that reads `yds[0]` and
/// `zds[0]` even when `yn == 0`, which would read outside `yds`.  Both
/// callers pass `yn >= 1` (`bigdivrem1` gets `yn >= 2` from
/// `bigdivrem_restoring`; Toom-3 passes the non-empty `t4`).
fn bigdivrem_mulsub_impl(z: &[Cell<BDIGIT>], x: BDIGIT, y: &[Cell<BDIGIT>]) -> BdigitDblSigned {
    let yn = y.len();
    // The C reads z[0..=yn]; slicing up front removes the bounds checks.
    let z = &z[..yn + 1];
    let mut num: BdigitDblSigned = 0;
    let mut t2: BdigitDbl = 0;
    let mut i = 0;

    loop {
        t2 = t2.wrapping_add((y[i].get() as BdigitDbl).wrapping_mul(x as BdigitDbl));
        // BIGLO(t2) is an unsigned int, converted to long long.
        let ee: BdigitDblSigned = num.wrapping_sub(biglo(t2) as BdigitDblSigned);
        num = (z[i].get() as BdigitDblSigned).wrapping_add(ee);
        if ee != 0 {
            z[i].set(biglo_signed(num));
        }
        num = bigdn_signed(num);
        t2 = bigdn(t2);
        i += 1;
        if i >= yn {
            break;
        }
    }
    num = num.wrapping_sub(t2 as BdigitDblSigned);
    num = num.wrapping_add(z[yn].get() as BdigitDblSigned); /* borrow from high digit; don't update */
    num
}

/// `bary_mulsub_1xN`: `z[0..=yn] -= x * y` with `yn = y.len()`.  Returns 1
/// if the result is negative (a borrow out of `z[yn]`), else 0.
///
/// Preconditions: as for `bigdivrem_mulsub` (`z.len() >= y.len() + 1`,
/// `y.len() >= 1`).
pub fn bary_mulsub_1xn_impl(z: &[Cell<BDIGIT>], x: BDIGIT, y: &[Cell<BDIGIT>]) -> c_int {
    let yn = y.len();
    let num = bigdivrem_mulsub_impl(z, x, y);
    z[yn].set(biglo_signed(num));
    if bigdn_signed(num) != 0 {
        return 1;
    }
    0
}

/// `bary_sq_fast`: `z = x * x` (Handbook of Applied Cryptography,
/// Algorithm 14.16).  Zeroes all of `z` first.
///
/// Precondition (C `RUBY_ASSERT`): `2 * x.len() <= z.len()`.  The C callers
/// never pass overlapping `z` and `x`; if they did, the result would be the
/// same as in C, since the slices are read and written in the same order.
pub fn bary_sq_fast_impl(z: &[Cell<BDIGIT>], x: &[Cell<BDIGIT>]) {
    let xn = x.len();

    // BDIGITS_ZERO(zds, zn);
    for d in z.iter() {
        d.set(0);
    }

    if xn == 0 {
        return;
    }

    let mut i = 0;
    while i < xn - 1 {
        let mut v: BdigitDbl = x[i].get() as BdigitDbl;
        if v == 0 {
            i += 1;
            continue;
        }
        let mut c: BdigitDbl = (z[i + i].get() as BdigitDbl).wrapping_add(v.wrapping_mul(v));
        z[i + i].set(biglo(c));
        c = bigdn(c);
        v = v.wrapping_mul(2);
        let vl: BDIGIT = biglo(v);
        let vh: c_int = bigdn(v) as c_int;
        // for (j = i + 1; j < xn; j++): zds[i + j] with xds[j].
        for (zd, xd) in z[i + i + 1..i + xn].iter().zip(&x[i + 1..xn]) {
            let w: BdigitDbl = xd.get() as BdigitDbl;
            c = c.wrapping_add((zd.get() as BdigitDbl).wrapping_add((vl as BdigitDbl).wrapping_mul(w)));
            zd.set(biglo(c));
            c = bigdn(c);
            if vh != 0 {
                c = c.wrapping_add(w);
            }
        }
        if c != 0 {
            c = c.wrapping_add(z[i + xn].get() as BdigitDbl);
            z[i + xn].set(biglo(c));
            c = bigdn(c);
            if c != 0 {
                z[i + xn + 1].set(z[i + xn + 1].get().wrapping_add(c as BDIGIT));
            }
        }
        i += 1;
    }

    /* i == xn-1 */
    let v: BdigitDbl = x[i].get() as BdigitDbl;
    if v == 0 {
        return;
    }
    let mut c: BdigitDbl = (z[i + i].get() as BdigitDbl).wrapping_add(v.wrapping_mul(v));
    z[i + i].set(biglo(c));
    c = bigdn(c);
    if c != 0 {
        z[i + xn].set(z[i + xn].get().wrapping_add(biglo(c)));
    }
}

/// `max_size`.
#[inline]
pub fn max_size_impl(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}

/// The loop of `bigdivrem1`: divides `z` (`zn = z.len()` digits) by `y`
/// (`yn = y.len()` digits) in place, one quotient digit per step from the
/// top, until `zn == yn`.  Each quotient digit `q` is stored in `z[zn-1]`
/// as `zn` shrinks; the remainder is left in `z[0..yn)`.
///
/// `stopped()` is evaluated at the start of every step, where the C reads
/// `bds->stop`.  If it returns true, returns `Some(zn)` with the current
/// `zn` (which the C stores into `bds->zn`); otherwise `None` once done.
///
/// Preconditions (`bigdivrem_restoring` establishes them): `y.len() >= 1`,
/// `z.len() > y.len()` (the C's `do`/`while` runs one step before testing
/// `zn > yn`), and `y[yn-1] != 0` (normalized: its top bit is set).  With
/// `y[yn-1] == 0` the C divides by zero; the port then panics (rb_bug).
pub fn bigdivrem1_impl<F: FnMut() -> bool>(z: &[Cell<BDIGIT>], y: &[Cell<BDIGIT>], mut stopped: F) -> Option<usize> {
    let yn = y.len();
    let mut zn = z.len();
    loop {
        if stopped() {
            return Some(zn);
        }
        let mut q: BDIGIT = if z[zn - 1].get() == y[yn - 1].get() {
            BDIGMAX
        }
        else {
            // (BDIGIT)((BIGUP(zds[zn-1]) + zds[zn-2])/yds[yn-1])
            let n: BdigitDbl = ((z[zn - 1].get() as BdigitDbl) << BITSPERDIG).wrapping_add(z[zn - 2].get() as BdigitDbl);
            (n / (y[yn - 1].get() as BdigitDbl)) as BDIGIT
        };
        if q != 0 {
            let part = &z[zn - (yn + 1)..zn];
            let mut num: BdigitDblSigned = bigdivrem_mulsub_impl(part, q, y);
            while num != 0 {
                /* "add back" required */
                q = q.wrapping_sub(1);
                num = bary_add_impl(&part[..yn], &part[..yn], y) as BdigitDblSigned;
                num = num.wrapping_sub(1);
            }
        }
        zn -= 1;
        z[zn].set(q);
        if zn <= yn {
            break;
        }
    }
    None
}

/// `struct big_div_struct` from bignum.c, field for field:
///
/// ```c
/// struct big_div_struct {
///     size_t yn, zn;
///     BDIGIT *yds, *zds;
///     volatile VALUE stop;
/// };
/// ```
///
/// The layout is asserted below and, against the C definition, by the
/// `STATIC_ASSERT`s in the `#if USE_RUST_PORTS` branch of bignum.c.
#[repr(C)]
pub struct BigDivStruct {
    pub yn: usize,
    pub zn: usize,
    pub yds: *mut BDIGIT,
    pub zds: *mut BDIGIT,
    /// `volatile`: written by `rb_big_stop()` from another thread or a
    /// signal handler while `bigdivrem1` runs without the GVL.
    pub stop: VALUE,
}

const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(offset_of!(BigDivStruct, yn) == 0);
    assert!(offset_of!(BigDivStruct, zn) == size_of::<usize>());
    assert!(offset_of!(BigDivStruct, yds) == 2 * size_of::<usize>());
    assert!(offset_of!(BigDivStruct, zds) == 2 * size_of::<usize>() + size_of::<*mut BDIGIT>());
    assert!(offset_of!(BigDivStruct, stop) == 2 * size_of::<usize>() + 2 * size_of::<*mut BDIGIT>());
    assert!(size_of::<BigDivStruct>() == 2 * size_of::<usize>() + 2 * size_of::<*mut BDIGIT>() + size_of::<VALUE>());
};

/// `&[Cell<BDIGIT>]` view of `n` digits at `p`.  An empty slice for
/// `n == 0`, so that `p` may then be anything (even NULL).
///
/// # Safety
/// For `n > 0`, `p` must point to `n` valid digits that stay valid for
/// `'a` and are only accessed through such views (or not at all) meanwhile.
#[inline(always)]
unsafe fn cells<'a>(p: *const BDIGIT, n: usize) -> &'a [Cell<BDIGIT>] {
    if n == 0 {
        return &[];
    }
    // SAFETY: the caller guarantees `n` valid digits at `p`; Cell<u32> has
    // the same layout as u32, and shared Cell views may alias each other.
    unsafe { core::slice::from_raw_parts(p as *const Cell<BDIGIT>, n) }
}

/// Exported `bary_addc` for bignum.c.
///
/// # Safety
/// `zds` must point to `zn` writable digits, `xds` to `xn` and `yds` to `yn`
/// readable digits (C precondition: `xn <= zn`, `yn <= zn`).  The arrays
/// may overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_bignum_bary_addc(
    zds: *mut BDIGIT,
    zn: usize,
    xds: *const BDIGIT,
    xn: usize,
    yds: *const BDIGIT,
    yn: usize,
    carry: c_int,
) -> c_int {
    // SAFETY: the caller passes valid arrays of the given lengths; the C
    // writes only through zds.
    let (z, x, y) = unsafe { (cells(zds, zn), cells(xds, xn), cells(yds, yn)) };
    bary_addc_impl(z, x, y, carry)
}

/// Exported `bary_add` for bignum.c.
///
/// # Safety
/// As for [`rb_core_bignum_bary_addc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_bignum_bary_add(
    zds: *mut BDIGIT,
    zn: usize,
    xds: *const BDIGIT,
    xn: usize,
    yds: *const BDIGIT,
    yn: usize,
) -> c_int {
    // SAFETY: the caller passes valid arrays of the given lengths.
    let (z, x, y) = unsafe { (cells(zds, zn), cells(xds, xn), cells(yds, yn)) };
    bary_add_impl(z, x, y)
}

/// Exported `bary_add_one` for bignum.c.
///
/// # Safety
/// `ds` must point to `n` writable digits that nothing else accesses during
/// the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_bignum_bary_add_one(ds: *mut BDIGIT, n: usize) -> c_int {
    if n == 0 {
        return bary_add_one_impl(&mut []);
    }
    // SAFETY: the caller passes `n` valid, exclusively owned digits.
    let ds = unsafe { core::slice::from_raw_parts_mut(ds, n) };
    bary_add_one_impl(ds)
}

/// Exported `bary_mulsub_1xN` for bignum.c.
///
/// # Safety
/// `zds` must point to `yn + 1` writable digits and `yds` to `yn >= 1`
/// readable digits (C precondition `zn == yn + 1`; the C touches
/// `zds[0..=yn]` and ignores `zn` otherwise, and reads `yds[0]` even for
/// `yn == 0`).  The arrays may overlap.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn rb_core_bignum_bary_mulsub_1xN(
    zds: *mut BDIGIT,
    _zn: usize,
    x: BDIGIT,
    yds: *const BDIGIT,
    yn: usize,
) -> c_int {
    // SAFETY: the caller passes yn + 1 digits at zds and yn digits at yds;
    // yn + 1 cannot overflow because yn digits exist in memory.
    let (z, y) = unsafe { (cells(zds, yn + 1), cells(yds, yn)) };
    bary_mulsub_1xn_impl(z, x, y)
}

/// Exported `bary_sq_fast` for bignum.c.
///
/// # Safety
/// `zds` must point to `zn` writable digits and `xds` to `xn` readable
/// digits, with `2 * xn <= zn` (C precondition).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_bignum_bary_sq_fast(zds: *mut BDIGIT, zn: usize, xds: *const BDIGIT, xn: usize) {
    // SAFETY: the caller passes valid arrays of the given lengths.
    let (z, x) = unsafe { (cells(zds, zn), cells(xds, xn)) };
    bary_sq_fast_impl(z, x)
}

/// Exported `max_size` for bignum.c.
#[unsafe(no_mangle)]
pub extern "C" fn rb_core_bignum_max_size(a: usize, b: usize) -> usize {
    max_size_impl(a, b)
}

/// Exported `bigdivrem1` for bignum.c: the `void *(*)(void *)` that
/// `bigdivrem_restoring` calls directly or through `rb_nogvl()`.  Always
/// returns NULL.
///
/// # Safety
/// `ptr` must point to a valid `struct big_div_struct` whose `zds` points to
/// `zn` writable and `yds` to `yn` readable digits that do not overlap and
/// that nothing else accesses during the call, with the preconditions of
/// [`bigdivrem1_impl`].  Only `stop` may be written concurrently (by
/// `rb_big_stop()`); it is read with a volatile load at exactly the points
/// where the C reads it, and no reference to the struct is formed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_core_bignum_bigdivrem1(ptr: *mut c_void) -> *mut c_void {
    let bds = ptr as *mut BigDivStruct;
    // SAFETY: `bds` points to a valid struct big_div_struct; these fields
    // are not written concurrently, and the C reads them once here.
    let (yn, zn, yds, zds) = unsafe { ((*bds).yn, (*bds).zn, (*bds).yds, (*bds).zds) };
    // SAFETY: the caller guarantees zn and yn valid digits at zds and yds.
    let (z, y) = unsafe { (cells(zds, zn), cells(yds, yn)) };
    let stopped = || {
        // SAFETY: `bds` stays valid during the call; `stop` is volatile in
        // C, so it is read with a volatile load through a raw pointer.
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*bds).stop)) != 0 }
    };
    if let Some(zn) = bigdivrem1_impl(z, y, stopped) {
        // SAFETY: as above; the C stores the progress into bds->zn.
        unsafe { (*bds).zn = zn };
    }
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(v: &[u32]) -> Vec<Cell<u32>> {
        v.iter().map(|&d| Cell::new(d)).collect()
    }

    fn g(v: &[Cell<u32>]) -> Vec<u32> {
        v.iter().map(|d| d.get()).collect()
    }

    fn to_u128(v: &[u32]) -> u128 {
        v.iter().rev().fold(0u128, |acc, &d| (acc << 32) | d as u128)
    }

    /// Schoolbook product, as a reference.
    fn mul_ref(x: &[u32], y: &[u32]) -> Vec<u32> {
        let mut z = vec![0u32; x.len() + y.len()];
        for (i, &xi) in x.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &yj) in y.iter().enumerate() {
                let t = z[i + j] as u64 + xi as u64 * yj as u64 + carry;
                z[i + j] = t as u32;
                carry = t >> 32;
            }
            z[i + y.len()] = carry as u32;
        }
        z
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }
        fn digit(&mut self) -> u32 {
            match self.next() % 4 {
                0 => 0,
                1 => u32::MAX,
                _ => self.next() as u32,
            }
        }
    }

    #[test]
    fn addc_basic_and_carry() {
        let z = c(&[0; 3]);
        let x = c(&[u32::MAX, u32::MAX]);
        let y = c(&[1]);
        assert_eq!(unsafe { rb_core_bignum_bary_add(z.as_ptr() as *mut u32, 3, x.as_ptr() as *const u32, 2, y.as_ptr() as *const u32, 1) }, 0);
        assert_eq!(g(&z), [0, 0, 1]);
        // Carry out of the top digit.
        let z = c(&[9; 2]);
        assert_eq!(bary_addc_impl(&z, &x, &c(&[0]), 1), 1);
        assert_eq!(g(&z), [0, 0]);
        // num_is_zero copies the rest of the longer operand and zero-fills.
        let z = c(&[7; 4]);
        assert_eq!(bary_addc_impl(&z, &c(&[1]), &c(&[2, 3]), 0), 0);
        assert_eq!(g(&z), [3, 3, 0, 0]);
        // Swapped operands give the same result.
        let z = c(&[7; 4]);
        assert_eq!(bary_addc_impl(&z, &c(&[2, 3]), &c(&[1]), 0), 0);
        assert_eq!(g(&z), [3, 3, 0, 0]);
    }

    #[test]
    fn addc_in_place_matches_u128() {
        let mut r = Rng(1);
        for _ in 0..2000 {
            let a = [r.digit(), r.digit(), r.digit()];
            let b = [r.digit(), r.digit()];
            let z = c(&a);
            let carry = bary_add_impl(&z, &z, &c(&b));
            let sum = to_u128(&a) + to_u128(&b);
            assert_eq!(to_u128(&g(&z)), sum & ((1u128 << 96) - 1));
            assert_eq!(carry as u128, sum >> 96);
            // z aliasing y with yn == zn (the early return of num_is_zero).
            let z = c(&a);
            let carry = bary_add_impl(&z, &c(&b), &z);
            assert_eq!(to_u128(&g(&z)), sum & ((1u128 << 96) - 1));
            assert_eq!(carry as u128, sum >> 96);
        }
    }

    #[test]
    fn add_one() {
        let mut d = [u32::MAX, u32::MAX, 5];
        assert_eq!(unsafe { rb_core_bignum_bary_add_one(d.as_mut_ptr(), 3) }, 0);
        assert_eq!(d, [0, 0, 6]);
        let mut d = [u32::MAX, u32::MAX];
        assert_eq!(bary_add_one_impl(&mut d), 1);
        assert_eq!(d, [0, 0]);
        assert_eq!(unsafe { rb_core_bignum_bary_add_one(core::ptr::null_mut(), 0) }, 1);
    }

    #[test]
    fn mulsub_matches_u128() {
        let mut r = Rng(2);
        for _ in 0..2000 {
            let zv = [r.digit(), r.digit(), r.digit()];
            let yv = [r.digit(), r.digit()];
            let x = r.digit();
            let z = c(&zv);
            let neg = unsafe { rb_core_bignum_bary_mulsub_1xN(z.as_ptr() as *mut u32, 3, x, yv.as_ptr() as *const u32, 2) };
            let lhs = to_u128(&zv) as i128;
            let prod = to_u128(&yv) as i128 * x as i128;
            let diff = lhs - prod;
            assert_eq!(neg, (diff < 0) as c_int);
            assert_eq!(to_u128(&g(&z)) as i128, diff.rem_euclid(1i128 << 96));
        }
    }

    #[test]
    fn sq_fast_matches_schoolbook() {
        let mut r = Rng(3);
        for n in 0..12 {
            for _ in 0..50 {
                let xv: Vec<u32> = (0..n).map(|_| r.digit()).collect();
                let z = c(&vec![0xa5a5a5a5; 2 * n + 1]);
                unsafe { rb_core_bignum_bary_sq_fast(z.as_ptr() as *mut u32, 2 * n + 1, xv.as_ptr(), n) };
                let mut want = mul_ref(&xv, &xv);
                want.push(0);
                assert_eq!(g(&z), want);
            }
        }
        let xv = [u32::MAX; 5];
        let z = c(&[0; 10]);
        bary_sq_fast_impl(&z, &c(&xv));
        assert_eq!(g(&z), mul_ref(&xv, &xv));
    }

    #[test]
    fn max_size_values() {
        assert_eq!(rb_core_bignum_max_size(3, 5), 5);
        assert_eq!(rb_core_bignum_max_size(5, 3), 5);
        assert_eq!(rb_core_bignum_max_size(usize::MAX, 0), usize::MAX);
    }

    #[test]
    fn bigdivrem1_divides() {
        let mut r = Rng(4);
        for _ in 0..500 {
            let yn = 2 + (r.next() % 4) as usize;
            let zn = yn + 1 + (r.next() % 5) as usize;
            let mut yv: Vec<u32> = (0..yn).map(|_| r.digit()).collect();
            yv[yn - 1] |= 0x8000_0000;
            let mut zv: Vec<u32> = (0..zn).map(|_| r.digit()).collect();
            zv[zn - 1] %= yv[yn - 1];
            let mut zbuf = zv.clone();
            let mut ybuf = yv.clone();
            let mut bds = BigDivStruct { yn, zn, yds: ybuf.as_mut_ptr(), zds: zbuf.as_mut_ptr(), stop: 0 };
            let ret = unsafe { rb_core_bignum_bigdivrem1(&mut bds as *mut BigDivStruct as *mut c_void) };
            assert!(ret.is_null());
            assert_eq!(bds.zn, zn);
            assert_eq!(ybuf, yv);
            // z == q * y + r, with q = zbuf[yn..] and r = zbuf[..yn] < y.
            let (rem, quo) = zbuf.split_at(yn);
            let back = c(&mul_ref(quo, &yv));
            let carry = bary_add_impl(&back, &back, &c(rem));
            assert_eq!(carry, 0);
            assert_eq!(g(&back), zv);
            let rem_lt = rem.iter().rev().cmp(yv.iter().rev()) == core::cmp::Ordering::Less;
            assert!(rem_lt);
        }
    }

    #[test]
    fn bigdivrem1_stop() {
        let mut zbuf = [1u32, 2, 3, 4];
        let mut ybuf = [5u32, 0x8000_0000];
        let mut bds = BigDivStruct { yn: 2, zn: 4, yds: ybuf.as_mut_ptr(), zds: zbuf.as_mut_ptr(), stop: 0x14 };
        unsafe { rb_core_bignum_bigdivrem1(&mut bds as *mut BigDivStruct as *mut c_void) };
        assert_eq!(bds.zn, 4);
        assert_eq!(zbuf, [1, 2, 3, 4]);
        // Stop after the first step.
        let z = c(&[1, 2, 3, 4]);
        let mut calls = 0;
        let got = bigdivrem1_impl(&z, &c(&ybuf), || {
            calls += 1;
            calls > 1
        });
        assert_eq!(got, Some(3));
    }
}
