use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Core,
    Yjit,
    Zjit,
    All,
}

impl Target {
    fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "core" | "jit" => Some(Target::Core),
            "yjit" => Some(Target::Yjit),
            "zjit" => Some(Target::Zjit),
            "all" => Some(Target::All),
            _ => None,
        }
    }
}

fn main() {
    env_logger::init();

    let mut target_opt: Option<Target> = None;
    let mut filtered_clang_args = Vec::new();

    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--target" {
            if i + 1 < args.len() {
                if let Some(t) = Target::parse(&args[i + 1]) {
                    target_opt = Some(t);
                    i += 2;
                    continue;
                }
            }
            // Not a bindgen CLI target name; pass through to clang
            filtered_clang_args.push(arg.clone());
        } else if let Some(val) = arg.strip_prefix("--target=") {
            if let Some(t) = Target::parse(val) {
                target_opt = Some(t);
                i += 1;
                continue;
            }
            // Not a bindgen CLI target name; pass through to clang
            filtered_clang_args.push(arg.clone());
        } else if arg != "-fvisibility=hidden" {
            filtered_clang_args.push(arg.clone());
        }
        i += 1;
    }

    if target_opt.is_none() {
        if let Ok(env_target) = env::var("TARGET") {
            target_opt = Target::parse(&env_target);
        }
    }

    // `make TARGET=...` exports `TARGET` to environment.
    // bindgen crate reads `TARGET` env var as the Rust/clang compilation target triple.
    // Remove `TARGET` if it's one of our CLI target names so bindgen doesn't pass `--target core` to libclang.
    if let Ok(env_target) = env::var("TARGET") {
        if Target::parse(&env_target).is_some() {
            unsafe {
                env::remove_var("TARGET");
            }
        }
    }

    let target = target_opt.unwrap_or(Target::All);

    let src_root = if let Ok(val) = env::var("RUST_BINDGEN_SRC_ROOT_PATH") {
        PathBuf::from(val)
    } else if let Ok(val) = env::var("YJIT_SRC_ROOT_PATH") {
        PathBuf::from(val)
    } else if let Ok(val) = env::var("ZJIT_SRC_ROOT_PATH") {
        PathBuf::from(val)
    } else if let Ok(val) = env::var("SRC_ROOT_PATH") {
        PathBuf::from(val)
    } else {
        PathBuf::from(".")
    };

    assert!(
        src_root.is_dir(),
        "src_root must be set to a valid directory path: {:?}",
        src_root
    );

    let targets_to_run = match target {
        Target::All => vec![Target::Core, Target::Yjit, Target::Zjit],
        t => vec![t],
    };

    for t in targets_to_run {
        generate_bindings_for_target(t, &src_root, &filtered_clang_args);
    }
}

