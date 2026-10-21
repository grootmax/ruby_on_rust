/*
 * Differential fuzz: the digit-array primitives of bignum.c (port unit
 * bignum-A-04, core_rs/src/bignum.rs) against the original C.
 *
 * FUZZ-EXTRACT: bignum.c bignum_macros:117-172 bary_addc bary_add bary_add_one bigdivrem_mulsub bary_mulsub_1xN bary_sq_fast max_size big_div_struct:2642-2646 bigdivrem1
 *
 * bignum_macros:117-172 copies the digit macros the functions use (BIGLO,
 * BIGDN, BIGUP, BDIGMAX, BDIGITS_ZERO, ...); big_div_struct:2642-2646 copies
 * struct big_div_struct (renamed struct ref_big_div_struct), which the
 * harness uses for both sides; the port mirrors it with #[repr(C)].
 *
 * Exercised through the entry points bignum.c calls: bary_addc, bary_add,
 * bary_add_one, bary_mulsub_1xN, bary_sq_fast, max_size and bigdivrem1.
 * bigdivrem_mulsub (private in the port) is reached through
 * bary_mulsub_1xN and bigdivrem1.
 *
 * The array functions work on digit ranges placed in one arena per side.
 * bignum.c calls them with overlapping arrays (bary_add(z, n, z, n, w, n)
 * adds in place, Toom-3 also passes z == y), and the C defines the result
 * of any overlap since nothing is `restrict`, so the ranges are placed both
 * disjoint and overlapping (exactly, and at random offsets).  After each
 * call every digit of the arena is compared, plus the return value.
 *
 * bigdivrem1 is given the preconditions bigdivrem_restoring establishes:
 * yn >= 1, zn > yn, a normalized divisor (top bit of yds[yn-1] set) and
 * zds[zn-1] < yds[yn-1].  Without them the C divides by zero or the
 * "add back" loop runs for up to 2^32 rounds.  `stop` is nonzero in some
 * cases (bigdivrem1 then only stores zn); a stop that arrives from another
 * thread in the middle of the loop cannot be reproduced deterministically
 * here, and the Rust reads the flag at the same point in every step.
 */
#define FUZZ_UNIT "bignum_04"
#include "ruby/ruby.h"
#include "internal/bignum.h"
#include "ruby_assert.h"
#include <limits.h>
#include <stddef.h>
#include <string.h>
#include FUZZ_REF
#include "fuzz.h"

int rs_rb_core_bignum_bary_addc(BDIGIT *zds, size_t zn, const BDIGIT *xds, size_t xn, const BDIGIT *yds, size_t yn, int carry);
int rs_rb_core_bignum_bary_add(BDIGIT *zds, size_t zn, const BDIGIT *xds, size_t xn, const BDIGIT *yds, size_t yn);
int rs_rb_core_bignum_bary_add_one(BDIGIT *ds, size_t n);
int rs_rb_core_bignum_bary_mulsub_1xN(BDIGIT *zds, size_t zn, BDIGIT x, const BDIGIT *yds, size_t yn);
void rs_rb_core_bignum_bary_sq_fast(BDIGIT *zds, size_t zn, const BDIGIT *xds, size_t xn);
size_t rs_rb_core_bignum_max_size(size_t a, size_t b);
void *rs_rb_core_bignum_bigdivrem1(void *ptr);

#define ARENA 128
#define MAXN 40

static BDIGIT c_ar[ARENA], rs_ar[ARENA];

/* Digits that reach the carry and zero paths: 0 and BDIGMAX dominate. */
static BDIGIT
digit(void)
{
    switch (fuzz_below(9)) {
      case 0: case 1: return 0;
      case 2: case 3: return 0xffffffffU;
      case 4: return 0x80000000U;
      case 5: return 0x7fffffffU;
      case 6: return (BDIGIT)fuzz_below(4);
      default: return (BDIGIT)fuzz_next();
    }
}

/* A length in [0, max], biased towards 0, 1 and max. */
static size_t
len_upto(size_t max)
{
    switch (fuzz_below(5)) {
      case 0: return 0;
      case 1: return max;
      case 2: return max < 1 ? max : 1;
      default: return (size_t)fuzz_below(max + 1);
    }
}

/* A random offset where `len` digits fit in the arena. */
static size_t
place(size_t len)
{
    return (size_t)fuzz_below(ARENA - len + 1);
}

/* Fill both arenas with the same digits. */
static void
setup_arena(void)
{
    size_t i;
    for (i = 0; i < ARENA; i++) c_ar[i] = digit();
    memcpy(rs_ar, c_ar, sizeof(c_ar));
}

/* In mutation mode, change one byte of the digits [lo, lo+n) of the Rust
 * arena (an input the call reads). */
static void
mutate_rs(size_t lo, size_t n)
{
    const unsigned char *m;
    if (n == 0) return;
    m = fuzz_rs_bytes((const unsigned char *)(c_ar + lo), n * sizeof(BDIGIT));
    memcpy(rs_ar + lo, m, n * sizeof(BDIGIT));
}

