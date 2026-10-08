use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let config = cbindgen::Config::from_file(PathBuf::from(&crate_dir).join("cbindgen.toml"))
        .expect("Failed to parse zjit/cbindgen.toml");

    let header_path = PathBuf::from(&crate_dir).join("../zjit.h");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate zjit bindings")
        .write_to_file(&header_path);

    // option_env! automatically registers a rerun-if-env-changed
    if let Some(ruby_build_dir) = option_env!("RUBY_BUILD_DIR") {
        // Link against libminiruby.a
        println!("cargo:rustc-link-search=native={ruby_build_dir}");
        println!("cargo:rustc-link-lib=static:-bundle=miniruby");
        // Re-link when libminiruby.a changes
        println!("cargo:rerun-if-changed={ruby_build_dir}/libminiruby.a");

        // System libraries that libminiruby needs. Has to be
        // ordered after -lminiruby above.
        if let Ok(link_flags) = env::var("RUBY_LD_FLAGS") {
            let mut split_iter = link_flags.split(" ");
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
    }
}
