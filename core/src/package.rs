use crate::{error::CoreError, OutputFormat};
use serde::Deserialize;
use std::{io::Read, sync::Arc};
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

pub type NodeName = String;

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Transform {
    pub from: NodeName,
    pub to: Vec<OutputFormat>,
    pub arguments: Vec<ArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ArgInfo {
    pub name: String,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub info: Arc<PackageInfo>,
    pub wasm_module: Module,
}

impl Package {
    /// Read the binary data from a `.wasm` file and create a Package
    /// containing info about the package as well as the compiled wasm source module.
    pub fn new(wasm_source: &[u8], store: &mut Store) -> Result<Self, CoreError> {
        // Compile the module and store it
        #[cfg(feature = "native")]
        let module = Module::from_binary(store, wasm_source)?;

        #[cfg(feature = "web")]
        let module = Module::from_binary(store, wasm_source).expect("Web wasm compiler error");

        let input = Pipe::new();
        let mut output = Pipe::new();

        let wasi_env = WasiState::new("")
            .stdin(Box::new(input))
            .stdout(Box::new(output.clone()))
            .arg("manifest")
            .finalize(store)?;

        let import_object = wasi_env.import_object(store, &module)?;
        let instance = Instance::new(store, &module, &import_object)?;

        // Attach the memory export
        let memory = instance.exports.get_memory("memory")?;
        wasi_env.data_mut(store).set_memory(memory.clone());

        // Retrieve name of the package
        // Call the `name` function
        let manifest = instance.exports.get_function("_start")?;
        manifest.call(store, &[])?;

        // Read package info from stdin
        let manifest = {
            let mut buffer = String::new();
            output.read_to_string(&mut buffer)?;
            serde_json::from_str(&buffer)?
        };

        Ok(Package {
            info: Arc::new(manifest),
            wasm_module: module,
        })
    }
}

/// A helper to parse the output from the "transforms" function
/// when loading a package.
fn parse_transforms(input: &str) -> Option<Vec<Transform>> {
    let mut transforms = Vec::new();
    let mut lines = input.lines();

    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }
        // [foo] -> html tex
        let (raw_name, raw_outputs) = line.split_once("->")?;
        let name = raw_name.trim();
        let outputs: Vec<OutputFormat> = raw_outputs
            .split_whitespace()
            .map(|output| OutputFormat::new(output.trim()))
            .collect();

        // parse the following lines of arguments up until
        // the next blank line
        // foo = 20 - A description
        let mut arguments = Vec::new();

        for line in lines.by_ref() {
            if line.trim().is_empty() {
                break;
            }
            if let Some(arg) = parse_arg(line) {
                arguments.push(arg);
            };
        }

        transforms.push(Transform {
            from: name.to_string(),
            to: outputs,
            arguments,
        });
    }

    Some(transforms)
}

/// Parse an argument written like this
/// name = "optional default value" - Description
/// FIXME: this parser breaks on the following input
/// foo ="-" - description
fn parse_arg(input: &str) -> Option<ArgInfo> {
    let (lhs, description) = input.split_once('-')?;
    let name = lhs.split_whitespace().take(1).collect();
    let maybe_default = lhs.split_whitespace().skip(1).collect::<String>();
    let default = maybe_default.strip_prefix('=');

    Some(ArgInfo {
        name,
        description: description.trim().to_string(),
        default: default.map(|s| s.trim().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_argument_test() {
        let s = "x = 30 - The y position";
        assert_eq!(
            parse_arg(s),
            Some(ArgInfo {
                name: "x".to_string(),
                default: Some("30".to_string()),
                description: "The y position".to_string(),
            })
        );
    }

    #[test]
    fn parse_transforms_test() {
        let s = r#"[code] -> latex
                arg1 - This is a required positional
                ident = 4 - The amount of spaces to indent the block

                foo -> bar

                baz -> html
                a - Description for a
                b = 1 - Description for b
                c = 2 - Description for "#;

        assert_eq!(parse_transforms(s).is_some(), true);
    }
}