static void
compare_arena(void)
{
    size_t i;
    for (i = 0; i < ARENA; i++) {
        FUZZ_EQ("arena digit", c_ar[i], rs_ar[i]);
        if (fuzz_case_failed) break;
    }
}

/* bary_addc / bary_add: z = x + y (+ carry). */
static void
fuzz_add(int with_carry)
{
    size_t zn = len_upto(MAXN), xn = len_upto(zn), yn = len_upto(zn);
    size_t zo, xo, yo;
    int carry = 0, c_ret, rs_ret;

    zo = place(zn);
    switch (fuzz_below(7)) {
      case 0: xo = zo; yo = place(yn); break;             /* in place: z == x */
      case 1: yo = zo; xo = place(xn); break;             /* z == y (yn == zn: early return) */
      case 2: xo = yo = zo; break;                        /* z == x == y */
      case 3: xo = yo = place(xn > yn ? xn : yn); break;  /* x == y */
      case 4: zo = 0; xo = 48; yo = 88; break;            /* disjoint */
      default: xo = place(xn); yo = place(yn); break;     /* any overlap */
    }
    if (with_carry) {
        switch (fuzz_below(5)) {
          case 0: carry = 0; break;
          case 1: carry = 2; break;
          case 2: carry = -1; break;
          case 3: carry = INT_MIN; break;
          default: carry = 1; break;
        }
    }
    setup_arena();
    fuzz_input_num(with_carry ? "fn bary_addc, zn<<32|(unsigned)carry" : "fn bary_add, zn<<32",
                   (long)(((unsigned long)zn << 32) | (unsigned int)carry));
    fuzz_input_num("xn<<32|yn", (long)((xn << 32) | yn));
    fuzz_input_num("zo<<32|xo<<16|yo", (long)((zo << 32) | (xo << 16) | yo));
    fuzz_input("arena", c_ar, sizeof(c_ar));
    if (fuzz_one_in(2)) mutate_rs(xo, xn); else mutate_rs(yo, yn);

    if (with_carry) {
        c_ret = ref_bary_addc(c_ar + zo, zn, c_ar + xo, xn, c_ar + yo, yn, carry);
        rs_ret = rs_rb_core_bignum_bary_addc(rs_ar + zo, zn, rs_ar + xo, xn, rs_ar + yo, yn, carry);
    }
    else {
        c_ret = ref_bary_add(c_ar + zo, zn, c_ar + xo, xn, c_ar + yo, yn);
        rs_ret = rs_rb_core_bignum_bary_add(rs_ar + zo, zn, rs_ar + xo, xn, rs_ar + yo, yn);
    }
    FUZZ_EQ("return", c_ret, rs_ret);
    compare_arena();
}

/* bary_add_one: increment in place. */
static void
fuzz_add_one(void)
{
    size_t n = len_upto(MAXN), o = place(n), i, run;
    int c_ret, rs_ret;

    setup_arena();
    /* A run of BDIGMAX at the start, so the carry travels (and, for a
     * run of n, leaves the array). */
    run = fuzz_one_in(2) ? n : len_upto(n);
    for (i = 0; i < run; i++) c_ar[o + i] = rs_ar[o + i] = 0xffffffffU;
    fuzz_input_num("fn bary_add_one, n", (long)n);
    fuzz_input_num("offset", (long)o);
    fuzz_input("arena", c_ar, sizeof(c_ar));
    mutate_rs(o, n);

    c_ret = ref_bary_add_one(c_ar + o, n);
    rs_ret = rs_rb_core_bignum_bary_add_one(rs_ar + o, n);
    FUZZ_EQ("return", c_ret, rs_ret);
    compare_arena();
}

/* bary_mulsub_1xN (and through it bigdivrem_mulsub): z[0..yn] -= x * y. */
static void
fuzz_mulsub(void)
{
    size_t yn = 1 + (size_t)len_upto(MAXN - 1), zn = yn + 1, zo, yo;
    BDIGIT x;
    int c_ret, rs_ret;

    if (fuzz_one_in(8)) {
        zo = place(zn);
        yo = place(yn);  /* any overlap */
    }
    else {
        zo = 0;
        yo = 64;
    }
    switch (fuzz_below(4)) {
      case 0: x = 2; break;  /* what Toom-3 passes */
      default: x = digit(); break;
    }
    setup_arena();
    fuzz_input_num("fn bary_mulsub_1xN, yn", (long)yn);
    fuzz_input_num("x", (long)x);
    fuzz_input_num("zo<<16|yo", (long)((zo << 16) | yo));
    fuzz_input("arena", c_ar, sizeof(c_ar));
    if (fuzz_one_in(2)) mutate_rs(zo, zn); else mutate_rs(yo, yn);

    c_ret = ref_bary_mulsub_1xN(c_ar + zo, zn, x, c_ar + yo, yn);
    rs_ret = rs_rb_core_bignum_bary_mulsub_1xN(rs_ar + zo, zn, x, rs_ar + yo, yn);
    FUZZ_EQ("return", c_ret, rs_ret);
    compare_arena();
}

