/*
 * test/ruby_on_rust/test_value_helpers_parity.c
 * Verifies bit-for-bit parity between CRuby header macros and Rust inline helpers.
 */

#include "ruby/ruby.h"
#include <stdio.h>
#include <assert.h>

int main(void) {
    // Special Constants Parity
    assert(Qfalse == 0x00);
#if USE_FLONUM
    assert(Qnil == 0x04);
    assert(Qtrue == 0x14);
    assert(Qundef == 0x24);
    assert(RUBY_IMMEDIATE_MASK == 0x07);
    assert(RUBY_FIXNUM_FLAG == 0x01);
    assert(RUBY_FLONUM_MASK == 0x03);
    assert(RUBY_FLONUM_FLAG == 0x02);
    assert(RUBY_SYMBOL_FLAG == 0x0c);
#else
    assert(Qnil == 0x02);
    assert(Qtrue == 0x06);
    assert(Qundef == 0x0a);
    assert(RUBY_IMMEDIATE_MASK == 0x03);
    assert(RUBY_FIXNUM_FLAG == 0x01);
    assert(RUBY_FLONUM_MASK == 0x00);
    assert(RUBY_FLONUM_FLAG == 0x02);
    assert(RUBY_SYMBOL_FLAG == 0x0e);
#endif

    // RTEST / NIL_P / FIXNUM_P Macro Parity
    assert(!RB_TEST(Qfalse));
    assert(!RB_TEST(Qnil));
    assert(RB_TEST(Qtrue));
    assert(RB_NIL_P(Qnil));
    assert(!RB_NIL_P(Qfalse));
    assert(RB_UNDEF_P(Qundef));
    assert(!RB_UNDEF_P(Qnil));

    // Fixnum conversion Parity
    assert(INT2FIX(0) == 1);
    assert(INT2FIX(1) == 3);
    assert(INT2FIX(-1) == (VALUE)-1);
    assert(FIX2LONG(INT2FIX(42)) == 42);
    assert(FIX2LONG(INT2FIX(-42)) == -42);

    // Flag Bitmask Parity
    assert(FL_USHIFT == 12);
    assert(FL_FREEZE == (1 << 11));
    assert(FL_SHAREABLE == (1 << 8));
    assert(RSTRING_NOEMBED == (1 << 13));
    assert(RARRAY_EMBED_FLAG == (1 << 13));
    assert(RARRAY_EMBED_LEN_SHIFT == 15);

    printf("CRuby C macro bitmask parity checks passed!\n");
    return 0;
}
