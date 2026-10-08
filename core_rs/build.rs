fn main() {
    println!("cargo:rustc-cfg=core_rs_flonum");
    println!("cargo:rustc-check-cfg=cfg(core_rs_flonum)");
    println!("cargo:rustc-check-cfg=cfg(core_rs_no_flonum)");
}
