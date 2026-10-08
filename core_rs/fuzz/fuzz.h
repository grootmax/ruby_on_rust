/*
 * core_rs/fuzz/fuzz.h: differential fuzzing of a core_rs port against the
 * original C.
 *
 * A unit harness (core_rs/fuzz/units/<unit>.c) includes this header after
 * the C reference that core_rs/fuzz/run.rb extracts from the CRuby source.
 * For every ported function `f` it can then call:
 *
 *   ref_f(...)  the original C, extracted verbatim from the .c file;
 *   rs_f(...)   the Rust port, taken from core_rs's staticlib with its
 *               rb_* / ruby_* symbols renamed to rs_*.
 *
 * The harness defines, before including this header,
 *
 *   #define FUZZ_UNIT "name"              the unit name for the report;
 *
 * and after it
 *
 *   static void fuzz_one(void);           one random test case.
 *
 * and optionally `#define FUZZ_HAVE_SETUP` plus `static void fuzz_setup(void)`.
 *
 * In fuzz_one() it draws inputs with fuzz_below() / fuzz_fill(), records
 * them with fuzz_input() (so a mismatch can be reproduced), passes the
 * Rust side its inputs through fuzz_rs_bytes() / fuzz_rs_long(), and
 * compares every observable result with FUZZ_EQ().
 *
 * Normal mode exits 0 only if there were 0 mismatches.  With --mutate the
 * Rust side receives a perturbed input in every case, and the run exits 0
 * only if the harness noticed (mismatches > 0).  The mutation run proves
 * that the comparisons can fail at all.
 *
 * Uses only the C standard library.  No allocation in the hot path.
 */
#ifndef CORE_RS_FUZZ_H
#define CORE_RS_FUZZ_H 1

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---- deterministic PRNG (splitmix64) ---------------------------------- */

static uint64_t fuzz_state;

static uint64_t
fuzz_next(void)
{
    uint64_t z = (fuzz_state += UINT64_C(0x9e3779b97f4a7c15));
    z = (z ^ (z >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94d049bb133111eb);
    return z ^ (z >> 31);
}

/* Uniform in [0, n); n must be > 0. */
static uint64_t
fuzz_below(uint64_t n)
{
    return fuzz_next() % n;
}

/* True with probability 1/n. */
static int
fuzz_one_in(uint64_t n)
{
    return fuzz_below(n) == 0;
}

/*
 * Fill buf with len bytes.  Each byte comes from `alphabet` (if non-NULL
 * and non-empty) with probability 7/8, otherwise it is any byte 0..255.
 * Biasing towards an alphabet of interesting bytes (digits, signs, spaces,
 * the needle's bytes, ...) reaches the deep paths far more often than
 * uniform bytes.
 */
static void
fuzz_fill(unsigned char *buf, size_t len, const char *alphabet)
{
    size_t alen = alphabet ? strlen(alphabet) : 0;
    size_t i;
    for (i = 0; i < len; i++) {
        if (alen && !fuzz_one_in(8))
            buf[i] = (unsigned char)alphabet[fuzz_below(alen)];
        else
            buf[i] = (unsigned char)fuzz_below(256);
    }
}

/* A length in [0, max], biased towards short and boundary lengths. */
static size_t
fuzz_len(size_t max)
{
    switch (fuzz_below(4)) {
      case 0: return (size_t)fuzz_below(max < 8 ? max + 1 : 9);
      case 1: return max;
      default: return (size_t)fuzz_below(max + 1);
    }
}

/* ---- mutation mode ----------------------------------------------------- */

static int fuzz_mutating;
static unsigned char fuzz_rs_buf[2][1 << 16];

/*
 * The bytes the Rust side should see.  Normally `buf` itself.  In
 * mutation mode a copy with one byte changed (slot 0 or 1, so a harness
 * can pass two mutated buffers to one call).
 */
static const unsigned char *
fuzz_rs_bytes_slot(int slot, const unsigned char *buf, size_t len)
{
    if (!fuzz_mutating || len == 0) return buf;
    if (len > sizeof(fuzz_rs_buf[slot])) {
        fprintf(stderr, "fuzz: input of %lu bytes is too long for mutation mode\n", (unsigned long)len);
        exit(2);
    }
    memcpy(fuzz_rs_buf[slot], buf, len);
    fuzz_rs_buf[slot][fuzz_below(len)] ^= (unsigned char)(1 + fuzz_below(255));
    return fuzz_rs_buf[slot];
}
#define fuzz_rs_bytes(buf, len) fuzz_rs_bytes_slot(0, (buf), (len))

/* The integer the Rust side should see: v, or v + 1 in mutation mode. */
static long
fuzz_rs_long(long v)
{
    return fuzz_mutating ? (long)((unsigned long)v + 1) : v;
}

/* ---- recording inputs and comparing results ---------------------------- */

#define FUZZ_MAX_INPUTS 4
static struct { const char *name; const unsigned char *p; size_t len; long num; int is_num; } fuzz_inputs[FUZZ_MAX_INPUTS];
static int fuzz_ninputs;
static uint64_t fuzz_case, fuzz_mismatches, fuzz_case_seed;
static int fuzz_case_failed;

/* Record a byte-string input of the current case for the report. */
static void
fuzz_input(const char *name, const void *p, size_t len)
{
    if (fuzz_ninputs < FUZZ_MAX_INPUTS) {
        fuzz_inputs[fuzz_ninputs].name = name;
        fuzz_inputs[fuzz_ninputs].p = (const unsigned char *)p;
        fuzz_inputs[fuzz_ninputs].len = len;
        fuzz_inputs[fuzz_ninputs].is_num = 0;
        fuzz_ninputs++;
    }
}

/* Record an integer input of the current case for the report. */
static void
fuzz_input_num(const char *name, long num)
{
    if (fuzz_ninputs < FUZZ_MAX_INPUTS) {
        fuzz_inputs[fuzz_ninputs].name = name;
        fuzz_inputs[fuzz_ninputs].num = num;
        fuzz_inputs[fuzz_ninputs].is_num = 1;
        fuzz_ninputs++;
    }
}

static void
fuzz_report(const char *what, unsigned long long c_val, unsigned long long rs_val, const char *file, int line)
{
    int i;
    size_t j;
    if (!fuzz_case_failed) fuzz_mismatches++;
    fuzz_case_failed = 1;
    if (fuzz_mismatches > 10 || fuzz_mutating) return;
    fprintf(stderr, "MISMATCH %s:%d case %llu (case seed 0x%016llx): %s: C=%llu (0x%llx) Rust=%llu (0x%llx)\n",
            file, line, (unsigned long long)fuzz_case, (unsigned long long)fuzz_case_seed,
            what, c_val, c_val, rs_val, rs_val);
    for (i = 0; i < fuzz_ninputs; i++) {
        if (fuzz_inputs[i].is_num) {
            fprintf(stderr, "  %s = %ld\n", fuzz_inputs[i].name, fuzz_inputs[i].num);
            continue;
        }
        fprintf(stderr, "  %s (%lu bytes) =", fuzz_inputs[i].name, (unsigned long)fuzz_inputs[i].len);
        for (j = 0; j < fuzz_inputs[i].len && j < 64; j++)
            fprintf(stderr, " %02x", fuzz_inputs[i].p[j]);
        fprintf(stderr, "%s\n", fuzz_inputs[i].len > 64 ? " ..." : "");
    }
}

/*
 * Compare one observable result of the C reference and the Rust port.
 * Both sides are converted to unsigned long long, so compare pointers as
 * offsets from their buffer (never raw addresses of different buffers).
 */
#define FUZZ_EQ(what, c_val, rs_val) do { \
    unsigned long long fuzz_c_ = (unsigned long long)(c_val); \
    unsigned long long fuzz_r_ = (unsigned long long)(rs_val); \
    if (fuzz_c_ != fuzz_r_) fuzz_report((what), fuzz_c_, fuzz_r_, __FILE__, __LINE__); \
} while (0)

