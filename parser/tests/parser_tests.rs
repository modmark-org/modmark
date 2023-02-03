use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;

use std::path::{PathBuf};

use json::{object, JsonValue};

use parser::{parse, Element};

#[test]
fn test_a() {
    let filename = format!(
        "{}/tests/compilation_tests/simple_paragraph.mdm",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );
    let lines = fs::read_to_string(filename).unwrap().lines().count();
    assert_eq!(dbg!(lines), 4);

    let root = dbg!(env::var("CARGO_MANIFEST_DIR").unwrap());
    let tests = CompilationTest::find_tests_in_folder(&(root.clone() + "/tests/compilation_tests"));
    tests.iter().for_each(|t| t.run_test());

    test_file("simple_paragraph", &root);
}

fn test_file(name: &str, root: &str) {
    let mdm_path = format!("{root}/tests/compilation_tests/{name}.mdm");
    let mdm_file = fs::read_to_string(mdm_path).unwrap();
    let json_path = format!("{root}/tests/compilation_tests/{name}.json");
    let json_file = fs::read_to_string(json_path).unwrap();

    let mdm_obj = elem_to_json(&parse(&mdm_file));
    let json_obj = json::parse(&json_file).unwrap();

    assert_eq!(mdm_obj, json_obj);
}

struct CompilationTest {
    name: String,
    json_text: String,
    mdm_lines: Vec<String>,
}

impl CompilationTest {
    const MODMARK_FILE_EXT: &'static str = "mdm";
    const JSON_FILE_EXT: &'static str = "json";
    const UNIFIED_FILE_EXT: &'static str = "mdmtest";

    fn find_tests_in_folder(path: &str) -> Vec<CompilationTest> {
        fs::read_dir(path)
            .unwrap()
            .map(|f| dbg!(f.unwrap().path()))
            .filter_map(|p| {
                if let Some(ext) = p.extension() {
                    if ext == CompilationTest::MODMARK_FILE_EXT {
                        Some(CompilationTest::parse_split( p))
                    } else if ext == CompilationTest::UNIFIED_FILE_EXT {
                        Some(CompilationTest::parse_unified(p))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    fn parse_unified(path: PathBuf) -> CompilationTest {
        let file: Vec<String> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        let mdm_lines: Vec<String> = file
            .splitn(2, |x| x == "```mdm" || x == "```modmark")
            .nth(1)
            .unwrap()
            .splitn(2, |x| x == "```")
            .next()
            .unwrap()
            .into();

        let json_text: String = file
            .splitn(2, |x| x == "```json")
            .nth(1)
            .unwrap()
            .splitn(2, |x| x == "```")
            .next()
            .unwrap()
            .join("\n");

        dbg!(&mdm_lines, &json_text);

        CompilationTest {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            json_text,
            mdm_lines,
        }
    }

    fn parse_split(path: PathBuf) -> CompilationTest {
        // path is modmark path
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let mut path = path;
        let mdm_lines = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();
        path.set_extension(OsString::from(CompilationTest::JSON_FILE_EXT));
        let json_file = fs::read_to_string(&path).unwrap();

        CompilationTest {
            name,
            json_text: json_file,
            mdm_lines,
        }
    }

    fn run_test(&self) {
        println!("Running test for {}", self.name);

        let json_parse = json::parse(&self.json_text);
        assert!(
            json_parse.is_ok(),
            "Failed to parse json for test {}",
            self.name
        );
        let expected_json = json_parse.unwrap();

        let lf_body = self.mdm_lines.join("\n");
        let lf_elem = parse(&lf_body);
        let lf_json = elem_to_json(&lf_elem);
        assert_eq!(
            lf_json,
            expected_json,
            "Failed for test {} using LF, expected {} got {}",
            self.name,
            self.json_text,
            lf_json.dump()
        );

        let json_parse = json::parse(&self.json_text.replace("\\n","\\r\\n"));
        let expected_json = json_parse.unwrap();

        let crlf_body = self.mdm_lines.join("\r\n");
        let crlf_elem = parse(&crlf_body);
        let crlf_json = elem_to_json(&crlf_elem);
        assert_eq!(
            crlf_json,
            expected_json,
            "Failed for test {} using CRLF, expected {} got {}",
            self.name,
            self.json_text,
            crlf_json.dump()
        );
    }
}

fn elem_to_json(elem: &Element) -> JsonValue {
    match elem {
        Element::Data(str) => str.as_str().into(),
        Element::Node {
            name,
            environment: _environment,
            children,
        } => {
            object! {
                name: name.as_str(),
                children: children.iter().map(elem_to_json).collect::<Vec<JsonValue>>()
            }
        }
        Element::ModuleInvocation {
            name,
            args,
            body,
            one_line,
        } => {
            object! {
                name: name.as_str(),
                args: JsonValue::from(args.positioned.clone().unwrap_or_default().iter().enumerate().map(|(a,b)| (a.to_string(),b.to_string())).chain(
                    args.named.clone().unwrap_or_default().iter().map(|(a,b)| (a.to_string(), b.to_string()))
                ).collect::<HashMap<String, String>>()),
                body: body.as_str(),
                one_line: JsonValue::from(*one_line),
            }
        }
    }
}
