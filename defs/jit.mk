# Make recipes that deal with the rust code of YJIT and ZJIT.
#
# $(gnumake_recursive) adds the '+' prefix to pass down GNU make's
# jobserver resources to cargo/rustc as rust-lang.org recommends.
# Without it, certain make version trigger a warning. It does not
# add the prefix when `make --dry-run` so dry runs are indeed dry.

ifneq ($(JIT_CARGO_SUPPORT),no)

# Show Cargo progress when doing `make V=1`
CARGO_VERBOSE_0 = -q
CARGO_VERBOSE_1 =
CARGO_VERBOSE = $(CARGO_VERBOSE_$(V))

# Because of Cargo cache, if the actual binary is not changed from the
# previous build, the mtime is preserved as the cached file.
# This means the target is not updated actually, and it will need to
# rebuild at the next build.
RUST_LIB_TOUCH = touch $@

# NOTE: MACOSX_DEPLOYMENT_TARGET to match `rustc --print deployment-target` to avoid the warning below.
#    ld: warning: object file (target/debug/libjit.a(<libcapstone object>)) was built for
#    newer macOS version (15.2) than being linked (15.0)
# This limits us to an older set of macOS API in the rust code, but we don't use any.
$(RUST_LIB): $(srcdir)/ruby.rs target/.rustc-version
	$(Q)if [ '$(ZJIT_SUPPORT)' != no -a '$(YJIT_SUPPORT)' != no ]; then \
	    echo 'building YJIT and ZJIT ($(JIT_CARGO_SUPPORT:yes=release) mode)'; \
	elif [ '$(ZJIT_SUPPORT)' != no ]; then \
	    echo 'building ZJIT ($(JIT_CARGO_SUPPORT) mode)'; \
	elif [ '$(YJIT_SUPPORT)' != no ]; then \
	    echo 'building YJIT ($(JIT_CARGO_SUPPORT) mode)'; \
	fi
	$(gnumake_recursive)$(Q)CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    CARGO_TERM_PROGRESS_WHEN='never' \
	    MACOSX_DEPLOYMENT_TARGET=11.0 \
	    $(CARGO) $(CARGO_VERBOSE) build --manifest-path '$(top_srcdir)/Cargo.toml' $(CARGO_BUILD_ARGS)
	$(RUST_LIB_TOUCH)
else ifneq ($(RUST_LIB),)

ifneq ($(strip $(RUST_TARGET)),)
RUST_TARGET_FLAG = --target=$(RUST_TARGET)
endif

CORE_RS_SRC_FILES = $(wildcard \
	$(top_srcdir)/core_rs/Cargo.* \
	$(top_srcdir)/core_rs/src/*.rs \
	$(top_srcdir)/core_rs/src/*/*.rs \
	)

CORE_RS_RLIB = $(TOP_BUILD_DIR)/target/release/libcore_rs.rlib

$(CORE_RS_RLIB): $(CORE_RS_SRC_FILES) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=core_rs \
	    --edition=2024 \
	    --crate-type=rlib \
	    $(RUST_TARGET_FLAG) \
	    $(RUSTC_FLAGS) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/core_rs/src/lib.rs'

ifeq ($(USE_RUST_PORTS),1)
CORE_RS_DEP = $(CORE_RS_RLIB)
CORE_RS_EXTERN = --extern=core_rs
CORE_RS_CFG = --cfg 'feature="core_rs"'
else
CORE_RS_DEP =
CORE_RS_EXTERN =
CORE_RS_CFG =
endif

ifneq ($(strip $(RLIB_DIR)),) # combo build

$(RUST_LIB): $(srcdir)/ruby.rs $(CORE_RS_DEP) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2024 \
	    $(RUST_TARGET_FLAG) \
	    $(RUSTC_FLAGS) \
	    '-L$(@D)' \
	    $(CORE_RS_EXTERN) \
	    --extern=yjit \
	    --extern=zjit \
	    --crate-type=staticlib \
	    $(CORE_RS_CFG) \
	    --cfg 'feature="yjit"' \
	    --cfg 'feature="zjit"' \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/ruby.rs'

# Absolute path to avoid VPATH ambiguity
JIT_RLIB = $(TOP_BUILD_DIR)/$(RLIB_DIR)/libjit.rlib
$(YJIT_RLIB): $(JIT_RLIB)
$(ZJIT_RLIB): $(JIT_RLIB)
$(JIT_RLIB): $(top_srcdir)/jit/src/lib.rs target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=jit \
	    --edition=2024 \
	    $(RUST_TARGET_FLAG) \
	    $(JIT_RUST_FLAGS) \
	    $(RUSTC_FLAGS) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/jit/src/lib.rs'
