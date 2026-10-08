/*
 * Not compiled: core_rs/core_rs.mk runs this file through the C preprocessor
 * with the same flags as the C sources ($(XCFLAGS) $(CPPFLAGS)), and turns
 * the result into rustc --cfg options.  So core_rs/src/ffi/value.rs uses the
 * configuration the C build really has, including overrides passed with
 * cppflags (e.g. cppflags=-DUSE_FLONUM=0, which CI builds).
 */
#include "ruby/internal/special_consts.h"

core_rs_cfg_use_flonum USE_FLONUM
