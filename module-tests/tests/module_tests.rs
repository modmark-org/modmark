use std::path::Path;

fn test_module_input(file: &Path) -> datatest_stable::Result<()> {
    // path is module-name/tests/file-name.json

    Ok(())
}

datatest_stable::harness!(
    test_module_input,
    "../modules",
    r"[^/\\]*[/\\]tests[/\\].*\.json$"
);
