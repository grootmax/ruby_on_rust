/*
 * Differential fuzz: strftime.c leaf functions
 * (core_rs/src/strftime.rs) against original C.
 *
 * FUZZ-EXTRACT: strftime.c STRFTIME_FLAGS:185-186 min max case_conv strftime_size_limit isleap iso8601wknum weeknumber
 */
#define FUZZ_UNIT "strftime_01"
#include "ruby/ruby.h"
#include <time.h>
#include FUZZ_REF
#include "fuzz.h"

int rs_rb_core_strftime_min(int a, int b);
int rs_rb_core_strftime_max(int a, int b);
char *rs_rb_core_strftime_case_conv(char *s, ptrdiff_t i, int flags);
size_t rs_rb_core_strftime_strftime_size_limit(size_t format_len);
int rs_rb_core_strftime_isleap(long year);
int rs_rb_core_strftime_iso8601wknum(const struct tm *timeptr);
int rs_rb_core_strftime_weeknumber(const struct tm *timeptr, int firstweekday);

#define MAXLEN 64

static void
fuzz_one(void)
{
    // 1. min / max
    int a = (int)fuzz_below(2000) - 1000;
    int b = (int)fuzz_below(2000) - 1000;
    FUZZ_EQ("min", ref_min(a, b), rs_rb_core_strftime_min((int)fuzz_rs_long(a), b));
    FUZZ_EQ("max", ref_max(a, b), rs_rb_core_strftime_max((int)fuzz_rs_long(a), b));

    // 2. isleap
    long yr = (long)fuzz_below(3000);
    FUZZ_EQ("isleap", ref_isleap(yr), rs_rb_core_strftime_isleap(fuzz_rs_long(yr)));

    // 3. strftime_size_limit
    size_t flen = (size_t)fuzz_below(10000);
    FUZZ_EQ("strftime_size_limit", ref_strftime_size_limit(flen), rs_rb_core_strftime_strftime_size_limit((size_t)fuzz_rs_long(flen)));

    // 4. case_conv
    unsigned char buf[MAXLEN];
    size_t len = fuzz_len(MAXLEN - 1) + 1;
    fuzz_fill(buf, len, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ 0123");

    unsigned char c_buf[MAXLEN], rs_buf[MAXLEN];
    memcpy(c_buf, buf, len);
    memcpy(rs_buf, buf, len);

    int flags = (int)fuzz_below(16);
    char *c_res = ref_case_conv((char *)c_buf, (ptrdiff_t)len, flags);
    char *rs_res = rs_rb_core_strftime_case_conv((char *)fuzz_rs_bytes(rs_buf, len), (ptrdiff_t)len, flags);

    FUZZ_EQ("case_conv ptr offset", (long)(c_res - (char *)c_buf), (long)(rs_res - (char *)rs_buf));
    FUZZ_EQ("case_conv bytes", memcmp(c_buf, rs_buf, len), 0);

    // 5. weeknumber & iso8601wknum
    struct tm tm;
    memset(&tm, 0, sizeof(tm));
    tm.tm_year = (int)fuzz_below(200) + 70;
    tm.tm_mon = (int)fuzz_below(12);
    tm.tm_mday = (int)fuzz_below(28) + 1;
    tm.tm_wday = (int)fuzz_below(7);
    tm.tm_yday = (int)fuzz_below(365);

    int fwd = (int)fuzz_below(2);
    FUZZ_EQ("weeknumber", ref_weeknumber(&tm, fwd), rs_rb_core_strftime_weeknumber(&tm, (int)fuzz_rs_long(fwd)));
    FUZZ_EQ("iso8601wknum", ref_iso8601wknum(&tm), rs_rb_core_strftime_iso8601wknum(&tm));
}
