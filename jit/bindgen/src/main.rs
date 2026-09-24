//! Core CRuby binding generation tool for the `jit` core crate.

use std::env;
use std::path::PathBuf;

const SRC_ROOT_ENV: &str = "JIT_SRC_ROOT_PATH";

fn main() {
    let src_root = env::var(SRC_ROOT_ENV)
        .or_else(|_| env::var("SRC_ROOT_PATH"))
        .or_else(|_| env::var("YJIT_SRC_ROOT_PATH"))
        .unwrap_or_else(|_| ".".to_string());
    let src_root = PathBuf::from(src_root);

    assert!(
        src_root.is_dir(),
        "{} must be set to a path to a directory",
        SRC_ROOT_ENV
    );

    env_logger::init();

    let filtered_clang_args = env::args().filter(|arg| arg != "-fvisibility=hidden");

    let bindings = bindgen::builder()
        .clang_args(filtered_clang_args)
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
        .blocklist_type("FILE")
        .blocklist_type("_IO_.*")
        .blocklist_type("VALUE")

        // Core VM / EC types
        .allowlist_type("rb_execution_context_struct")
        .allowlist_type("rb_execution_context_t")
        .allowlist_type("rb_control_frame_struct")
        .allowlist_type("rb_control_frame_t")
        .allowlist_type("rb_iseq_struct")
        .allowlist_type("rb_iseq_t")
        .allowlist_type("rb_callable_method_entry_struct")
        .allowlist_type("rb_callable_method_entry_t")

        .allowlist_type("ruby_vminsn_type")
        .allowlist_type("ruby_special_consts")
        .allowlist_type("RBasic")
        .allowlist_type("RArray")
        .allowlist_type("ruby_value_type")
        .allowlist_type("shape_id_t")
        .allowlist_type("rb_call_data")
        .allowlist_type("rb_callinfo")

        .allowlist_type("ruby_fl_type")
        .allowlist_type("ruby_fl_ushift")
        .allowlist_type("ruby_robject_flags")
        .allowlist_type("ruby_rarray_flags")
        .allowlist_type("ruby_rarray_consts")
        .allowlist_type("ruby_rmodule_flags")
        .allowlist_type("ruby_rstring_flags")
        .allowlist_type("ruby_rstruct_flags")
        .allowlist_type("st_retval")

        .allowlist_function("rb_bug")
        .allowlist_function("rb_utf8_str_new")
        .allowlist_function("rb_funcall")
        .allowlist_function("rb_obj_shape_id")
        .allowlist_function("rb_shape_id_offset")
        .allowlist_function("rb_shape_get_iv_index")
        .allowlist_function("rb_gc_mark")
        .allowlist_function("rb_gc_mark_movable")
        .allowlist_function("rb_gc_location")
        .allowlist_function("rb_gc_writebarrier")

        .allowlist_var("rb_cBasicObject")
        .allowlist_var("rb_cObject")
        .allowlist_var("rb_cModule")
        .allowlist_var("rb_cNilClass")
        .allowlist_var("rb_cTrueClass")
        .allowlist_var("rb_cFalseClass")
        .allowlist_var("rb_cInteger")
        .allowlist_var("rb_cString")
        .allowlist_var("rb_cArray")
        .allowlist_var("rb_cHash")
        .allowlist_var("rb_cClass")

        .allowlist_type("jit_bindgen_constants")

        .prepend_enum_name(false)
        .translate_enum_integer_types(true)

        .generate()
        .expect("Unable to generate bindings");

    let mut bindings_string = Vec::new();
    bindings.write(Box::new(&mut bindings_string)).expect("Couldn't write bindings!");
    let mut bindings_string = String::from_utf8(bindings_string).expect("bindings should be UTF-8");

    bindings_string = bindings_string.replace("extern \"C\" {", "unsafe extern \"C\" {");

    const TYPE_REPLACEMENTS: &[(&str, &str)] = &[
        ("pub type ruby_rstruct_flags = u32;", "pub type ruby_rstruct_flags = usize;"),
    ];
    for (needle, replacement) in TYPE_REPLACEMENTS {
        if bindings_string.contains(needle) {
            bindings_string = bindings_string.replace(needle, replacement);
        }
    }

    let mut out_path: PathBuf = src_root;
    out_path.push("jit");
    out_path.push("src");
    out_path.push("cruby_bindings.inc.rs");

    std::fs::write(out_path, bindings_string).expect("file output failed");
}
