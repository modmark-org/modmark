use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use diffy::create_patch;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
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
        .arg("-q")
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

fn test_package_conventions(file: &Path) -> datatest_stable::Result<()> {
    // path is ../packages/package-name/Cargo.toml
    let package_path = file
        .parent()
        .expect("Popping one component should give package path");

    // We now need to synchronize so that we wait until other threads operating on the same package
    // is done. Since we want multiple tests in parallel but only one per package, we have a map
    // from package path to a mutex, which we need the lock for. The map lock can't be poisoned.
    let mut map = LOCK.lock().unwrap();
    let mutex = map.entry(package_path.to_path_buf()).or_default().clone();
    // Drop the lock on the map now, so we let other threads test if they can proceed
    drop(map);

    let lock = {
        match mutex.lock() {
            Ok(lock) => lock,
            Err(poison) => poison.into_inner(),
        }
    };

    let manifest: Manifest = {
        let cmd = Command::new(env!("CARGO"))
            .arg("run")
            .arg(format!("--manifest-path={}", file.to_string_lossy()))
            .arg("-q")
            .arg("--")
            .arg("manifest")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Spawn `cargo run` process");

        let result = String::from_utf8(cmd.wait_with_output().unwrap().stdout)
            .expect("Expected result to be utf8");
        serde_json::from_str(&result).expect("Not a valid package manifest")
    };

    let cargo_manifest: Value = {
        let cmd = Command::new(env!("CARGO"))
            .arg("read-manifest")
            .arg(format!("--manifest-path={}", file.to_string_lossy()))
            .stdout(Stdio::piped())
            .spawn()
            .expect("Spawn `cargo read-manifest` process");

        let result = String::from_utf8(cmd.wait_with_output().unwrap().stdout)
            .expect("Expected result to be utf8");
        serde_json::from_str(&result).expect("Cargo read-manifest not valid JSON")
    };

    // Make sure to explicitly drop the lock, so that we don't drop it earlier implicitly. It is ok
    // to drop it now since we are done with the package itself.
    // Even though the tests aren't done, we are done with running Cargo and thus don't need
    // exclusive access to the directory anymore.
    drop(lock);

    // First, we want to check the cargo manifest validity
    // Get the last component of the path, which is the folder name
    let package_folder = package_path.components().next_back().unwrap().as_os_str();
    // Get the project name from cargo manifest
    let project_name = cargo_manifest["name"].as_str().unwrap();

    assert_eq!(
        project_name, package_folder,
        "Cargo project name should have the same name as the folder the package is in"
    );

    // Loop though all cargo targets and find all "bin" targets
    let bin_targets: Vec<&serde_json::Map<String, Value>> = cargo_manifest["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|target| {
            let target = target.as_object().unwrap();
            target["kind"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t.as_str().unwrap() == "bin")
                .then_some(target)
        })
        .collect();

    assert_eq!(
        bin_targets.len(),
        1,
        "Cargo project should have exactly one binary target"
    );

    assert_eq!(
        bin_targets[0]["name"].as_str().unwrap(),
        package_folder,
        "Cargo project target should have the same name as the folder the package is in"
    );

    // Now, let's check the package manifest
    // Check that the name is lowercase/digit/_/- and starts lowercase
    let name = manifest.name;

    assert_eq!(
        name.as_str(),
        package_folder,
        "Package should have the same name as the enclosing folder"
    );

    let ok_name = name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');

    assert!(ok_name, "Package name must start with lowercase letter, and must only contain lowercase letters, digits, underscores and dashes");

    // Check that it declares at least one transform
    assert!(
        !manifest.transforms.is_empty(),
        "Package should declare at least one transform"
    );

    // Check all transform details (see check_transform)
    manifest.transforms.iter().for_each(check_transform);

    // See if the package has tests
    let has_tests = package_path
        .join("tests")
        .read_dir()
        .ok()
        .and_then(|mut content| {
            // Note: we can't use .path().ends_with(".json") since that checks one entire component,
            // and not the end of the last component
            content
                .any(|f| f.unwrap().path().to_string_lossy().ends_with(".json"))
                .then_some(())
        })
        .is_some();

    if !has_tests {
        let red = "\x1b[31m";
        let reset = "\x1b[0m";
        eprintln!("{red}WARNING{reset}: Package {project_name} has no tests. Place tests in the /tests folder");
    }

    Ok(())
}

fn check_transform(transform: &Transform) {
    // Check "from"
    assert!(
        transform.from.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.'),
        "Parent and module names should only contain lowercase letters, digits, dashes and dots (transform from {} failed)",
        transform.from
    );

    // Check "to" exists
    assert!(
        !transform.to.is_empty(),
        "Transform should have at least one output format (transform from {} failed)",
        transform.from
    );

    // Check "to" values
    assert!(
        transform
            .to
            .iter()
            .all(|to| to.chars().all(|c| c.is_ascii_lowercase())),
        "Output format should only contain lowercase letters (transform from {} failed)",
        transform.from
    );

    // Check "arguments"
    assert!(
        transform.arguments.iter().all(|arg|
            arg.name.starts_with(|c: char| c.is_ascii_lowercase()) &&
                arg.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        ),
        "Argument names should start with lowercase letter, and must only contain lowercase letters, digits and underscores (transform from {} failed)",
        transform.from
    );

    // Check argument types
    assert!(
        transform.arguments.iter().all(|arg| arg
            .default
            .as_ref()
            .map(|d| arg.r#type.can_be_parsed_from(d))
            .unwrap_or(true)),
        "Argument default values should be of the same type as the specified type"
    )
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct Transform {
    pub from: String,
    pub to: Vec<String>,
    pub description: Option<String>,
    pub arguments: Vec<ArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ArgInfo {
    pub name: String,
    pub default: Option<Value>,
    pub description: String,
    #[serde(default = "default_arg_type")]
    pub r#type: ArgType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ArgType {
    Enum(Vec<String>),
    Primitive(PrimitiveArgType),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimitiveArgType {
    #[serde(alias = "string")]
    String,
    #[serde(alias = "int", alias = "integer", alias = "i64")]
    Integer,
    #[serde(
        rename = "Unsigned integer",
        alias = "uint",
        alias = "unsigned_integer",
        alias = "u64"
    )]
    UnsignedInteger,
    #[serde(alias = "float", alias = "number", alias = "f64")]
    Float,
}

fn default_arg_type() -> ArgType {
    ArgType::Primitive(PrimitiveArgType::String)
}

impl From<PrimitiveArgType> for ArgType {
    fn from(value: PrimitiveArgType) -> Self {
        Self::Primitive(value)
    }
}

impl ArgType {
    pub(crate) fn can_be_parsed_from(&self, value: &Value) -> bool {
        match &self {
            ArgType::Enum(vs) => value
                .as_str()
                .map_or(false, |x| vs.contains(&x.to_string())),
            ArgType::Primitive(t) => t.can_be_parsed_from(value),
        }
    }
}

impl PrimitiveArgType {
    pub(crate) fn can_be_parsed_from(&self, value: &Value) -> bool {
        match self {
            PrimitiveArgType::String => value.is_string(),
            PrimitiveArgType::Integer => value.is_i64(),
            PrimitiveArgType::UnsignedInteger => value.is_u64(),
            PrimitiveArgType::Float => value.is_f64(),
        }
    }
}

datatest_stable::harness!(
    test_package_input,
    "../packages",
    r"[^/\\]*[/\\]tests[/\\].*\.json$",
    test_package_conventions,
    "../packages",
    r"[^/\\]*[/\\]Cargo.toml"
);