/* bary_sq_fast: z = x * x. */
static void
fuzz_sq(void)
{
    size_t xn = len_upto(MAXN), zn = 2 * xn + (size_t)fuzz_below(3), zo, xo;

    if (fuzz_one_in(16)) {
        zo = place(zn);
        xo = place(xn);  /* any overlap (no C caller does this) */
    }
    else {
        zo = 0;
        xo = ARENA - MAXN;
    }
    setup_arena();
    fuzz_input_num("fn bary_sq_fast, xn", (long)xn);
    fuzz_input_num("zn", (long)zn);
    fuzz_input_num("zo<<16|xo", (long)((zo << 16) | xo));
    fuzz_input("arena", c_ar, sizeof(c_ar));
    mutate_rs(xo, xn);

    ref_bary_sq_fast(c_ar + zo, zn, c_ar + xo, xn);
    rs_rb_core_bignum_bary_sq_fast(rs_ar + zo, zn, rs_ar + xo, xn);
    compare_arena();
}

static size_t
interesting_size(void)
{
    switch (fuzz_below(5)) {
      case 0: return 0;
      case 1: return SIZE_MAX;
      case 2: return (size_t)fuzz_below(4);
      default: return (size_t)fuzz_next();
    }
}

static void
fuzz_max_size(void)
{
    size_t a = interesting_size(), b = fuzz_one_in(4) ? a : interesting_size();
    fuzz_input_num("fn max_size, a", (long)a);
    fuzz_input_num("b", (long)b);
    FUZZ_EQ("max_size", ref_max_size(a, b),
            rs_rb_core_bignum_max_size((size_t)fuzz_rs_long((long)a), b));
}

/* bigdivrem1: schoolbook division of z (zn digits) by y (yn digits). */
static void
fuzz_divrem1(void)
{
    size_t yn = 1 + (size_t)fuzz_below(16), zn = yn + 1 + (size_t)len_upto(24);
    const size_t zo = 0, yo = 64;
    struct ref_big_div_struct c_bds, rs_bds;
    VALUE stop;
    void *c_ret, *rs_ret;
    size_t i;

    setup_arena();
    /* Divisors close to the dividend's top digits reach the q == BDIGMAX
     * estimate and the "add back" loop more often. */
    if (fuzz_one_in(4)) {
        for (i = 0; i < yn && i + 1 < zn; i++)
            c_ar[zo + zn - 1 - i] = c_ar[yo + yn - 1 - i];
    }
    c_ar[yo + yn - 1] |= 0x80000000U;
    if (c_ar[zo + zn - 1] >= c_ar[yo + yn - 1])
        c_ar[zo + zn - 1] = fuzz_one_in(2) ? c_ar[yo + yn - 1] - 1 : c_ar[zo + zn - 1] % c_ar[yo + yn - 1];
    memcpy(rs_ar, c_ar, sizeof(c_ar));
    switch (fuzz_below(16)) {
      case 0: stop = Qtrue; break;
      case 1: stop = 1; break;
      default: stop = Qfalse; break;
    }
    fuzz_input_num("fn bigdivrem1, yn", (long)yn);
    fuzz_input_num("zn", (long)zn);
    fuzz_input_num("stop", (long)stop);
    fuzz_input("arena", c_ar, sizeof(c_ar));
    /* Mutation mode changes a lower digit only, which keeps the
     * preconditions (the top digits of y and z stay as they are). */
    if (fuzz_one_in(2) || yn == 1) mutate_rs(zo, zn - 1); else mutate_rs(yo, yn - 1);

    c_bds.yn = yn; c_bds.zn = zn; c_bds.yds = c_ar + yo; c_bds.zds = c_ar + zo; c_bds.stop = stop;
    rs_bds.yn = yn; rs_bds.zn = zn; rs_bds.yds = rs_ar + yo; rs_bds.zds = rs_ar + zo; rs_bds.stop = stop;
    c_ret = ref_bigdivrem1(&c_bds);
    rs_ret = rs_rb_core_bignum_bigdivrem1(&rs_bds);
    FUZZ_EQ("return", (uintptr_t)c_ret, (uintptr_t)rs_ret);
    FUZZ_EQ("bds->yn", c_bds.yn, rs_bds.yn);
    FUZZ_EQ("bds->zn", c_bds.zn, rs_bds.zn);
    FUZZ_EQ("bds->yds offset", c_bds.yds - c_ar, rs_bds.yds - rs_ar);
    FUZZ_EQ("bds->zds offset", c_bds.zds - c_ar, rs_bds.zds - rs_ar);
    FUZZ_EQ("bds->stop", c_bds.stop, rs_bds.stop);
    compare_arena();
}

static void
fuzz_one(void)
{
    switch (fuzz_below(8)) {
      case 0: fuzz_add(1); break;
      case 1: fuzz_add(0); break;
      case 2: fuzz_add_one(); break;
      case 3: fuzz_mulsub(); break;
      case 4: fuzz_sq(); break;
      case 5: fuzz_max_size(); break;
      default: fuzz_divrem1(); break;
    }
}