fn apply_common_builder_options(
    builder: bindgen::Builder,
    src_root: &Path,
    clang_args: &[String],
) -> bindgen::Builder {
    let builder = builder.rust_target("1.64.0".parse::<bindgen::RustTarget>().unwrap());
    builder
        .clang_args(clang_args)
        .header("encindex.h")
        .header("internal.h")
        .header("internal/object.h")
        .header("internal/re.h")
        .header("include/ruby/ruby.h")
        .header("shape.h")
        .header("vm_core.h")
        .header("vm_callinfo.h")
        .header(src_root.join("jit.c").to_str().unwrap())
        .generate_comments(false)
        .merge_extern_blocks(true)
        .layout_tests(false)
        .blocklist_type("size_t")
        .blocklist_type("fpos_t")
        .allowlist_type("ruby_vminsn_type")
        .allowlist_type("ruby_special_consts")
        .allowlist_function("rb_utf8_str_new")
        .allowlist_function("rb_str_buf_append")
        .allowlist_function("rb_str_dup")
        .allowlist_type("ruby_preserved_encindex")
        .allowlist_function("rb_class2name")
        .allowlist_type("RBasic")
        .allowlist_type("ruby_rstring_flags")
        .allowlist_type("ruby_rstruct_flags")
        .allowlist_function("rb_bug")
        .allowlist_function("rb_obj_shape_id")
        .allowlist_function("rb_shape_id_offset")
        .allowlist_function("rb_shape_get_iv_index")
        .allowlist_function("rb_shape_transition_add_ivar_no_warnings")
        .allowlist_var("SHAPE_ID_NUM_BITS")
        .allowlist_type("shape_id_mask")
        .allowlist_function("rb_funcall")
        .allowlist_function("rb_obj_is_kind_of")
        .allowlist_function("rb_obj_frozen_p")
        .allowlist_type("ruby_encoding_consts")
        .allowlist_function("rb_hash_new")
        .allowlist_function("rb_hash_new_capa")
        .allowlist_function("rb_hash_resurrect")
        .allowlist_function("rb_to_hash_type")
        .allowlist_type("st_retval")
        .allowlist_function("rb_hash_aset")
        .allowlist_function("rb_hash_aref")
        .allowlist_function("rb_hash_bulk_insert")
        .allowlist_function("rb_hash_stlike_lookup")
        .allowlist_function("rb_ary_new_capa")
        .allowlist_function("rb_ary_store")
        .allowlist_function("rb_ary_resurrect")
        .allowlist_function("rb_ary_cat")
        .allowlist_function("rb_ary_clear")
        .allowlist_function("rb_ary_dup")
        .allowlist_function("rb_ary_push")
        .allowlist_function("rb_ary_unshift_m")
        .allowlist_function("rb_ec_ary_new_from_values")
        .allowlist_function("rb_ary_tmp_new_from_values")
        .allowlist_function("rb_class_attached_object")
        .allowlist_function("rb_singleton_class")
        .allowlist_function("rb_class_get_superclass")
        .allowlist_function("rb_gc_mark")
        .allowlist_function("rb_gc_mark_movable")
        .allowlist_function("rb_gc_location")
        .allowlist_function("rb_gc_writebarrier")
        .allowlist_var("rb_cBasicObject")
        .allowlist_var("rb_cModule")
        .allowlist_var("rb_cNilClass")
        .allowlist_var("rb_cTrueClass")
        .allowlist_var("rb_cFalseClass")
        .allowlist_var("rb_cInteger")
        .allowlist_var("rb_cIO")
        .allowlist_var("rb_cSymbol")
        .allowlist_var("rb_cFloat")
        .allowlist_var("rb_cNumeric")
        .allowlist_var("rb_cString")
        .allowlist_var("rb_cThread")
        .allowlist_var("rb_cArray")
        .allowlist_var("rb_cHash")
        .allowlist_var("rb_cClass")
        .allowlist_type("ruby_fl_type")
        .allowlist_type("ruby_fl_ushift")
        .allowlist_type("ruby_robject_flags")
        .allowlist_type("ruby_rarray_flags")
        .allowlist_type("ruby_rarray_consts")
        .allowlist_type("ruby_rmodule_flags")
        .allowlist_var("rb_mKernel")
        .allowlist_type("vm_call_flag_bits")
        .allowlist_type("rb_call_data")
        .blocklist_type("rb_callcache.*")
        .opaque_type("rb_callcache.*")
        .allowlist_type("rb_callinfo")
        .allowlist_var("VM_ENV_DATA_INDEX_ME_CREF")
        .allowlist_var("VM_KW_SPECIFIED_BITS_MAX")
        .allowlist_var("rb_block_param_proxy")
        .allowlist_function("rb_range_new")
        .allowlist_function("rb_intern")
        .allowlist_function("rb_intern2")
        .allowlist_function("rb_id2sym")
        .allowlist_function("rb_sym2id")
        .allowlist_function("rb_str_intern")
        .allowlist_function("rb_fix_aref")
        .allowlist_function("rb_float_plus")
        .allowlist_function("rb_float_minus")
        .allowlist_function("rb_float_mul")
        .allowlist_function("rb_float_div")
        .allowlist_type("ruby_rstring_private_flags")
        .allowlist_function("rb_ec_str_resurrect")
        .allowlist_function("rb_str_concat_literals")
        .allowlist_function("rb_str_byte_substr")
        .allowlist_function("rb_str_substr_two_fixnums")
        .allowlist_function("rb_backref_get")
        .allowlist_function("rb_reg_last_match")
        .allowlist_function("rb_reg_match_pre")
        .allowlist_function("rb_reg_match_post")
        .allowlist_function("rb_reg_match_last")
        .allowlist_function("rb_reg_nth_match")
        .allowlist_function("rb_reg_new_from_values")
        .prepend_enum_name(false)
        .translate_enum_integer_types(true)
        .allowlist_type("ruby_value_type")
        .allowlist_type("ruby_rhash_flags")
        .allowlist_type("rb_method_visibility_t")
        .allowlist_type("rb_method_type_t")
        .allowlist_type("method_optimized_type")
        .allowlist_type("rb_callable_method_entry_t")
        .allowlist_type("rb_callable_method_entry_struct")
        .allowlist_function("rb_method_entry_at")
        .allowlist_type("rb_method_entry_t")
        .blocklist_type("rb_method_cfunc_t")
        .blocklist_type("rb_method_definition_.*")
        .opaque_type("rb_method_definition_.*")
        .allowlist_function("rb_float_new")
        .allowlist_var("rb_mRubyVMFrozenCore")
        .allowlist_var("VM_BLOCK_HANDLER_NONE")
        .allowlist_type("vm_frame_env_flags")
        .allowlist_type("rb_seq_param_keyword_struct")
        .allowlist_type("rb_callinfo_kwarg")
        .allowlist_type("ruby_basic_operators")
        .allowlist_var(".*_REDEFINED_OP_FLAG")
        .allowlist_type("rb_num_t")
        .allowlist_function("rb_callable_method_entry")
        .allowlist_function("rb_callable_method_entry_or_negative")
        .allowlist_function("rb_vm_frame_method_entry")
        .allowlist_type("IVC")
        .allowlist_type("IC")
        .allowlist_type("iseq_inline_constant_cache_entry")
        .blocklist_type("rb_cref_t")
        .opaque_type("rb_cref_t")
        .allowlist_type("iseq_inline_iv_cache_entry")
        .allowlist_type("ICVARC")
        .allowlist_type("iseq_inline_cvar_cache_entry")
        .blocklist_type("rb_execution_context_.*")
        .opaque_type("rb_execution_context_.*")
        .allowlist_function("rb_vm_bh_to_procval")
        .allowlist_function("rb_vm_env_write")
        .allowlist_function("rb_vm_ep_local_ep")
        .allowlist_type("vm_special_object_type")
        .allowlist_var("VM_ENV_DATA_INDEX_SPECVAL")
        .allowlist_var("VM_ENV_DATA_INDEX_FLAGS")
        .allowlist_var("VM_ENV_DATA_SIZE")
        .allowlist_function("rb_iseq_path")
        .allowlist_type("rb_builtin_attr")
        .allowlist_type("ruby_tag_type")
        .allowlist_type("ruby_vm_throw_flags")
        .allowlist_type("vm_check_match_type")
        .allowlist_type("vm_opt_newarray_send_type")
        .allowlist_type("rb_iseq_type")
        .allowlist_function("rb_object_shape_count")
        .allowlist_function("rb_iseq_pc_at_idx")
        .allowlist_function("rb_iseq_opcode_at_pc")
        .allowlist_function("rb_jit_reserve_addr_space")
        .allowlist_function("rb_jit_mark_writable")
        .allowlist_function("rb_jit_mark_executable")
        .allowlist_function("rb_jit_mark_unused")
        .allowlist_function("rb_jit_get_page_size")
        .allowlist_function("rb_jit_iseq_builtin_attrs")
        .allowlist_function("rb_set_cfp_(pc|sp)")
        .allowlist_function("rb_c_method_tracing_currently_enabled")
        .allowlist_function("rb_full_cfunc_return")
        .allowlist_function("rb_assert_(iseq|cme)_handle")
        .allowlist_function("rb_IMEMO_TYPE_P")
        .allowlist_function("rb_iseq_reset_jit_func")
        .allowlist_function("rb_jit_str_simple_append")
        .allowlist_function("rb_RSTRING_PTR")
        .allowlist_function("rb_RSTRING_LEN")
        .allowlist_function("rb_ENCODING_GET")
        .allowlist_function("rb_jit_get_proc_ptr")
        .allowlist_type("rb_block_type")
        .allowlist_function("rb_jit_icache_invalidate")
        .allowlist_function("rb_optimized_call")
        .allowlist_function("rb_jit_str_concat_codepoint")
        .allowlist_function("rb_assert_holding_vm_lock")
        .allowlist_function("rb_jit_shape_complex_p")
        .allowlist_function("rb_jit_multi_ractor_p")
        .allowlist_function("rb_jit_constcache_shareable")
        .allowlist_function("rb_jit_vm_lock_then_barrier")
        .allowlist_function("rb_jit_vm_unlock")
        .allowlist_function("rb_jit_for_each_iseq")
        .allowlist_type("jit_bindgen_constants")
        .allowlist_function("rb_vm_barrier")
        .blocklist_type("FILE")
        .blocklist_type("_IO_.*")
        .allowlist_function("rb_vm_insn_decode")
        .allowlist_function("rb_jit_cont_each_iseq")
        .allowlist_function("rb_vm_insn_addr2opcode")
        .allowlist_function("rb_iseqw_to_iseq")
        .allowlist_function("rb_iseq_label")
        .allowlist_function("rb_iseq_line_no")
        .allowlist_type("defined_type")
        .allowlist_type("rb_builtin_function.*")
        .allowlist_function("rb_gvar_(get|set)")
        .allowlist_function("rb_ensure_iv_list_size")
        .allowlist_function("rb_attr_get")
        .allowlist_function("rb_ivar_defined")
        .allowlist_function("rb_ivar_get")
        .allowlist_function("rb_mod_name")
        .allowlist_function("rb_const_get")
        .allowlist_var("rb_vm_insn_count")
        .allowlist_function("rb_vm_instruction_size")
        .allowlist_function("rb_obj_info_dump")
        .allowlist_function("rb_get_alloc_func")
        .allowlist_function("rb_class_allocate_instance")
        .allowlist_function("rb_obj_equal")
        .allowlist_function("rb_class_new_instance_pass_kw")
        .allowlist_function("rb_obj_alloc")
        .allowlist_function("rb_obj_info")
        .allowlist_function("ruby_xfree")
        .allowlist_function("rb_profile_frames")
        .allowlist_function("rb_insn_name")
        .allowlist_function("rb_insn_len")
        .allowlist_function("rb_yarv_class_of")
        .allowlist_function("rb_get_ec_cfp")
        .allowlist_function("rb_get_cfp_iseq")
        .allowlist_function("rb_get_cfp_pc")
        .allowlist_function("rb_get_cfp_sp")
        .allowlist_function("rb_get_cfp_self")
        .allowlist_function("rb_get_cfp_ep")
        .allowlist_function("rb_get_cme_def_type")
        .allowlist_function("rb_get_cme_def_body_attr_id")
        .allowlist_function("rb_get_symbol_id")
        .allowlist_function("rb_get_cme_def_body_optimized_type")
        .allowlist_function("rb_get_cme_def_body_optimized_index")
        .allowlist_function("rb_get_cme_def_body_cfunc")
        .allowlist_function("rb_get_def_method_serial")
        .allowlist_function("rb_get_def_original_id")
        .allowlist_function("rb_get_mct_argc")
        .allowlist_function("rb_get_mct_func")
        .allowlist_function("rb_get_def_iseq_ptr")
        .allowlist_function("rb_get_def_bmethod_proc")
        .allowlist_function("rb_iseq_encoded_size")
        .allowlist_function("rb_get_iseq_body_local_iseq")
        .allowlist_function("rb_get_iseq_body_parent_iseq")
        .allowlist_function("rb_get_iseq_body_iseq_encoded")
        .allowlist_function("rb_get_iseq_body_stack_max")
        .allowlist_function("rb_get_iseq_body_type")
        .allowlist_function("rb_get_iseq_flags_has_lead")
        .allowlist_function("rb_get_iseq_flags_has_opt")
        .allowlist_function("rb_get_iseq_flags_has_kw")
        .allowlist_function("rb_get_iseq_flags_has_rest")
        .allowlist_function("rb_get_iseq_flags_has_post")
        .allowlist_function("rb_get_iseq_flags_has_kwrest")
        .allowlist_function("rb_get_iseq_flags_anon_kwrest")
        .allowlist_function("rb_get_iseq_flags_has_block")
        .allowlist_function("rb_get_iseq_flags_ambiguous_param0")
        .allowlist_function("rb_get_iseq_flags_accepts_no_kwarg")
        .allowlist_function("rb_get_iseq_flags_ruby2_keywords")
        .allowlist_function("rb_get_iseq_flags_forwardable")
        .allowlist_function("rb_get_iseq_body_local_table_size")
        .allowlist_function("rb_get_iseq_body_param_keyword")
        .allowlist_function("rb_get_iseq_body_param_size")
        .allowlist_function("rb_get_iseq_body_param_lead_num")
        .allowlist_function("rb_get_iseq_body_param_opt_num")
        .allowlist_function("rb_get_iseq_body_param_opt_table")
        .allowlist_function("rb_get_cikw_keyword_len")
        .allowlist_function("rb_get_cikw_keywords_idx")
        .allowlist_function("rb_yarv_str_eql_internal")
        .allowlist_function("rb_str_neq_internal")
        .allowlist_function("rb_yarv_ary_entry_internal")
        .allowlist_function("rb_FL_TEST")
        .allowlist_function("rb_FL_TEST_RAW")
        .allowlist_function("rb_RB_TYPE_P")
        .allowlist_function("rb_BASIC_OP_UNREDEFINED_P")
        .allowlist_function("rb_vm_ci_argc")
        .allowlist_function("rb_vm_ci_mid")
        .allowlist_function("rb_vm_ci_flag")
        .allowlist_function("rb_vm_ci_kwarg")
        .allowlist_function("rb_METHOD_ENTRY_VISI")
        .allowlist_function("rb_RCLASS_ORIGIN")
        .allowlist_function("rb_method_basic_definition_p")
        .allowlist_function("rb_obj_class")
        .allowlist_function("rb_vm_base_ptr")
        .allowlist_function("rb_ec_stack_check")
        .allowlist_function("rb_vm_top_self")
        .blocklist_type("VALUE")
}

