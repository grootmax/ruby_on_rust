/*
 * Differential fuzz: re.c rb_memsearch and rb_memcicmp
 * (core_rs/src/re.rs) against the original C.
 *
 * FUZZ-EXTRACT: re.c casetable rb_memcicmp rb_memsearch_ss rb_memsearch_qs rb_memsearch_qs_utf8_hash rb_memsearch_qs_utf8 rb_memsearch_with_char_size rb_memsearch_wchar rb_memsearch_qchar rb_memsearch
 *
 * rb_memsearch_ss has two #if variants in re.c; the extractor takes the
 * first one, the memmem() variant, which is the one Linux compiles.
 */
#define FUZZ_UNIT "re_memsearch"
#define FUZZ_HAVE_SETUP 1
#include "ruby/ruby.h"
#include "ruby/encoding.h"
#include "internal.h"
#include <string.h>
#if !defined(HAVE_MEMMEM) || defined(__APPLE__)
# error "this harness extracts the memmem() variant of rb_memsearch_ss"
#endif
#include FUZZ_REF
#include "fuzz.h"

long rs_rb_memsearch(const void *x0, long m, const void *y0, long n, rb_encoding *enc);
int rs_rb_memcicmp(const void *x, const void *y, long len);

#define MAXLEN 96
/* Slack after each buffer: the C Quick Search reads one byte past the last
 * window and the UTF-8 hash up to three past the needle.  Zero-filled, as
 * the Rust port treats bytes outside the strings as 0. */
#define SLACK 8

static rb_encoding *encs[6];
static OnigEncodingType fake_min2, fake_min4, fake_min3;

static void
fuzz_setup(void)
{
    RUBY_INIT_STACK;
    ruby_init();
    encs[0] = rb_utf8_encoding();
    encs[1] = rb_ascii8bit_encoding();
    encs[2] = rb_usascii_encoding();
    if (!encs[0] || !encs[1] || !encs[2] || encs[0]->min_enc_len != 1) {
        fprintf(stderr, "fuzz re_memsearch: encodings not initialised\n");
        exit(2);
    }
    /* rb_memsearch reads only min_enc_len and compares the pointer with
     * rb_utf8_encoding(); fakes cover the 2- and 4-byte paths and the
     * fall-through for any other minimum length. */
    fake_min2 = *rb_ascii8bit_encoding(); fake_min2.min_enc_len = 2; encs[3] = &fake_min2;
    fake_min4 = *rb_ascii8bit_encoding(); fake_min4.min_enc_len = 4; encs[4] = &fake_min4;
    fake_min3 = *rb_ascii8bit_encoding(); fake_min3.min_enc_len = 3; encs[5] = &fake_min3;
}

/* UTF-8-heavy alphabet: ASCII, lead bytes of every length, continuations. */
static const char alphabet[] = "abcAB\xc3\xa9\xe3\x81\x82\xf0\x9f\x98\x80\x80\xbf\xf5\xff";

static void
fuzz_one(void)
{
    static unsigned char x[MAXLEN + SLACK], y[MAXLEN + SLACK];
    size_t n = fuzz_len(MAXLEN), m;
    const unsigned char *rx, *ry;

    memset(x, 0, sizeof(x));
    memset(y, 0, sizeof(y));
    fuzz_fill(y, n, fuzz_one_in(3) ? "ab" : alphabet);
    /* Needle: often a real substring of the haystack, sometimes random. */
    m = fuzz_one_in(6) ? n + fuzz_below(3) : fuzz_len(n);
    if (m > MAXLEN) m = MAXLEN;
    if (m <= n && !fuzz_one_in(3)) {
        size_t at = (size_t)fuzz_below(n - m + 1);
        memcpy(x, y + at, m);
        if (m && fuzz_one_in(4)) x[fuzz_below(m)] ^= 1; /* near miss */
    }
    else {
        fuzz_fill(x, m, alphabet);
    }
    fuzz_input("needle", x, m);
    fuzz_input("haystack", y, n);

    if (fuzz_one_in(4)) {
        /* rb_memcicmp over min(m, n) bytes, with ASCII case flips. */
        long len = (long)(m < n ? m : n);
        size_t k;
        for (k = 0; k < (size_t)len; k++)
            if (fuzz_one_in(3)) x[k] = y[k] ^ (((y[k] | 0x20) >= 'a' && (y[k] | 0x20) <= 'z') ? 0x20 : 0);
        rx = fuzz_rs_bytes_slot(0, x, (size_t)len);
        FUZZ_EQ("memcicmp", ref_rb_memcicmp(x, y, len), rs_rb_memcicmp(rx, y, len));
        return;
    }
    {
        rb_encoding *enc = encs[fuzz_below(6)];
        long c_r, rs_r;
        rx = m ? fuzz_rs_bytes_slot(0, x, m) : x;
        ry = y;
        if (fuzz_mutating && rx != x) {
            /* Mutated copies need the same zero slack. */
            static unsigned char mx[MAXLEN + SLACK];
            memset(mx, 0, sizeof(mx));
            memcpy(mx, rx, m);
            rx = mx;
        }
        fuzz_input_num("enc min_enc_len (0=UTF-8)", enc == encs[0] ? 0 : enc->min_enc_len);
        c_r = ref_rb_memsearch(x, (long)m, y, (long)n, enc);
        rs_r = rs_rb_memsearch(rx, (long)m, ry, (long)n, enc);
        FUZZ_EQ("memsearch", c_r, rs_r);
    }
}
