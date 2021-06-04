use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // \drg\sdk_gen
    let mut workspace_path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());

    // \drg\
    workspace_path.pop();

    // \drg\sdk
    workspace_path.push("sdk");

    let mut out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    out.push("sdk_path");

    fs::write(out, workspace_path.to_str().unwrap()).unwrap();
}
