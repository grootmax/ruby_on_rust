# Make rules for core_rs, the Ruby on Rust core crate.
#
# Included only when configure enabled the Rust ports (USE_RUST_PORTS=1):
# from defs/gmake.mk under GNU make, and appended to the Makefile by
# configure otherwise.  Stick to POSIX make features.
#
# core_rs is built with rustc alone (no cargo, no network) as a no_std
# staticlib, then partially linked into a single object that joins
# COMMONOBJS.  Only `rb_*` and `ruby_*` symbols stay global.

# Keep in sync with core_rs/src/.
CORE_RS_SRCS = $(srcdir)/core_rs/src/lib.rs \
	$(srcdir)/core_rs/src/util.rs \
	$(empty)

$(CORE_RS_LIB): $(CORE_RS_SRCS)
	$(ECHO) 'building core_rs (Rust ports)'
	$(Q) $(MAKEDIRS) $(TOP_BUILD_DIR)/target/core_rs
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=core_rs --crate-type=staticlib --edition=2024 \
	    $(CORE_RS_RUSTC_FLAGS) \
	    -o $(CORE_RS_LIB) \
	    $(srcdir)/core_rs/src/lib.rs

$(CORE_RS_OBJ): $(CORE_RS_LIB)
	$(ECHO) 'partial linking core_rs into $@'
	$(Q) $(CORE_RS_PARTIAL_LINK)

core-rs: $(CORE_RS_OBJ)

# Unit tests for the pure parts of core_rs (needs only rustc; std is used
# by the test harness, the library itself stays no_std).
core-rs-test:
	$(Q) $(MAKEDIRS) $(TOP_BUILD_DIR)/target/core_rs
	$(Q) $(RUSTC) --crate-name=core_rs --edition=2024 --test \
	    -o $(TOP_BUILD_DIR)/target/core_rs/core_rs-test \
	    $(srcdir)/core_rs/src/lib.rs
	$(Q) $(TOP_BUILD_DIR)/target/core_rs/core_rs-test
