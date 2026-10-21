/*
 * Differential fuzz: numeric.c math and rounding functions
 * (core_rs/src/numeric.rs) against the original C.
 *
 * FUZZ-EXTRACT: numeric.c round round_half_up round_half_down round_half_even double_div_double rb_dbl_hash int_round_half_even int_round_half_up int_round_half_down float_round_overflow float_round_underflow ruby_float_step_size
 * FUZZ-EXTRACT: hash.c dbl_to_index rb_dbl_long_hash rb_objid_hash:288-322
 */
#define FUZZ_UNIT "numeric_01"
#include "ruby/ruby.h"
#include <float.h>
#include <math.h>
#include "internal.h"
#include "internal/compilers.h"
#include "internal/hash.h"
static long ref_rb_objid_hash(st_index_t index);
#include FUZZ_REF
#include "fuzz.h"

/* Bridge for C hash function extracted from hash.c */
long ref_rb_dbl_long_hash(double d);
long rb_dbl_long_hash(double d) { return ref_rb_dbl_long_hash(d); }

/* Declarations for the Rust ports (renamed with rs_ by run.rb) */
double round(double x);
static inline double rs_round(double x) { return round(x); }
double rs_rb_core_numeric_round_half_up(double x, double s);
double rs_rb_core_numeric_round_half_down(double x, double s);
double rs_rb_core_numeric_round_half_even(double x, double s);
double rs_rb_core_numeric_double_div_double(double x, double y);
VALUE rs_rb_core_numeric_rb_dbl_hash(double d);
SIGNED_VALUE rs_rb_core_numeric_int_round_half_even(SIGNED_VALUE x, SIGNED_VALUE y);
SIGNED_VALUE rs_rb_core_numeric_int_round_half_up(SIGNED_VALUE x, SIGNED_VALUE y);
SIGNED_VALUE rs_rb_core_numeric_int_round_half_down(SIGNED_VALUE x, SIGNED_VALUE y);
int rs_rb_core_numeric_float_round_overflow(int ndigits, int binexp);
int rs_rb_core_numeric_float_round_underflow(int ndigits, int binexp);
double rs_ruby_float_step_size(double beg, double end, double unit, int excl);

#define FUZZ_DBL_EQ(what, c_val, rs_val) do { \
    double c_d_ = (double)(c_val); \
    double rs_d_ = (double)(rs_val); \
    if (isnan(c_d_) && isnan(rs_d_)) { \
        /* both NaN: match */ \
    } else { \
        union { double d; uint64_t u; } uc_, urs_; \
        uc_.d = c_d_; \
        urs_.d = rs_d_; \
        FUZZ_EQ((what), uc_.u, urs_.u); \
    } \
} while (0)

static double
gen_double(void)
{
    switch (fuzz_below(12)) {
      case 0: return 0.0;
      case 1: return -0.0;
      case 2: return 1.0;
      case 3: return -1.0;
      case 4: return 0.5;
      case 5: return -0.5;
      case 6: return 1.5;
      case 7: return 2.5;
      case 8: return INFINITY;
      case 9: return -INFINITY;
      case 10: return NAN;
      default: {
        union { uint64_t u; double d; } conv;
        conv.u = fuzz_next();
        return conv.d;
      }
    }
}

