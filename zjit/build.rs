// This build script is used for `cargo test` and `make zjit-test` for building
// the test binary; ruby builds don't use this.
fn main() {
    use std::env;
    use std::path::Path;

    println!("cargo:rerun-if-env-changed=RUBY_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=RUBY_LD_FLAGS");

    let ruby_build_dir = env::var("RUBY_BUILD_DIR")
        .ok()
        .or_else(|| option_env!("RUBY_BUILD_DIR").map(String::from))
        .unwrap_or_else(|| {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            let workspace_root = Path::new(&manifest_dir).parent().unwrap();
            workspace_root.to_str().unwrap().to_string()
        });

    let libminiruby_path = Path::new(&ruby_build_dir).join("libminiruby.a");
    if !libminiruby_path.exists() {
        eprintln!(
            "error: libminiruby.a not found at {}. Please build miniruby first.",
            libminiruby_path.display()
        );
        panic!(
            "libminiruby.a not found at {}. Please build miniruby first.",
            libminiruby_path.display()
        );
    }

    // Link against libminiruby.a
    println!("cargo:rustc-link-search=native={ruby_build_dir}");
    println!("cargo:rustc-link-lib=static:-bundle=miniruby");
    // Re-link when libminiruby.a changes
    println!("cargo:rerun-if-changed={ruby_build_dir}/libminiruby.a");

    // System libraries that libminiruby needs. Has to be
    // ordered after -lminiruby above.
    let link_flags = env::var("RUBY_LD_FLAGS")
        .ok()
        .or_else(|| option_env!("RUBY_LD_FLAGS").map(String::from))
        .unwrap_or_else(|| "-lpthread -ldl -lm -lz -lgmp -lcrypt -lrt".to_string());

    let mut split_iter = link_flags.split_whitespace();
    while let Some(token) = split_iter.next() {
        if token == "-framework" {
            if let Some(framework) = split_iter.next() {
                println!("cargo:rustc-link-lib=framework={framework}");
            }
        } else if let Some(lib_name) = token.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib_name}");
        }
    }
}
