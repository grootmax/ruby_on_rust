# Make recipes that deal with the rust code of YJIT, ZJIT, and core Rust.
#
# $(gnumake_recursive) adds the '+' prefix to pass down GNU make's
# jobserver resources to cargo/rustc as rust-lang.org recommends.
# Without it, certain make version trigger a warning. It does not
# add the prefix when `make --dry-run` so dry runs are indeed dry.

RUST_TARGET_FLAG = $(if $(RUST_TARGET),--target=$(RUST_TARGET),)

ifneq ($(JIT_CARGO_SUPPORT),no)

# Show Cargo progress when doing `make V=1`
CARGO_VERBOSE_0 = -q
CARGO_VERBOSE_1 =
CARGO_VERBOSE = $(CARGO_VERBOSE_$(V))

RUST_LIB_TOUCH = touch $@

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

else ifneq ($(USE_RUST_PORTS),0) # Direct rustc build for core Rust

CORE_RS_RLIB = $(TOP_BUILD_DIR)/target/release/libcore_rs.rlib

$(CORE_RS_RLIB): $(top_srcdir)/core_rs/src/lib.rs target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=core_rs \
	    --edition=2024 \
	    --crate-type=rlib \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/core_rs/src/lib.rs'

ifneq ($(strip $(RLIB_DIR)),) # combo build of YJIT + ZJIT
JIT_RLIB = $(TOP_BUILD_DIR)/$(RLIB_DIR)/libjit.rlib
$(YJIT_RLIB): $(JIT_RLIB)
$(ZJIT_RLIB): $(JIT_RLIB)
$(JIT_RLIB): $(top_srcdir)/jit/src/lib.rs target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=jit \
	    --edition=2024 \
	    $(JIT_RUST_FLAGS) \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/jit/src/lib.rs'

$(RUST_LIB): $(srcdir)/ruby.rs $(CORE_RS_RLIB) $(JIT_RLIB) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2024 \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '-L$(@D)' \
	    --extern=core_rs=$(CORE_RS_RLIB) \
	    --extern=yjit \
	    --extern=zjit \
	    --crate-type=staticlib \
	    --cfg 'feature="core_rs"' \
	    --cfg 'feature="yjit"' \
	    --cfg 'feature="zjit"' \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/ruby.rs'

else # single core_rs, or core_rs + YJIT release

ifneq ($(YJIT_SUPPORT),no)
YJIT_RLIB = $(TOP_BUILD_DIR)/target/release/libyjit.rlib
$(YJIT_RLIB): $(top_srcdir)/yjit/src/lib.rs target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --crate-name=yjit \
	    --edition=2021 \
	    --crate-type=rlib \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/yjit/src/lib.rs'

$(RUST_LIB): $(srcdir)/ruby.rs $(CORE_RS_RLIB) $(YJIT_RLIB) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2024 \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '-L$(@D)' \
	    --extern=core_rs=$(CORE_RS_RLIB) \
	    --extern=yjit=$(YJIT_RLIB) \
	    --crate-type=staticlib \
	    --cfg 'feature="core_rs"' \
	    --cfg 'feature="yjit"' \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/ruby.rs'
else
$(RUST_LIB): $(srcdir)/ruby.rs $(CORE_RS_RLIB) target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2024 \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '-L$(@D)' \
	    --extern=core_rs=$(CORE_RS_RLIB) \
	    --crate-type=staticlib \
	    --cfg 'feature="core_rs"' \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/ruby.rs'
endif # ifneq ($(YJIT_SUPPORT),no)

endif # ifneq ($(strip $(RLIB_DIR)),)

else ifneq ($(strip $(RLIB_DIR)),) # combo build without core_rs

$(RUST_LIB): $(srcdir)/ruby.rs target/.rustc-version
	$(ECHO) 'building $(@F)'
	$(Q)$(MAKEDIRS) $(@D)
	$(gnumake_recursive)$(Q) $(RUSTC) --edition=2024 \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '-L$(@D)' \
	    --extern=yjit \
	    --extern=zjit \
	    --crate-type=staticlib \
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
	    $(JIT_RUST_FLAGS) \
	    $(RUSTC_FLAGS) \
	    $(RUST_TARGET_FLAG) \
	    '--out-dir=$(@D)' \
	    '$(top_srcdir)/jit/src/lib.rs'

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

# For Darwin only
ifneq ($(findstring darwin,$(target_os)),)
$(RUST_LIB_SYMBOLS): $(RUST_LIB)
	$(Q) $(tooldir)/darwin-ar $(NM) --defined-only --extern-only $(RUST_LIB) | \
	sed -n -e 's/.* //' -e '/^$(SYMBOL_PREFIX)rb_/p' \
	-e '/^$(SYMBOL_PREFIX)rust_eh_personality/p' \
	> $@

$(RUST_LIBOBJ): $(RUST_LIB_SYMBOLS)
endif
endif # ifneq ($(RUST_LIB),)

rustc-version-check: target/.rustc-version

target/.rustc-version: PHONY
	$(eval prev_version := $(if $(wildcard $@),$(shell cat $@)))
	$(eval curr_version := $(shell $(RUSTC) -V))
	$(eval clean := $(filter-out $(prev_version),$(curr_version)))
	$(if $(clean),$(ECHO) "Cleaning $(@D) for $(curr_version)")
	$(if $(clean),$(Q)$(RMALL) $(@D))
	$(if $(clean),$(Q)$(MAKEDIRS) $(@D))
	$(if $(clean),$(Q)echo "$(curr_version)" > $@)
