use std::process::Child;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    // build packages if we want to bundle them
    #[cfg(feature = "bundle_std_packages")]
    build_packages()
}

// This build script will build all of the crates found in the top level "packages" directory
// into wasm binary files which allows core to load them from the path
// env!("OUT_DIR")/out/<name_of_module>/wasm32_wasi/release/<name_of_module>.wasm
fn build_packages() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let workspace_path = {
        let mut path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        path.pop();
        path
    };

    let packages_path = workspace_path.join("packages");

    println!("cargo:rerun-if-changed={}", packages_path.to_string_lossy());

    let packages_dir = fs::read_dir(&packages_path).expect("No packages directory found.");

    // Build the wasm file for every crate in the packages directory.
    packages_dir
        .into_iter()
        .map(|f| f.unwrap())
        .filter(|f| f.file_type().unwrap().is_dir())
        .map(|module| {
            build_wasm_package(
                &module.file_name().to_string_lossy(),
                &packages_path,
                &out_path,
            )
        })
        .for_each(|(mut f, name)| {
            let exit = f.wait().expect("failed to launch wasm build");
            if !exit.success() {
                println!("cargo:warning=failed to build package: {name}")
            }
        });
}

fn build_wasm_package(name: &str, packages_path: &Path, output_path: &Path) -> (Child, String) {
    let manifest_path = packages_path.join(name).join("Cargo.toml");
    let output_sub_dir = output_path.join(name);

    let child = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--release")
        .arg(format!(
            "--manifest-path={}",
            manifest_path.to_string_lossy()
        ))
        .arg("--target")
        .arg("wasm32-wasi")
        .arg(format!("--target-dir={}", output_sub_dir.to_string_lossy()))
        .spawn()
        .expect("failed to start wasm build");

    (child, name.to_string())
}
