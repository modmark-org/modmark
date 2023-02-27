use std::fs::read_to_string;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use diffy::create_patch;

fn test_module_input(file: &Path) -> datatest_stable::Result<()> {
    // path is ../modules/module-name/tests/file-name.json
    // to get the path to the module, we pop last 2 components
    let module_path = file.parent().and_then(Path::parent).unwrap();
    let manifest_path = module_path.join("Cargo.toml");
    let input_file = read_to_string(file).unwrap();

    let input_json: Value = serde_json::from_str(&input_file).expect("Valid JSON example file");
    let from = input_json["name"].as_str().expect("Name for parent/module");
    let to = input_json["__test_transform_to"]
        .as_str()
        .expect("Example has __test_transform_to");
    let expected_result: &Value = &input_json["__test_expected_result"];

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
    let mut child_in = cmd.stdin.as_ref().unwrap();
    write!(child_in, "{}", input_file).unwrap();

    let result = String::from_utf8(cmd.wait_with_output().unwrap().stdout).unwrap();
    let json_out: Value = serde_json::from_str(&result).unwrap();

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
    test_module_input,
    "../modules",
    r"[^/\\]*[/\\]tests[/\\].*\.json$"
);