fn generate_bindings_for_target(target: Target, src_root: &Path, clang_args: &[String]) {
    let (builder, type_replacements, out_path) = match target {
        Target::Core => {
            let builder = bindgen::builder();
            let builder = apply_common_builder_options(builder, src_root, clang_args);
            let builder = builder
                .allowlist_function("rb_id2name")
                .allowlist_var("rb_cRubyVM")
                .blocklist_type("rb_control_frame_struct")
                .opaque_type("rb_control_frame_struct")
                .allowlist_function("rb_ivar_get_at")
                .allowlist_function("rb_ivar_get_at_no_ractor_check")
                .allowlist_function("rb_iseq_(get|set)_jit_payload")
                .allowlist_function("rb_obj_as_string_result")
                .allowlist_function("rb_get_call_data_ci")
                .allowlist_function("rb_get_cfp_ep_level")
                .allowlist_function("rb_jit_ruby2_keywords_splat_p")
                .allowlist_function("rb_jit_fix_div_fix")
                .allowlist_function("rb_jit_fix_mod_fix")
                .allowlist_function("rb_RSTRUCT_LEN")
                .allowlist_function("rb_RSTRUCT_SET")
                .allowlist_function("rb_jit_array_len")
                .allowlist_function("rb_obj_is_proc")
                .opaque_type("rb_iseq_t")
                .blocklist_type("rb_iseq_t");

            let type_replacements: &[(&str, &str)] = &[
                ("pub type jit_bindgen_constants = u32;", "pub type jit_bindgen_constants = i32;"),
                ("pub type ruby_rstruct_flags = u32;", "pub type ruby_rstruct_flags = usize;"),
            ];
            let out_path = src_root.join("jit").join("src").join("cruby_bindings.inc.rs");
            (builder, type_replacements, out_path)
        }
        Target::Yjit => {
            let builder = bindgen::builder();
            let builder = apply_common_builder_options(builder, src_root, clang_args);
            let builder = builder
                .header(src_root.join("yjit.c").to_str().unwrap())
                .allowlist_function("rb_id2name")
                .allowlist_function("rb_obj_as_string_result")
                .allowlist_function("rb_get_call_data_ci")
                .allowlist_function("rb_get_cfp_ep_level")
                .allowlist_function("rb_yjit_shape_obj_complex_p")
                .allowlist_function("rb_yjit_shape_obj_embedded_p")
                .allowlist_function("rb_yjit_shape_capacity")
                .allowlist_function("rb_yjit_shape_index")
                .allowlist_function("rb_yjit_rb_ary_subseq_length")
                .allowlist_var("rb_cRubyVM")
                .allowlist_function("rb_ivar_get_at")
                .allowlist_function("rb_ivar_get_at_no_ractor_check")
                .allowlist_function("rb_iseq_(get|set)_jit_payload")
                .allowlist_function("rb_yjit_iseq_inspect")
                .allowlist_function("rb_yjit_builtin_function")
                .allowlist_function("rb_yjit_dump_iseq_loc")
                .allowlist_function("rb_yjit_obj_written")
                .allowlist_type("yjit_bindgen_constants")
                .allowlist_function("rb_yjit_exit_locations_dict")
                .allowlist_function("rb_yjit_sendish_sp_pops")
                .allowlist_function("rb_yjit_invokeblock_sp_pops")
                .allowlist_function("rb_yjit_cme_ractor_serial")
                .allowlist_function("rb_yjit_set_exception_return")
                .allowlist_function("rb_yjit_cdhash_all_fixnum_p")
                .allowlist_function("rb_yjit_cdhash_lookup")
                .allowlist_function("rb_yjit_splat_varg_checks")
                .allowlist_function("rb_yjit_splat_varg_cfunc")
                .allowlist_function("rb_jit_ruby2_keywords_splat_p")
                .allowlist_function("rb_jit_fix_div_fix")
                .allowlist_function("rb_jit_fix_mod_fix")
                .allowlist_function("rb_RSTRUCT_LEN")
                .allowlist_function("rb_RSTRUCT_SET")
                .allowlist_function("rb_jit_array_len")
                .allowlist_function("rb_obj_is_proc")
                .blocklist_type("rb_control_frame_struct")
                .opaque_type("rb_control_frame_struct")
                .opaque_type("rb_iseq_t")
                .blocklist_type("rb_iseq_t");

            let type_replacements: &[(&str, &str)] = &[
                ("pub type ruby_rstruct_flags = u32;", "pub type ruby_rstruct_flags = usize;"),
            ];
            let out_path = src_root.join("yjit").join("src").join("cruby_bindings.inc.rs");
            (builder, type_replacements, out_path)
        }
        Target::Zjit => {
            let jit_name = env::var("BINDGEN_JIT_NAME").unwrap_or_else(|_| "zjit".to_string());
            let c_file = format!("{}.c", jit_name);

            let builder = bindgen::builder();
            let builder = apply_common_builder_options(builder, src_root, clang_args);
            let builder = builder
                .header(src_root.join(c_file).to_str().unwrap())
                .header(src_root.join("gc/gc_impl.h").to_str().unwrap())
                .header(src_root.join("gc/default/zjit_fastpath.h").to_str().unwrap())
                .header(src_root.join("gc/mmtk/zjit_fastpath.h").to_str().unwrap())
                .allowlist_type("zjit_jit_frame")
                .allowlist_function("rb_str_getbyte")
                .allowlist_type("ruby_coderange_type")
                .allowlist_type("RArray")
                .allowlist_type("rb_gc_zjit_fastpath_kind")
                .allowlist_type("rb_gc_zjit_fastpath")
                .allowlist_type("rb_gc_zjit_fastpath_data")
                .allowlist_type("rb_gc_zjit_default_new_obj_fastpath")
                .allowlist_type("rb_gc_zjit_mmtk_new_obj_fastpath")
                .allowlist_var("RB_GC_ZJIT_FASTPATH_.*")
                .allowlist_function("rb_raw_obj_info")
                .allowlist_function("ruby_init")
                .allowlist_function("ruby_init_stack")
                .allowlist_function("ruby_options")
                .allowlist_function("ruby_executable_node")
                .allowlist_function("rb_funcallv")
                .allowlist_function("rb_protect")
                .allowlist_function("rb_zjit_iseq_has_profiled_enough")
                .allowlist_function("rb_zjit_iseq_set_jit_entry")
                .allowlist_function("rb_zjit_profile_disable")
                .allowlist_function("rb_zjit_profile_enable")
                .allowlist_function("rb_zjit_insn_to_bare_insn")
                .allowlist_function("rb_zjit_hash_new_size")
                .allowlist_function("rb_zjit_class_allocate_instance_fastpath")
                .allowlist_function("rb_zjit_str_resurrect_fastpath")
                .allowlist_function("rb_zjit_array_dup_can_fastpath")
                .allowlist_function("rb_zjit_array_new_can_fastpath")
                .allowlist_function("rb_zjit_hash_dup_can_fastpath")
                .allowlist_function("rb_zjit_range_new_fastpath")
                .allowlist_function("rb_jit_shape_capacity")
                .allowlist_var("rb_invalid_shape_id")
                .allowlist_type("shape_id_fl_type")
                .allowlist_function("rb_class_inherited_p")
                .allowlist_function("rb_class_real")
                .allowlist_function("rb_hash_stlike_foreach")
                .allowlist_function("rb_hash_new_with_bulk_insert")
                .allowlist_function("rb_ary_concat")
                .allowlist_function("rb_ary_pop")
                .allowlist_function("rb_ary_entry")
                .allowlist_function("rb_define_class")
                .allowlist_function("rb_class_superclass")
                .allowlist_function("rb_gc_disable")
                .allowlist_function("rb_gc_enable")
                .allowlist_function("rb_gc_writebarrier_remember")
                .allowlist_function("rb_gc_register_mark_object")
                .allowlist_function("rb_zjit_new_obj_shape")
                .allowlist_var("rb_cObject")
                .allowlist_var("rb_cRange")
                .allowlist_var("rb_cProc")
                .allowlist_var("rb_cSet")
                .allowlist_var("rb_cRegexp")
                .allowlist_var("rb_cISeq")
                .allowlist_var("rb_cRubyVM")
                .allowlist_function("rb_id2str")
                .allowlist_function("rb_sym2str")
                .allowlist_function("rb_flo_to_i")
                .allowlist_function("rb_any_to_s")
                .allowlist_var("ARG_ENCODING_FIXED")
                .allowlist_var("ARG_ENCODING_NONE")
                .allowlist_var("ONIG_OPTION_IGNORECASE")
                .allowlist_var("ONIG_OPTION_EXTEND")
                .allowlist_var("ONIG_OPTION_MULTILINE")
                .allowlist_function("rb_vm_once_done_value")
                .allowlist_type("rb_control_frame_struct")
                .allowlist_type("rb_event_flag_t")
                .allowlist_function("rb_iseq_(get|set|clear)_jit_payload")
                .allowlist_function("rb_iseq_bare_opcode_at_pc")
                .allowlist_function("rb_zjit_reserve_low_addr_space")
                .allowlist_function("rb_jit_array_len")
                .allowlist_function("rb_jit_ruby2_keywords_splat_p")
                .allowlist_function("rb_jit_fix_div_fix")
                .allowlist_function("rb_zjit_iseq_inspect")
                .allowlist_function("rb_zjit_iseq_insn_set")
                .allowlist_function("rb_zjit_local_id")
                .allowlist_function("rb_id_table_lookup")
                .allowlist_function("rb_zjit_method_tracing_currently_enabled")
                .allowlist_function("rb_zjit_iseq_tracing_currently_enabled")
                .allowlist_function("rb_profile_frame_full_label")
                .allowlist_function("rb_profile_frame_absolute_path")
                .allowlist_function("rb_profile_frame_path")
                .allowlist_function("rb_zjit_print_exception")
                .allowlist_function("rb_zjit_singleton_class_p")
                .allowlist_function("rb_zjit_defined_ivar")
                .allowlist_function("rb_zjit_insn_leaf")
                .allowlist_type("zjit_struct_offsets")
                .allowlist_var("rb_zjit_runtime_offsets")
                .allowlist_var("ZJIT_STACK_MAP_SHIFT")
                .allowlist_var("ZJIT_STACK_MAP_VREG_TAG")
                .allowlist_var("ZJIT_STACK_MAP_SKIP_TAG")
                .allowlist_var("ZJIT_STACK_MAP_BASE_PTR_TAG")
                .allowlist_var("ZJIT_STACK_MAP_BASE_PTR_SIZE_SHIFT")
                .allowlist_var("ZJIT_STACK_MAP_BASE_PTR_INDEX_MASK")
                .allowlist_var("ZJIT_JIT_RETURN_C_FRAME")
                .allowlist_function("rb_iseq_defined_string")
                .allowlist_function("rb_ivar_defined")
                .allowlist_function("rb_ivar_get")
                .allowlist_function("rb_ivar_get_at_no_ractor_check")
                .allowlist_function("rb_ivar_set")
                .allowlist_function("rb_zjit_class_initialized_p")
                .allowlist_function("rb_zjit_can_load_superclass_p")
                .allowlist_function("rb_zjit_class_has_default_allocator")
                .allowlist_function("rb_zjit_class_has_struct_allocator")
                .allowlist_function("rb_zjit_class_get_alloc_func")
                .allowlist_function("rb_zjit_vm_search_method")
                .allowlist_function("rb_zjit_cme_is_cfunc")
                .allowlist_function("rb_get_iseq_body_total_calls")
                .allowlist_function("rb_vm_get_untagged_block_handler")
                .allowlist_function("rb_const_lookup")
                .blocklist_type("ID")
                .blocklist_type("rb_iseq_constant_body")
                .opaque_type("rb_id_table")
                .blocklist_item("rb_thread_struct.*")
                .opaque_type("rb_thread_struct.*")
                .blocklist_item("iseq_inline_storage_entry_.*")
                .opaque_type("iseq_inline_storage_entry")
                .opaque_type("iseq_compile_data");

            let type_replacements: &[(&str, &str)] = &[
                ("pub type jit_bindgen_constants = u32;", "pub type jit_bindgen_constants = i32;"),
                ("pub type ruby_rstruct_flags = u32;", "pub type ruby_rstruct_flags = usize;"),
            ];
            let out_path = src_root.join(jit_name).join("src").join("cruby_bindings.inc.rs");
            (builder, type_replacements, out_path)
        }
        Target::All => unreachable!(),
    };

    let bindings = builder.generate().expect("Unable to generate bindings");

    let mut bindings_string = Vec::new();
    bindings
        .write(Box::new(&mut bindings_string))
        .expect("Couldn't write bindings!");
    let mut bindings_string =
        String::from_utf8(bindings_string).expect("bindings should be UTF-8");

    for (needle, replacement) in type_replacements {
        assert!(
            bindings_string.contains(needle),
            "no line to replace: {needle} for target {target:?}"
        );
        bindings_string = bindings_string.replace(needle, replacement);
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create directory for bindings");
    }

    std::fs::write(&out_path, bindings_string).expect("file output failed");
    println!("Generated bindings for {:?} at {:?}", target, out_path);
}
