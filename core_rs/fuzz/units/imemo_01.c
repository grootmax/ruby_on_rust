/*
 * Differential fuzz: imemo.c rb_imemo_name (port unit imemo-A-01,
 * core_rs/src/imemo.rs) against the original C.
 *
 * FUZZ-EXTRACT: imemo.c rb_imemo_name
 *
 * Every enum imemo_type value is drawn.  Values outside the enum end the
 * process through rb_bug() on both sides, so they are covered by the Rust
 * unit tests instead (they return None from the safe core).  The mutated run
 * shifts the type by one modulo 16, so it stays inside the enum.  The two
 * names are compared byte by byte, terminating NUL included.
 */
#define FUZZ_UNIT "imemo_01"
#include "ruby/ruby.h"
#include "internal.h"
#include "internal/imemo.h"
#include <string.h>
#include FUZZ_REF
#include "fuzz.h"

const char *rs_rb_imemo_name(enum imemo_type type);

#define IMEMO_TYPES 16

static void
fuzz_one(void)
{
    enum imemo_type type = (enum imemo_type)fuzz_below(IMEMO_TYPES);
    const char *c_name, *rs_name;
    size_t c_len, rs_len, i;

    fuzz_input_num("type", (long)type);
    c_name = ref_rb_imemo_name(type);
    rs_name = rs_rb_imemo_name((enum imemo_type)((unsigned long)fuzz_rs_long((long)type) % IMEMO_TYPES));
    c_len = strlen(c_name);
    rs_len = strlen(rs_name);
    FUZZ_EQ("len", c_len, rs_len);
    for (i = 0; i <= c_len && i <= rs_len; i++) {
        FUZZ_EQ("byte", (unsigned char)c_name[i], (unsigned char)rs_name[i]);
    }
}