static void
fuzz_one(void)
{
    switch (fuzz_below(12)) {
      case 0: { /* round */
        double x = gen_double();
        double rs_x = fuzz_mutating ? x + 1.0 : x;
        fuzz_input_num("round x", (long)x);
        FUZZ_DBL_EQ("round", ref_round(x), rs_round(rs_x));
        break;
      }
      case 1: { /* round_half_up */
        double x = gen_double();
        double s = gen_double();
        double rs_x = fuzz_mutating ? x + 1.0 : x;
        fuzz_input_num("round_half_up x", (long)x);
        FUZZ_DBL_EQ("round_half_up", ref_round_half_up(x, s), rs_rb_core_numeric_round_half_up(rs_x, s));
        break;
      }
      case 2: { /* round_half_down */
        double x = gen_double();
        double s = gen_double();
        double rs_x = fuzz_mutating ? x + 1.0 : x;
        fuzz_input_num("round_half_down x", (long)x);
        FUZZ_DBL_EQ("round_half_down", ref_round_half_down(x, s), rs_rb_core_numeric_round_half_down(rs_x, s));
        break;
      }
      case 3: { /* round_half_even */
        double x = gen_double();
        double s = gen_double();
        double rs_x = fuzz_mutating ? x + 1.0 : x;
        fuzz_input_num("round_half_even x", (long)x);
        FUZZ_DBL_EQ("round_half_even", ref_round_half_even(x, s), rs_rb_core_numeric_round_half_even(rs_x, s));
        break;
      }
      case 4: { /* double_div_double */
        double x = gen_double();
        double y = gen_double();
        double rs_x = fuzz_mutating ? x + 1.0 : x;
        fuzz_input_num("double_div_double x", (long)x);
        FUZZ_DBL_EQ("double_div_double", ref_double_div_double(x, y), rs_rb_core_numeric_double_div_double(rs_x, y));
        break;
      }
      case 5: { /* rb_dbl_hash */
        double d = gen_double();
        double rs_d = fuzz_mutating ? d + 1.0 : d;
        fuzz_input_num("rb_dbl_hash d", (long)d);
        FUZZ_EQ("rb_dbl_hash", ref_rb_dbl_hash(d), rs_rb_core_numeric_rb_dbl_hash(rs_d));
        break;
      }
      case 6: { /* int_round_half_even */
        SIGNED_VALUE x = (SIGNED_VALUE)fuzz_next();
        SIGNED_VALUE y = (SIGNED_VALUE)fuzz_next();
        if (y == 0) y = 1;
        SIGNED_VALUE rs_x = fuzz_rs_long(x);
        FUZZ_EQ("int_round_half_even", ref_int_round_half_even(x, y), rs_rb_core_numeric_int_round_half_even(rs_x, y));
        break;
      }
      case 7: { /* int_round_half_up */
        SIGNED_VALUE x = (SIGNED_VALUE)fuzz_next();
        SIGNED_VALUE y = (SIGNED_VALUE)fuzz_next();
        if (y == 0) y = 1;
        SIGNED_VALUE rs_x = fuzz_rs_long(x);
        FUZZ_EQ("int_round_half_up", ref_int_round_half_up(x, y), rs_rb_core_numeric_int_round_half_up(rs_x, y));
        break;
      }
      case 8: { /* int_round_half_down */
        SIGNED_VALUE x = (SIGNED_VALUE)fuzz_next();
        SIGNED_VALUE y = (SIGNED_VALUE)fuzz_next();
        if (y == 0) y = 1;
        SIGNED_VALUE rs_x = fuzz_rs_long(x);
        FUZZ_EQ("int_round_half_down", ref_int_round_half_down(x, y), rs_rb_core_numeric_int_round_half_down(rs_x, y));
        break;
      }
      case 9: { /* float_round_overflow */
        int ndigits = (int)fuzz_below(300) - 150;
        int binexp = (int)fuzz_below(2000) - 1000;
        int rs_ndigits = (int)fuzz_rs_long(ndigits);
        FUZZ_EQ("float_round_overflow", ref_float_round_overflow(ndigits, binexp), rs_rb_core_numeric_float_round_overflow(rs_ndigits, binexp));
        break;
      }
      case 10: { /* float_round_underflow */
        int ndigits = (int)fuzz_below(300) - 150;
        int binexp = (int)fuzz_below(2000) - 1000;
        int rs_ndigits = (int)fuzz_rs_long(ndigits);
        FUZZ_EQ("float_round_underflow", ref_float_round_underflow(ndigits, binexp), rs_rb_core_numeric_float_round_underflow(rs_ndigits, binexp));
        break;
      }
      default: { /* ruby_float_step_size */
        double beg = gen_double();
        double end = gen_double();
        double unit = gen_double();
        int excl = (int)fuzz_below(2);
        double rs_beg = fuzz_mutating ? beg + 1.0 : beg;
        FUZZ_DBL_EQ("ruby_float_step_size", ref_ruby_float_step_size(beg, end, unit, excl), rs_ruby_float_step_size(rs_beg, end, unit, excl));
        break;
      }
    }
}
