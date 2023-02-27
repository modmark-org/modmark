use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use diffy::create_patch;
use once_cell::sync::Lazy;
use serde_json::Value;

// Unfortunately, if we run multiple tests at the very same time, some of them will sometimes
// fail without reason. This makes sure that only one test is running at a time per package.
// How this is used: When a package with path `p` is attempted at being run, we lock the map,
// lookup `p`, insert default if absent, clone the Arc and then immediately drop the lock on
// the map. The Arc we now got is the mutex associated with the given package. Then, we lock
// the mutex within that arc, and when we have the lock, we know that we are the only thread
// working on that package. Then, we can proceed with doing our actions, and then drop the
// lock after we have done everything.
static LOCK: Lazy<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = Lazy::new(Mutex::default);

fn test_package_input(file: &Path) -> datatest_stable::Result<()> {
    // path is ../packages/package-name/tests/file-name.json
    // to get the path to the package, we pop last 2 components

    // Step 1: Reading input file and preparing paths
    let package_path = file
        .parent()
        .and_then(Path::parent)
        .expect("Popping two components should give package path");

    // We now need to synchronize so that we wait until other threads operating on the same package
    // is done. Since we want multiple tests in parallel but only one per package, we have a map
    // from package path to a mutex, which we need the lock for. The map lock can't be poisoned.
    let mut map = LOCK.lock().unwrap();
    let mutex = map.entry(package_path.to_path_buf()).or_default().clone();
    // Drop the lock on the map now, so we let other threads test if they can proceed
    drop(map);

    // We don't care about poisoned errors here (poisoned errors = previous test failed)
    let lock = {
        match mutex.lock() {
            Ok(lock) => lock,
            Err(poison) => poison.into_inner(),
        }
    };

    let manifest_path = package_path.join("Cargo.toml");
    let input_file = read_to_string(file).expect("Input file should be readable");

    // Step 2: Deserialize json and figure out what we want to transform from, to and what the
    // expected result is
    let input_json: Value = serde_json::from_str(&input_file).expect("Valid JSON example file");
    let from = input_json["name"].as_str().expect("Name for parent/module");
    let to = input_json["__test_transform_to"]
        .as_str()
        .expect("Example has __test_transform_to");
    let expected_result: &Value = &input_json["__test_expected_result"];

    // Step 3: Run the program
    // This runs "cargo run --manifest-path <path-to-pkg-manifest> -- transform <from> <to>"
    // and sets up stdin and stdout to be piped.
    let cmd = Command::new(env!("CARGO"))
        .arg("run")
        .arg(format!(
            "--manifest-path={}",
            manifest_path.to_string_lossy()
        ))
        .arg("--")
        .arg("transform")
        .arg(from)
        .arg(to)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Spawn `cargo run` process");
    // This gets the stdin for the child, and writes the entire input file to it
    let mut stdin = cmd.stdin.as_ref().unwrap();
    write!(stdin, "{}", input_file).expect("Expected stdin to be writable");

    // Step 4: Get the result and compare
    let result = String::from_utf8(cmd.wait_with_output().unwrap().stdout)
        .expect("Expected result to be utf8");
    let json_out: Value = serde_json::from_str(&result).expect("Expected result to be valid json");

    // Make sure to explicitly drop the lock, so that we don't drop it earlier implicitly. It is ok
    // to drop it now since we are done with the package itself.
    drop(lock);

    // Note that we don't want assert_eq! here since we want a custom error message
    if &json_out != expected_result {
        // We have a mismatch and we should fail the test
        // Use diffy to print a nice difference between
        // expected and actual
        let expected = serde_json::to_string_pretty(expected_result).unwrap();
        let actual = serde_json::to_string_pretty(&json_out).unwrap();

        panic!(
            "Failed test,\nEXPECTED\n{}\nGOT\n{}\nDIFF\n{}",
            expected,
            actual,
            create_patch(&expected, &actual)
        );
    }
    Ok(())
}

datatest_stable::harness!(
    test_package_input,
    "../packages",
    r"[^/\\]*[/\\]tests[/\\].*\.json$"
);
