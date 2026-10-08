/*
 * Differential fuzz: pack.c leaf functions
 * (core_rs/src/pack.rs) against original C.
 *
 * FUZZ-EXTRACT: pack.c is_bigendian skip_to_eol pack_alignof hex2num
 */
#define FUZZ_UNIT "pack_01"
#include "ruby/ruby.h"
#include FUZZ_REF
#include "fuzz.h"

int rs_rb_core_pack_is_bigendian(void);
char *rs_rb_core_pack_skip_to_eol(const char *p, const char *pend);
int rs_rb_core_pack_pack_alignof(char type, int natint);
int rs_rb_core_pack_hex2num(char c);

#define MAXLEN 80

static void
fuzz_one(void)
{
    unsigned char buf[MAXLEN + 1];
    size_t len = fuzz_len(MAXLEN);
    const char *c_in, *rs_in;

    fuzz_fill(buf, len, "0123456789abcdefABCDEF \n\txyz");
    buf[len] = '\0';

    c_in = (const char *)buf;
    rs_in = (const char *)fuzz_rs_bytes(buf, len + 1);

    // 1. hex2num
    char ch = (char)buf[fuzz_below(len + 1)];
    int c_h = ref_hex2num(ch);
    int rs_h = rs_rb_core_pack_hex2num(ch);
    FUZZ_EQ("hex2num", c_h, rs_h);

    // 2. pack_alignof
    char type = (char)buf[fuzz_below(len + 1)];
    int natint = (int)fuzz_below(2);
    int c_al = ref_pack_alignof(type, natint);
    int rs_al = rs_rb_core_pack_pack_alignof(type, natint);
    FUZZ_EQ("pack_alignof", c_al, rs_al);

    // 3. is_bigendian
    int c_be = ref_is_bigendian();
    int rs_be = rs_rb_core_pack_is_bigendian();
    FUZZ_EQ("is_bigendian", c_be, rs_be);

    // 4. skip_to_eol
    const char *c_pend = c_in + len;
    const char *rs_pend = rs_in + len;
    char *c_eol = ref_skip_to_eol(c_in, c_pend);
    char *rs_eol = rs_rb_core_pack_skip_to_eol(rs_in, rs_pend);
    FUZZ_EQ("skip_to_eol offset", (long)(c_eol - c_in), (long)(rs_eol - rs_in));
}
