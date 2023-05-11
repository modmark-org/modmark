// Some functions are used conditionally based on features, so we allow dead code so we don't emit
// build warnings for functions not used
#![allow(dead_code)]

use std::fs::rename;
use std::process::Child;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(all(
    feature = "bundle_std_packages",
    feature = "native",
    feature = "precompile_wasm"
))]
use wasmer_compiler::{ArtifactCreate, Engine, EngineBuilder};
#[cfg(all(
    feature = "bundle_std_packages",
    feature = "native",
    feature = "precompile_wasm"
))]
use wasmer_compiler_cranelift::Cranelift;

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

    // If we want to pre-compile, we make a cranelift engine
    #[cfg(all(
        feature = "bundle_std_packages",
        feature = "native",
        feature = "precompile_wasm"
    ))]
    let engine = EngineBuilder::new(Cranelift::new()).engine();

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
            } else {
                #[cfg(all(
                    feature = "bundle_std_packages",
                    feature = "optimize_bundled_packages"
                ))]
                opt_wasm(&name, &out_path);
                #[cfg(all(
                    feature = "bundle_std_packages",
                    feature = "native",
                    feature = "precompile_wasm"
                ))]
                precompile_wasm(&name, &out_path, &engine);
            }
        });
}

fn opt_wasm(name: &str, out_dir: &Path) {
    let path = out_dir
        .join(name)
        .join("wasm32-wasi")
        .join("release")
        .join(format!("{name}.wasm"));
    let tmp_path = path.with_file_name(format!("{name}-unoptimized.wasm"));

    rename(&path, &tmp_path).expect("Move file");

    // pwd when executing the script is in core. We go to the parent, then search for wasm-opt
    // locally (in modmark root). If not found, we go to /website and search there. If still not
    // found, search globally.
    let local_suffix = PathBuf::from("node_modules/wasm-opt/bin/wasm-opt");
    let current_dir = env::current_dir().unwrap();
    let modmark_location = current_dir.parent().unwrap();
    let modmark_install_location = modmark_location.join(&local_suffix);
    let website_install_location = modmark_location.join("website").join(&local_suffix);
    let which_wasm_opt = which::which("wasm-opt").map_err(|_| ());

    let wasm_opt = fs::metadata(&modmark_install_location)
        .map(|_| &modmark_install_location)
        .or_else(|_| fs::metadata(&website_install_location).map(|_| &website_install_location))
        .map_err(|_| ())
        .or_else(|_| which_wasm_opt.as_ref())
        .expect(&format!(
            "Could not find wasm-opt, searched '{}', '{}' and globally",
            modmark_install_location.display(),
            website_install_location.display()
        ));

    let output = Command::new(&wasm_opt)
        .arg(tmp_path)
        .arg("--output")
        .arg(path)
        .arg("-Oz")
        .output()
        .unwrap();

    if !output.stdout.is_empty() {
        println!(
            "{}",
            String::from_utf8(output.stdout).expect("wasm-opt stdout being UTF-8")
        );
    }

    if !output.stderr.is_empty() {
        let message = String::from_utf8(output.stderr).expect("wasm-opt stderr being UTF-8");
        println!("cargo:warning=wasm-opt on {name:12} stderr: {message}");
    }
}

#[cfg(all(
    feature = "bundle_std_packages",
    feature = "native",
    feature = "precompile_wasm"
))]
/// This function pre-compiles a wasm file for a package with the given name, and makes a new file
/// `{name}-precompiled.wasm` which holds the serialized data for the precompiled package.
fn precompile_wasm(name: &str, output_path: &Path, engine: &Engine) {
    let in_path = output_path
        .join(name)
        .join("wasm32-wasi")
        .join("release")
        .join(format!("{name}.wasm"));

    let out_path = output_path
        .join(name)
        .join("wasm32-wasi")
        .join("release")
        .join(format!("{name}-precompiled.wir"));

    // Much of this code is taken from Module::serialize and by following what it does, function
    // calls etc
    let wasm_source = fs::read(in_path).expect("Read wasm module");
    let artifact = engine
        .compile(wasm_source.as_slice())
        .expect("Compile wasm module");
    let compiled = artifact.serialize().expect("Serialize wasm module");
    fs::write(out_path, compiled.as_slice()).expect("Write wasm module");
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