else ifeq ($(YJIT_SUPPORT),yes) # YJIT direct rustc build
YJIT_SRC_FILES = $(wildcard \
	$(top_srcdir)/yjit/Cargo.* \
	$(top_srcdir)/yjit/src/*.rs \
	$(top_srcdir)/yjit/src/*/*.rs \
	)
YJIT_RLIB = $(TOP_BUILD_DIR)/target/release/libyjit.rlib
$(YJIT_RLIB): $(YJIT_SRC_FILES) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=yjit \
	    --edition=2021 \
	    $(RUST_TARGET_FLAG) \
	    $(RUSTC_FLAGS) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/yjit/src/lib.rs'

$(RUST_LIB): $(srcdir)/ruby.rs $(CORE_RS_DEP) $(YJIT_RLIB) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2021 \
	    $(RUST_TARGET_FLAG) \
	    $(RUSTC_FLAGS) \
	    '-L$(@D)' \
	    $(CORE_RS_EXTERN) \
	    --extern=yjit \
	    --crate-type=staticlib \
	    $(CORE_RS_CFG) \
	    --cfg 'feature="yjit"' \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/ruby.rs'
else # standard direct rustc build
$(RUST_LIB): $(srcdir)/ruby.rs $(CORE_RS_DEP) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2024 \
	    $(RUST_TARGET_FLAG) \
	    $(RUSTC_FLAGS) \
	    '-L$(@D)' \
	    $(CORE_RS_EXTERN) \
	    --crate-type=staticlib \
	    $(CORE_RS_CFG) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/ruby.rs'
endif # ifneq ($(strip $(RLIB_DIR)),)
endif # ifneq ($(JIT_CARGO_SUPPORT),no)

ifneq ($(RUST_LIB),)
RUST_LIB_SYMBOLS = $(RUST_LIB:.a=).symbols
$(RUST_LIBOBJ): $(RUST_LIB)
	$(ECHO) 'partial linking $(RUST_LIB) into $@'
ifneq ($(findstring darwin,$(target_os)),)
	$(Q) $(CC) -nodefaultlibs -r -o $@ -exported_symbols_list $(RUST_LIB_SYMBOLS) $(RUST_LIB)
else
	$(Q) $(LD) -r -o $@ --whole-archive $(RUST_LIB)
	-$(Q) $(OBJCOPY) --wildcard --keep-global-symbol='$(SYMBOL_PREFIX)rb_*' $(@)
endif

rust-libobj: $(RUST_LIBOBJ)
rust-lib: $(RUST_LIB)

rustc-version-check: target/.rustc-version

target/.rustc-version: PHONY
	$(eval prev_version := $(if $(wildcard $@),$(shell cat $@)))
	$(eval curr_version := $(shell $(RUSTC) -V))
	$(eval clean := $(filter-out $(prev_version),$(curr_version)))
	$(if $(clean),$(ECHO) "Cleaning $(@D) for $(curr_version)")
	$(if $(clean),$(Q)$(RMALL) $(@D))
	$(if $(clean),$(Q)$(MAKEDIRS) $(@D))
	$(if $(clean),$(Q)echo "$(curr_version)" > $@)

# For Darwin only: a list of symbols that we want the glommed Rust static lib to export.
# Unfortunately, using wildcard like '_rb_*' with -exported-symbol does not work, at least
# not on version 820.1. Assume llvm-nm, so XCode 8.0 (from 2016) or newer.
#
# The -exported_symbols_list pulls out the right archive members. Symbols not listed
# in the list are made private extern, which are in turn made local as we're using `ld -r`.
# Note, section about -keep_private_externs in ld's man page hints at this behavior on which
# we rely.
ifneq ($(findstring darwin,$(target_os)),)
$(RUST_LIB_SYMBOLS): $(RUST_LIB)
	$(Q) $(tooldir)/darwin-ar $(NM) --defined-only --extern-only $(RUST_LIB) | \
	sed -n -e 's/.* //' -e '/^$(SYMBOL_PREFIX)rb_/p' \
	-e '/^$(SYMBOL_PREFIX)rust_eh_personality/p' \
	> $@

$(RUST_LIBOBJ): $(RUST_LIB_SYMBOLS)
endif
endif
