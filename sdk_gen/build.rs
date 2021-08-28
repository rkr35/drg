use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let sdk_path = get_sdk_path().expect("failed to get sdk path");
    // We need something that implements AsRef<[u8]> for `fs::write()`. Make that "something" a &str.
    let sdk_path = sdk_path.to_str().expect("failed to convert sdk_path to UTF-8");

    // The SDK generator will read a file at this path to learn about where to output the generated SDK. 
    let mut file_to_place_sdk_path_in = PathBuf::from(env::var_os("OUT_DIR").expect("failed to get output directory"));
    file_to_place_sdk_path_in.push("sdk_path");

    fs::write(file_to_place_sdk_path_in, sdk_path).expect("failed to write sdk_path to output directory");
}

fn get_sdk_path() -> Option<PathBuf> {
    // drg/sdk_gen
    let mut workspace_path = get_workspace_path()?;

    // drg/
    workspace_path.pop();

    // drg/sdk
    workspace_path.push("sdk");

    Some(workspace_path)
}

fn get_workspace_path() -> Option<PathBuf> {
    Some(PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?))
}