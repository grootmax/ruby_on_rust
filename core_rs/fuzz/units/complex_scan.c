/*
 * Differential fuzz: complex.c number-fragment scanner (port unit
 * complex-A-01, core_rs/src/complex.rs) against the original C.
 *
 * FUZZ-EXTRACT: complex.c issign read_sign isdecimal read_digits islettere read_num read_den read_rat_nos read_rat isimagunit skip_ws
 *
 * Exercised through the entry points the remaining C calls: read_sign,
 * read_rat_nos, read_rat, skip_ws, issign, isdecimal, isimagunit.  The
 * private helpers (read_digits, read_num, read_den, islettere) are reached
 * through them.
 */
#define FUZZ_UNIT "complex_scan"
#define FUZZ_HAVE_SETUP 1
#include "ruby/ruby.h"
#include <ctype.h>
#include <locale.h>
#include <string.h>
#include FUZZ_REF
#include "fuzz.h"

int rs_rb_core_complex_issign(int c);
int rs_rb_core_complex_isdecimal(int c);
int rs_rb_core_complex_isimagunit(int c);
int rs_rb_core_complex_read_sign(const char **s, char **b);
int rs_rb_core_complex_read_rat_nos(const char **s, int strict, char **b);
int rs_rb_core_complex_read_rat(const char **s, int strict, char **b);
void rs_rb_core_complex_skip_ws(const char **s);

static void
fuzz_setup(void)
{
    /* Ruby runs with LC_CTYPE from the environment; so does the harness. */
    setlocale(LC_CTYPE, "");
}

#define MAXLEN 48

/* Characters of complex literals, underscores in every position, spaces. */
static const char alphabet[] = "0123456789__..eE+-/iIjJ@ \t\n\v\f\r1";

static void
fuzz_one(void)
{
    unsigned char in[MAXLEN + 1];
    /* The C callers size the output as the whole input plus NUL; give both
     * sides that plus generous slack, filled with the same sentinel, and
     * compare all of it. */
    char c_out[2 * MAXLEN + 8], rs_out[2 * MAXLEN + 8];
    size_t len = fuzz_len(MAXLEN);
    const char *rs_in, *c_s, *rs_s;
    char *c_b, *rs_b;
    int strict = (int)fuzz_below(2), c_r = 0, rs_r = 0;
    size_t k;
    uint64_t which = fuzz_below(8);

    fuzz_fill(in, len, alphabet);
    in[len] = '\0';
    fuzz_input("str", in, len);
    fuzz_input_num("strict", strict);
    rs_in = (const char *)fuzz_rs_bytes(in, len);
    if (fuzz_mutating && len) {
        /* fuzz_rs_bytes() copies only len bytes; terminate the copy. */
        static char tmp[MAXLEN + 1];
        memcpy(tmp, rs_in, len);
        tmp[len] = '\0';
        rs_in = tmp;
    }

    memset(c_out, 0x5a, sizeof(c_out));
    memset(rs_out, 0x5a, sizeof(rs_out));
    c_s = (const char *)in;
    rs_s = rs_in;
    c_b = c_out;
    rs_b = rs_out;

    switch (which) {
      case 0:
        fuzz_input_num("fn read_sign", 0);
        c_r = ref_read_sign(&c_s, &c_b);
        rs_r = rs_rb_core_complex_read_sign(&rs_s, &rs_b);
        break;
      case 1: case 2:
        fuzz_input_num("fn read_rat_nos", 0);
        c_r = ref_read_rat_nos(&c_s, strict, &c_b);
        rs_r = rs_rb_core_complex_read_rat_nos(&rs_s, strict, &rs_b);
        break;
      case 3: case 4:
        fuzz_input_num("fn read_rat", 0);
        c_r = ref_read_rat(&c_s, strict, &c_b);
        rs_r = rs_rb_core_complex_read_rat(&rs_s, strict, &rs_b);
        break;
      case 5:
        fuzz_input_num("fn skip_ws", 0);
        ref_skip_ws(&c_s);
        rs_rb_core_complex_skip_ws(&rs_s);
        break;
      default: {
        /* The predicates take an int: every char value, plus EOF-ish and
         * out-of-range values. */
        int c = (int)fuzz_below(600) - 300;
        int rc = (int)fuzz_rs_long(c);
        fuzz_input_num("c", c);
        FUZZ_EQ("issign", ref_issign(c), rs_rb_core_complex_issign(rc));
        FUZZ_EQ("isdecimal", ref_isdecimal(c), rs_rb_core_complex_isdecimal(rc));
        FUZZ_EQ("isimagunit", ref_isimagunit(c), rs_rb_core_complex_isimagunit(rc));
        return;
      }
    }
    FUZZ_EQ("return", c_r, rs_r);
    FUZZ_EQ("*s offset", (long)(c_s - (const char *)in), (long)(rs_s - rs_in));
    FUZZ_EQ("*b offset", (long)(c_b - c_out), (long)(rs_b - rs_out));
    for (k = 0; k < sizeof(c_out); k++)
        FUZZ_EQ("output byte", (unsigned char)c_out[k], (unsigned char)rs_out[k]);
}
