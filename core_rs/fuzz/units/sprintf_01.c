/*
 * Differential fuzz: sprintf.c leaf functions
 * (core_rs/src/sprintf.rs) against original C.
 *
 * FUZZ-EXTRACT: sprintf.c sign_bits fmt_setup ruby_ultoa
 */
#define FUZZ_UNIT "sprintf_01"
#include "ruby/ruby.h"
#include FUZZ_REF
#include "fuzz.h"

char rs_rb_core_sprintf_sign_bits(int base, const char *p);
char *rs_rb_core_sprintf_fmt_setup(char *buf, size_t size, int c, int flags, int width, int prec);
char *rs_rb_core_sprintf_ruby_ultoa(unsigned long val, char *endp, int base, int flags);

#define MAXLEN 64

static void
fuzz_one(void)
{
    // 1. sign_bits
    int base = (int)fuzz_below(20);
    char p_str[2] = { fuzz_one_in(2) ? 'X' : 'x', '\0' };
    char c_sb = ref_sign_bits(base, p_str);
    char rs_sb = rs_rb_core_sprintf_sign_bits(base, p_str);
    FUZZ_EQ("sign_bits", c_sb, rs_sb);

    // 2. ruby_ultoa
    unsigned long val = (unsigned long)fuzz_below(100000);
    int ult_base = fuzz_one_in(3) ? 10 : (fuzz_one_in(2) ? 16 : 8);
    int flags = (int)fuzz_below(128);

    char c_buf[MAXLEN], rs_buf[MAXLEN];
    char *c_end = c_buf + MAXLEN;
    char *rs_end = rs_buf + MAXLEN;

    char *c_res = ref_ruby_ultoa(val, c_end, ult_base, flags);
    char *rs_res = rs_rb_core_sprintf_ruby_ultoa(val, rs_end, ult_base, flags);

    long c_len = (long)(c_end - c_res);
    long rs_len = (long)(rs_end - rs_res);
    FUZZ_EQ("ruby_ultoa len", c_len, rs_len);
    if (c_len == rs_len && c_len > 0) {
        FUZZ_EQ("ruby_ultoa bytes", memcmp(c_res, rs_res, c_len), 0);
    }

    // 3. fmt_setup
    int spec = 'd';
    int width = (int)fuzz_below(50);
    int prec = (int)fuzz_below(50);
    char c_fbuf[MAXLEN], rs_buf2[MAXLEN];

    char *c_fptr = ref_fmt_setup(c_fbuf, MAXLEN, spec, flags, width, prec);
    char *rs_fptr = rs_rb_core_sprintf_fmt_setup(rs_buf2, MAXLEN, spec, flags, width, prec);

    long c_flen = (long)(c_fbuf + MAXLEN - c_fptr);
    long rs_flen = (long)(rs_buf2 + MAXLEN - rs_fptr);
    FUZZ_EQ("fmt_setup len", c_flen, rs_flen);
    if (c_flen == rs_flen && c_flen > 0) {
        FUZZ_EQ("fmt_setup bytes", memcmp(c_fptr, rs_fptr, c_flen), 0);
    }
}
