use crate::error::CoreError;
use std::{io::Read, sync::Arc};
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

pub type NodeName = String;

/// Transform from a node into another node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Transform {
    pub from: NodeName,
    pub to: NodeName,
    pub arguments: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arg {
    pub name: String,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub info: Arc<ModuleInfo>,
    pub wasm_module: Module,
}

impl LoadedModule {
    /// Read the binary data from a `.wasm` file and create a LoadedModule
    /// containing info about the module as well as the compiled wasm source.
    pub fn new(wasm_source: &[u8], store: &mut Store) -> Result<Self, CoreError> {
        // Compile the module and store it
        #[cfg(feature = "native")]
        let module = Module::from_binary(store, wasm_source)?;

        #[cfg(feature = "web")]
        let module = Module::from_binary(store, wasm_source).expect("Web wasm compiler error");

        let input = Pipe::new();
        let mut output = Pipe::new();

        let wasi_env = WasiState::new("")
            .stdin(Box::new(input.clone()))
            .stdout(Box::new(output.clone()))
            .finalize(store)
            .map_err(CoreError::WasiStateCreation)?;

        let import_object = wasi_env
            .import_object(store, &module)
            .map_err(CoreError::WasiError)?;
        let instance = Instance::new(store, &module, &import_object)?;

        // Attach the memory export
        let memory = instance
            .exports
            .get_memory("memory")
            .map_err(CoreError::WasmerExport)?;
        wasi_env.data_mut(store).set_memory(memory.clone());

        // Retrieve name from module
        // Call the `name` function
        let name_fn = instance.exports.get_function("name")?;
        name_fn.call(store, &[])?;

        // Read the name from stdout
        let name = {
            let mut buffer = String::new();
            output
                .read_to_string(&mut buffer)
                .map_err(|_| CoreError::InvalidUTF8("unkown (when reading name)".to_string()))?;
            buffer.trim().to_string()
        };

        // Retrieve version from module
        // Call the `version` function
        let version_fn = instance.exports.get_function("version")?;
        version_fn.call(store, &[])?;

        // Read the version from stdout
        let version = {
            let mut buffer = String::new();
            output
                .read_to_string(&mut buffer)
                .map_err(|_| CoreError::InvalidUTF8(name.clone()))?;
            buffer.trim().to_string()
        };

        // Retrieve transform capabilities of module
        let transforms_fn = instance.exports.get_function("transforms")?;
        transforms_fn.call(store, &[])?;

        let raw_transforms_str = {
            let mut buffer = String::new();
            output
                .read_to_string(&mut buffer)
                .map_err(|_| CoreError::InvalidUTF8(name.clone()))?;
            buffer
        };

        let Some(transforms) = parse_transforms(&raw_transforms_str) else {
            return Err(CoreError::ParseTransforms(name))
        };

        let module_info = ModuleInfo {
            name,
            version,
            transforms,
        };

        Ok(LoadedModule {
            info: Arc::new(module_info),
            wasm_module: module,
        })
    }
}

/// A helper to parse the output from the "transforms" function
/// when loading a module.
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
        let outputs: Vec<String> = raw_outputs
            .split_whitespace()
            .map(|output| output.trim().to_string())
            .collect();

        // parse the following lines of arguments up until
        // the next blank line
        // foo = 20 - A description
        let mut arguments = Vec::new();

        while let Some(line) = lines.next() {
            if line.trim().is_empty() {
                break;
            }
            if let Some(arg) = parse_arg(line) {
                arguments.push(arg);
            };
        }

        // Add a tranform entry for each output
        for output in outputs {
            transforms.push(Transform {
                from: name.to_string(),
                to: output,
                arguments: arguments.clone(),
            });
        }
    }

    Some(transforms)
}

/// Parse an argument written like this
/// name = "optional default value" - Description
/// FIXME: this parser breaks on the following input
/// foo ="-" - description
fn parse_arg(input: &str) -> Option<Arg> {
    let (lhs, description) = input.split_once('-')?;
    let name = lhs.split_whitespace().take(1).collect();
    let maybe_default = lhs.split_whitespace().skip(1).collect::<String>();
    let default = maybe_default.strip_prefix('=');

    Some(Arg {
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
            Some(Arg {
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
