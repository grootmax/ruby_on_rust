/*
 * Differential fuzz: util.c number scanning and word splitting
 * (core_rs/src/util.rs) against the original C.
 *
 * FUZZ-EXTRACT: util.c ruby_scan_oct ruby_scan_hex ruby_scan_digits ruby_strtoul ruby_each_words
 */
#define FUZZ_UNIT "util_scan"
#include "ruby/ruby.h"
#include "ruby/util.h"
#include <errno.h>
#include <limits.h>
#include FUZZ_REF
#include "fuzz.h"

unsigned long rs_ruby_scan_digits(const char *str, ssize_t len, int base, size_t *retlen, int *overflow);
unsigned long rs_ruby_scan_oct(const char *str, size_t len, size_t *retlen);
unsigned long rs_ruby_scan_hex(const char *str, size_t len, size_t *retlen);
unsigned long rs_ruby_strtoul(const char *str, char **endptr, int base);
void rs_ruby_each_words(const char *str, void (*func)(const char *, int, void *), void *arg);

/* Digits of every base, signs, spaces, a radix prefix, and ',' for each_words. */
static const char alphabet[] = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFXZ +-\t\n\v\f\r,x0";

#define MAXLEN 80

/* ruby_each_words callback: log (offset, len) pairs. */
struct words { const char *base; int n; long log[2 * MAXLEN]; };

static void
collect(const char *word, int len, void *arg)
{
    struct words *w = arg;
    if (w->n < MAXLEN) {
        w->log[2 * w->n] = (long)(word - w->base);
        w->log[2 * w->n + 1] = len;
    }
    w->n++;
}

static void
fuzz_one(void)
{
    unsigned char buf[MAXLEN + 1];
    size_t len = fuzz_len(MAXLEN);
    const char *c_in, *rs_in;
    int i;

    fuzz_fill(buf, len, alphabet);
    buf[len] = '\0';
    /* Long digit runs reach the overflow paths. */
    if (fuzz_one_in(4)) {
        size_t k;
        if (fuzz_one_in(4) && len >= 20) {
            memcpy(buf, "18446744073709551616", 20);
        } else {
            char fill = "0123456789abcdefzZ"[fuzz_below(18)];
            for (k = 0; k < len; k++) buf[k] = fuzz_one_in(2) ? fill : "0123456789abcdefzZ"[fuzz_below(18)];
        }
    }
    fuzz_input("str", buf, len);
    c_in = (const char *)buf;
    rs_in = (const char *)fuzz_rs_bytes(buf, len + 1);
    if (fuzz_mutating) ((char *)rs_in)[len] = '\0';

    switch (fuzz_below(5)) {
      case 0: { /* ruby_scan_digits: base 2..36, len -1 (to NUL) or 0..len */
        int base = 2 + (int)fuzz_below(35);
        ssize_t n = fuzz_one_in(4) ? -1 : (ssize_t)fuzz_below(len + 1);
        size_t c_ret = 0xdead, rs_ret = 0xbeef;
        int c_ov = 7, rs_ov = 9;
        unsigned long c_v, rs_v;
        fuzz_input_num("base", base);
        fuzz_input_num("len", (long)n);
        c_v = ref_ruby_scan_digits(c_in, n, base, &c_ret, &c_ov);
        rs_v = rs_ruby_scan_digits(rs_in, n, base, &rs_ret, &rs_ov);
        FUZZ_EQ("scan_digits value", c_v, rs_v);
        FUZZ_EQ("scan_digits retlen", c_ret, rs_ret);
        FUZZ_EQ("scan_digits overflow", c_ov, rs_ov);
        break;
      }
      case 1: case 2: { /* ruby_scan_oct / ruby_scan_hex */
        int hex = fuzz_one_in(2);
        size_t n = (size_t)fuzz_below(len + 1);
        size_t c_ret = 0xdead, rs_ret = 0xbeef;
        unsigned long c_v, rs_v;
        fuzz_input_num(hex ? "scan_hex len" : "scan_oct len", (long)n);
        c_v = hex ? ref_ruby_scan_hex(c_in, n, &c_ret) : ref_ruby_scan_oct(c_in, n, &c_ret);
        rs_v = hex ? rs_ruby_scan_hex(rs_in, n, &rs_ret) : rs_ruby_scan_oct(rs_in, n, &rs_ret);
        FUZZ_EQ("scan_oct/hex value", c_v, rs_v);
        FUZZ_EQ("scan_oct/hex retlen", c_ret, rs_ret);
        break;
      }
      case 3: { /* ruby_strtoul: any base -2..40, endptr, errno */
        static const int bases[] = { -1, 0, 0, 0, 1, 2, 8, 10, 16, 36, 37 };
        int base = fuzz_one_in(10) ? -1 : (fuzz_one_in(10) ? 1 : (fuzz_one_in(10) ? 37 : (fuzz_one_in(2) ? bases[fuzz_below(sizeof(bases) / sizeof(bases[0]))] : (int)fuzz_below(43) - 2)));
        char *c_end = NULL, *rs_end = NULL;
        int c_errno, rs_errno, use_end = !fuzz_one_in(8);
        unsigned long c_v, rs_v;
        fuzz_input_num("strtoul base", base);
        errno = 12345;
        c_v = ref_ruby_strtoul(c_in, use_end ? &c_end : NULL, base);
        c_errno = errno;
        errno = 12345;
        rs_v = rs_ruby_strtoul(rs_in, use_end ? &rs_end : NULL, base);
        rs_errno = errno;
        FUZZ_EQ("strtoul value", c_v, rs_v);
        FUZZ_EQ("strtoul errno", c_errno, rs_errno);
        FUZZ_EQ("strtoul endptr offset",
                c_end ? (long)(c_end - c_in) : -1L, rs_end ? (long)(rs_end - rs_in) : -1L);
        break;
      }
      default: { /* ruby_each_words */
        static struct words cw, rw;
        if (fuzz_one_in(16)) {
            ref_ruby_each_words(NULL, collect, &cw);
            rs_ruby_each_words(NULL, collect, &rw);
            FUZZ_EQ("each_words NULL count", cw.n, rw.n);
            break;
        }
        cw.base = c_in; cw.n = 0;
        rw.base = rs_in; rw.n = 0;
        ref_ruby_each_words(c_in, collect, &cw);
        rs_ruby_each_words(rs_in, collect, &rw);
        FUZZ_EQ("each_words count", cw.n, rw.n);
        for (i = 0; i < cw.n && i < rw.n && i < MAXLEN; i++) {
            FUZZ_EQ("each_words offset", cw.log[2 * i], rw.log[2 * i]);
            FUZZ_EQ("each_words len", cw.log[2 * i + 1], rw.log[2 * i + 1]);
        }
        break;
      }
    }
}
