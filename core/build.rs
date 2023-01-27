use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

// This build script will build all of the crates found in the top level "modules" directory
// into wasm binary files which allows core to load them from the path
// env!("OUT_DIR")/out/<name_of_module>/wasm32_wasi/release/<name_of_module>.wasm
fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let workspace_path = {
        let mut path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        path.pop();
        path
    };

    let modules_path = workspace_path.join("modules");

    println!("cargo:rerun-if-changed={}", modules_path.to_string_lossy());

    let modules_dir = fs::read_dir(&modules_path).expect("No modules directory found.");

    // Build the wasm file for every crate in the modules directory.
    for module in modules_dir {
        // Ensure that it is a directory.
        let module = module.unwrap();
        if !module.file_type().unwrap().is_dir() {
            continue;
        }

        build_wasm_module(
            &module.file_name().to_string_lossy(),
            &modules_path,
            &out_path,
        );
    }
}

fn build_wasm_module(name: &str, modules_path: &Path, output_path: &Path) {
    let manifest_path = modules_path.join(name).join("Cargo.toml");
    let output_sub_dir = output_path.join(name);

    Command::new(env!("CARGO"))
        .arg("build")
        .arg("--release")
        .arg(format!(
            "--manifest-path={}",
            manifest_path.to_string_lossy()
        ))
        .arg("--target")
        .arg("wasm32-wasi")
        .arg(format!("--target-dir={}", output_sub_dir.to_string_lossy()))
        .output()
        .expect("failed to start wasm build");
}
