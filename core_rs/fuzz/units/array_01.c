/*
 * Differential fuzz: array.c rotate_count (port unit array-A-01,
 * core_rs/src/array.rs) against the original C.
 *
 * FUZZ-EXTRACT: array.c rotate_count
 *
 * len is drawn from the precondition len > 0: mostly small (the Array#rotate
 * case), sometimes anywhere up to LONG_MAX.  cnt is drawn over the whole of
 * long, around 0, and around multiples of len (0, +-len, +-2len, ...) and
 * LONG_MIN/LONG_MAX, so both branches, the exact-multiple cases and the
 * extremes of ~cnt are hit.
 */
#define FUZZ_UNIT "array_01"
#include "ruby/ruby.h"
#include <limits.h>
#include FUZZ_REF
#include "fuzz.h"

rb_len_t rs_rb_core_array_rotate_count(rb_len_t cnt, rb_len_t len);

static void
fuzz_one(void)
{
    rb_len_t len, cnt, c_ret, rs_ret;

    switch (fuzz_below(4)) {
      case 0: len = 1; break;
      case 1: len = (rb_len_t)(1 + fuzz_below(64)); break;
      case 2: len = (rb_len_t)(1 + fuzz_below(1 << 20)); break;
      default: len = (rb_len_t)(1 + fuzz_below((uint64_t)LONG_MAX)); break;
    }
    switch (fuzz_below(5)) {
      case 0: cnt = (rb_len_t)fuzz_next(); break;
      case 1: cnt = (rb_len_t)fuzz_below(201) - 100; break;
      case 2: cnt = (rb_len_t)((unsigned long)len * (fuzz_below(7) - 3) + fuzz_below(3) - 1); break;
      case 3: cnt = fuzz_one_in(2) ? LONG_MIN + (rb_len_t)fuzz_below(3) : LONG_MAX - (rb_len_t)fuzz_below(3); break;
      default: cnt = -(rb_len_t)fuzz_below((uint64_t)LONG_MAX); break;
    }
    fuzz_input_num("cnt", cnt);
    fuzz_input_num("len", len);

    c_ret = ref_rotate_count(cnt, len);
    rs_ret = rs_rb_core_array_rotate_count(fuzz_rs_long(cnt), len);
    FUZZ_EQ("ret", c_ret, rs_ret);
}
