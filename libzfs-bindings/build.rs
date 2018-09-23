
extern crate bindgen;
extern crate pkg_config;

use std::env;

fn main() {
    
    let out_file = env::var("OUT_DIR").unwrap() + "/bindings.rs";

    let lib = pkg_config::probe_library("libzfs").unwrap();
    let include_args: Vec<String> = lib.include_paths.iter()
        .map(|p| format!("-I{}", p.to_str().unwrap())).collect();

   let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .constified_enum_module("boolean")
        .constified_enum_module("zfs_type_t")
        .clang_args(include_args)
        .generate()
        .expect("could not generate bindings");

    bindings.write_to_file(out_file).expect("could not write bindings");
}