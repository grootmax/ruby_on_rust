// This build script is only used for `make zjit-test` / `cargo test` for building
// the test binary; ruby builds don't use this.
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=RUBY_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=RUBY_LD_FLAGS");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = Path::new(&manifest_dir);

    // Determine build directory containing libminiruby.a
    let explicit_build_dir = env::var("RUBY_BUILD_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| option_env!("RUBY_BUILD_DIR").map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty());

    let (build_dir, archive_found) = if let Some(dir_str) = explicit_build_dir {
        let path = PathBuf::from(&dir_str);
        let exists = path.join("libminiruby.a").exists();
        (path, exists)
    } else {
        // Fallback discovery: search relative build directories (.. or ../..)
        let candidate_1 = manifest_path.join("..");
        let candidate_2 = manifest_path.join("../..");

        if candidate_1.join("libminiruby.a").exists() {
            (candidate_1, true)
        } else if candidate_2.join("libminiruby.a").exists() {
            (candidate_2, true)
        } else {
            // Default fallback to parent directory if not found anywhere
            (candidate_1, false)
        }
    };

    if !archive_found {
        println!(
            "cargo:warning=libminiruby.a not found in build directory '{}'. Please build miniruby first (e.g. run 'make miniruby' or 'make').",
            build_dir.display()
        );
    }

    // Link against libminiruby.a
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static:-bundle=miniruby");

    let archive_path = build_dir.join("libminiruby.a");
    println!("cargo:rerun-if-changed={}", archive_path.display());

    // System libraries that libminiruby needs. Has to be ordered after -lminiruby above.
    if let Some(link_flags) = env::var("RUBY_LD_FLAGS").ok().filter(|s| !s.trim().is_empty()) {
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
    } else {
        // Default system libraries fallback
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=crypt");
        println!("cargo:rustc-link-lib=gmp");
        println!("cargo:rustc-link-lib=rt");
    }
}