/* ---- driver -------------------------------------------------------------- */

#ifndef FUZZ_UNIT
# error define FUZZ_UNIT before including fuzz.h
#endif
static void fuzz_one(void);
#ifdef FUZZ_HAVE_SETUP
static void fuzz_setup(void);
#endif

int
main(int argc, char **argv)
{
    uint64_t n = 1000000, seed = 1, i;
    int a;

    for (a = 1; a < argc; a++) {
        if (!strcmp(argv[a], "-n") && a + 1 < argc) n = strtoull(argv[++a], NULL, 0);
        else if (!strcmp(argv[a], "-s") && a + 1 < argc) seed = strtoull(argv[++a], NULL, 0);
        else if (!strcmp(argv[a], "--mutate")) fuzz_mutating = 1;
        else if (!strcmp(argv[a], "--case") && a + 1 < argc) {
            /* Replay a single case by its case seed (printed on mismatch). */
            fuzz_state = strtoull(argv[++a], NULL, 0);
            n = 0;
            fuzz_case_seed = fuzz_state;
        }
        else {
            fprintf(stderr, "usage: %s [-n CASES] [-s SEED] [--mutate] [--case CASE_SEED]\n", argv[0]);
            return 2;
        }
    }
#ifdef FUZZ_HAVE_SETUP
    fuzz_setup();
#endif
    if (n == 0) { /* --case */
        fuzz_one();
        printf("fuzz %s: replayed case seed 0x%016llx: %s\n", FUZZ_UNIT,
               (unsigned long long)fuzz_case_seed, fuzz_case_failed ? "MISMATCH" : "ok");
        return fuzz_case_failed ? 1 : 0;
    }
    for (i = 0; i < n; i++) {
        /* Each case starts from its own seed, so it can be replayed alone. */
        fuzz_state = seed * UINT64_C(0x100000001b3) + i;
        fuzz_case_seed = fuzz_state;
        fuzz_case = i;
        fuzz_ninputs = 0;
        fuzz_case_failed = 0;
        fuzz_one();
    }
    printf("fuzz %s: %s cases=%llu mismatches=%llu seed=%llu\n", FUZZ_UNIT,
           fuzz_mutating ? "MUTATION" : "differential",
           (unsigned long long)n, (unsigned long long)fuzz_mismatches, (unsigned long long)seed);
    if (fuzz_mutating) return fuzz_mismatches > 0 ? 0 : 1;
    return fuzz_mismatches == 0 ? 0 : 1;
}

#if defined(__GNUC__) || defined(__clang__)
__attribute__((weak))
#endif
unsigned long long
rb_core_vm_exec_loop(void *ec)
{
    (void)ec;
    return 0;
}

#endif /* CORE_RS_FUZZ_H */
