use std::fmt::Formatter;
use std::iter::once;
use std::{
    collections::HashMap,
    fmt,
    fmt::Debug,
    io::{Read, Write},
};

use either::{Either, Left};
use serde::{Deserialize, Serialize};
#[cfg(feature = "native")]
use wasmer::{Cranelift, Engine, EngineBuilder};
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use parser::ModuleArguments;

use crate::package::PackageImplementation;
use crate::{std_packages, Element};
use crate::{ArgInfo, CoreError, OutputFormat, Package, PackageInfo, Transform};

pub struct Context {
    pub(crate) packages: HashMap<String, Package>,
    pub(crate) transforms: HashMap<String, TransformVariant>,
    #[cfg(feature = "native")]
    engine: Engine,
    pub(crate) state: CompilationState,
}

#[derive(Default, Clone, Debug)]
pub struct CompilationState {
    pub warnings: Vec<Issue>,
    pub errors: Vec<Issue>,
    pub verbose_errors: bool,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub source: String,
    pub target: String,
    pub description: String,
    pub input: Option<String>,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(input) = &self.input {
            write!(
                f,
                "{} -> {}: {}, input: {}",
                self.source, self.target, self.description, input
            )
        } else {
            write!(
                f,
                "{} -> {}: {}",
                self.source, self.target, self.description
            )
        }
    }
}

impl CompilationState {
    fn clear(&mut self) {
        self.warnings.clear();
        self.errors.clear();
    }
}

impl Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("packages", &self.packages)
            .field("transforms", &self.transforms)
            .finish()
    }
}

/// This enum represents the different variants a transform can occur. Either a module/parent may be
/// transformed natively (in one way), or externally (possibly in different ways, depending on the
/// output format)
#[derive(Debug)]
pub enum TransformVariant {
    Native((Transform, Package)),
    External(HashMap<OutputFormat, (Transform, Package)>),
}

impl TransformVariant {
    /// This function finds the transform to an output format. If the transform is a native
    /// transform, that is returned regardless of the output format, but if it is external, the
    /// map is searched to find the appropriate transform
    pub(crate) fn find_transform_to(&self, format: &OutputFormat) -> Option<&(Transform, Package)> {
        match self {
            TransformVariant::Native(t) => Some(t),
            TransformVariant::External(map) => map.get(format),
        }
    }

    /// This function `.insert`s an entry to the map if this is of the `External` variant. If it
    /// is of the `Native` variant, this call does nothing.
    pub(crate) fn insert_into_external(
        &mut self,
        format: OutputFormat,
        entry: (Transform, Package),
    ) {
        match self {
            TransformVariant::Native(_) => {}
            TransformVariant::External(map) => {
                map.insert(format, entry);
            }
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            packages: HashMap::new(),
            transforms: HashMap::new(),
            #[cfg(feature = "native")]
            engine: EngineBuilder::new(Cranelift::new()).engine(),
            state: CompilationState::default(),
        }
    }

    /// Clears the internal `CompilationState` of this Context. This ensures that any information
    /// specific to previous compilations, such as errors and warnings, gets cleared.
    pub fn clear_state(&mut self) {
        self.state.clear();
    }

    /// Takes the internal `CompilationState` of this Context, and replacing it with
    /// a cleared out `CompilationState`
    pub fn take_state(&mut self) -> CompilationState {
        std::mem::take(&mut self.state)
    }

    /// This function loads the default packages to the Context. First, it loads all native
    /// packages, retrieved from `std_packages::native_package_list()`, and then it loads all
    /// standard packages by passing this Context to `std_packages::load_standard_packages()`
    pub fn load_default_packages(&mut self) -> Result<(), CoreError> {
        for pkg in std_packages::native_package_list() {
            self.load_package(Package::new_native(pkg)?)?
        }
        std_packages::load_standard_packages(self)?;
        Ok(())
    }

    /// This function loads the given package to this `Context`. It supports both native
    /// and external packages.
    pub fn load_package(&mut self, pkg: Package) -> Result<(), CoreError> {
        use crate::context::TransformVariant::{External, Native};

        self.packages
            .insert(pkg.info.as_ref().name.to_string(), pkg.clone());

        // Go through all transforms that the package supports and add them
        // to the Context.
        for transform in &pkg.info.transforms {
            let Transform {
                from,
                to,
                arguments: _,
            } = transform;

            // Check if the package is implemented natively or if it is an external package
            if pkg.implementation == PackageImplementation::Native {
                // If native => we don't have an output format, so map the key (mod/parent name) to
                // a `TransformVariant::Native`

                // First, assert that no transformations are registered for that key
                if self.transforms.contains_key(from) {
                    return Err(CoreError::OccupiedNativeTransform(
                        from.clone(),
                        pkg.info.name.clone(),
                    ));
                }

                // Insert the new `TransformVariant::Native` into the map
                self.transforms
                    .insert(from.clone(), Native((transform.clone(), pkg.clone())));
            } else {
                // We have an external package, loop though all output formats and register
                for output_format in to {
                    // Ensure that there are no other packages responsible for transforming to the
                    // same output format. Note that `find_transform_to` returns a native module
                    // if present, so this will fail if a native module is registered for that key
                    if self
                        .transforms
                        .get(from)
                        .and_then(|t| t.find_transform_to(output_format))
                        .is_some()
                    {
                        return Err(CoreError::OccupiedTransform(
                            from.clone(),
                            output_format.to_string(),
                            pkg.info.name.clone(),
                        ));
                    }

                    // Remove the target key from the map (which is either `External` or `None`)
                    // and then insert the new entry into the `external`
                    let mut target = self
                        .transforms
                        .remove(from)
                        .unwrap_or_else(|| External(HashMap::new()));
                    target.insert_into_external(
                        output_format.clone(),
                        (transform.clone(), pkg.clone()),
                    );
                    // Add the modified entry back to the map
                    self.transforms.insert(from.clone(), target);
                }
            }
        }

        Ok(())
    }

    /// This is a helper function to load a package directly from its wasm source. It will be
    /// compiled using `Package::new` to become a `Package` and then loaded using `load_package`
    pub fn load_package_from_wasm(&mut self, wasm_source: &[u8]) -> Result<(), CoreError> {
        #[cfg(feature = "native")]
        let pkg = Package::new(wasm_source, &self.engine)?;

        #[cfg(feature = "web")]
        let pkg = Package::new(wasm_source)?;

        self.load_package(pkg)
    }

    /// This gets the transform info for a given element and output format. If a native package
    /// supplies a transform for that element, that will be returned and the output format returned
    pub fn get_transform_info(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Option<&Transform> {
        self.transforms
            .get(element_name)
            .and_then(|t| t.find_transform_to(output_format))
            .map(|(transform, _)| transform)
    }

    /// Transform an Element by using the loaded packages. The function will return a
    /// `Element::Compound`.
    pub fn transform(
        &mut self,
        from: &Element,
        output_format: &OutputFormat,
    ) -> Result<Either<Element, String>, CoreError> {
        use Element::{Compound, Module, Parent};

        match from {
            Compound(_) => unreachable!("Should not transform compound element"),
            Parent {
                name,
                args: _,
                children: _,
            }
            | Module {
                name,
                args: _,
                body: _,
                inline: _,
            } => {
                let Some((_, package)) =
                    self.transforms.get(name).and_then(|t|t.find_transform_to(output_format))
                     else {
                    return Err(CoreError::MissingTransform(name.clone(), output_format.to_string()));
                };

                match &package.implementation {
                    PackageImplementation::Wasm(wasm_module) => {
                        // note: cloning modules is cheap
                        self.transform_from_wasm(wasm_module, name, from, output_format)
                    }
                    PackageImplementation::Native => self.transform_from_native(
                        &package.info.name.clone(),
                        name,
                        from,
                        output_format,
                    ),
                }
            }
        }
    }

    fn transform_from_native(
        &mut self,
        package_name: &str,
        node_name: &str, // name of module or parent
        element: &Element,
        output_format: &OutputFormat,
    ) -> Result<Either<Element, String>, CoreError> {
        let args = match element {
            Element::Parent {
                name,
                args,
                children: _,
            } => self.collect_parent_arguments(args, name, output_format),
            Element::Module {
                name,
                args,
                body: _,
                inline: _,
            } => self.collect_module_arguments(args, name, output_format),
            Element::Compound(_) => unreachable!("Cannot transform compound"),
        }?;

        std_packages::handle_native(self, package_name, node_name, element, args, output_format)
    }

    /// This function transforms an Element to another Element by invoking the Wasm module.
    fn transform_from_wasm(
        &self,
        module: &Module,
        name: &str,
        from: &Element,
        output_format: &OutputFormat,
    ) -> Result<Either<Element, String>, CoreError> {
        // Create a new store
        #[cfg(feature = "native")]
        let mut store = Store::new(&self.engine);

        #[cfg(feature = "web")]
        let mut store = Store::new();

        // Create pipes for stdin, stdout, stderr
        let mut input = Pipe::new();
        let mut output = Pipe::new();
        let mut err_out = Pipe::new();

        // Generate the input data (by serializing elements)
        let input_data = self.serialize_element(from, output_format)?;
        write!(&mut input, "{}", input_data)?;

        // Function to create an issue given a body text and if it is an error or not. This closure
        // captures references to the appropriate variables from this scope to generate correct
        // issues.
        let create_issue = |error: bool, body: String| -> Element {
            Element::Module {
                name: if error {
                    "error".to_string()
                } else {
                    "warning".to_string()
                },
                args: ModuleArguments {
                    positioned: None,
                    named: Some({
                        let mut map = HashMap::new();
                        map.insert("source".to_string(), name.to_string());
                        map.insert("target".to_string(), output_format.0.to_string());
                        // these two ifs can't be joined, unfortunately, or it won't run on stable
                        if self.state.verbose_errors {
                            map.insert("input".to_string(), input_data.to_string());
                        }
                        map
                    }),
                },
                body,
                inline: false,
            }
        };

        let wasi_env = WasiState::new("")
            .stdin(Box::new(input))
            .stdout(Box::new(output.clone()))
            .stderr(Box::new(err_out.clone()))
            .args(["transform", name, &output_format.to_string()])
            .finalize(&mut store)?;

        let import_object = wasi_env.import_object(&mut store, module)?;
        let instance = Instance::new(&mut store, module, &import_object)?;

        // Attach the memory export
        let memory = instance.exports.get_memory("memory")?;
        wasi_env.data_mut(&mut store).set_memory(memory.clone());

        // Call the main entry point of the program
        let main_fn = instance.exports.get_function("_start")?;
        let fn_res = main_fn.call(&mut store, &[]);

        if let Err(e) = fn_res {
            // An error occurred when executing Wasm module =>
            // it probably crashed, so just insert an error node
            return Ok(Left(create_issue(true, format!("Wasm module crash: {e}"))));
        }

        // Read the output of the package from stdout
        let result = {
            let mut buffer = String::new();
            output.read_to_string(&mut buffer)?;
            Self::deserialize_compound(&buffer)
        };

        // Read (possible) warnings and errors
        let err_str = {
            let mut buffer = String::new();
            err_out.read_to_string(&mut buffer)?;
            buffer
        };

        // If we have no stderr, just return the result early
        if err_str.is_empty() {
            return match result {
                // This is the only fully successful exit point, where we have a result and no
                // stderr => no errors/warnings logged
                Ok(res) => Ok(Left(res)),
                // If there is an issue in "result", the result was deserialized incorrectly.
                // The CoreError error message is misleading so we skip printing it and only print
                // our custom message
                Err(_) => Ok(Left(create_issue(
                    true,
                    "Error deserializing result from module".to_string(),
                ))),
            };
        }

        // If we have stderr, check if result is successful or not
        // If successful, we treat the messages in stderr as warnings
        // If not, we treat them as if they are errors
        if let Ok(elem) = result {
            let elems = err_str
                .lines()
                .map(|line| create_issue(false, format!("Logged warning: {line}")))
                // Here, in the warnings case, chain the result and emit it as well
                .chain(once(elem))
                .collect();
            Ok(Left(Element::Compound(elems)))
        } else {
            let errors = err_str
                .lines()
                .map(|line| create_issue(true, format!("Logged error: {line}")))
                .collect();
            Ok(Left(Element::Compound(errors)))
        }
    }

    /// Borrow information about a package with a given name
    pub fn get_package_info(&self, name: &str) -> Option<&PackageInfo> {
        self.packages.get(name).map(|pkg| pkg.info.as_ref())
    }

    /// Serialize and element into a string that can be sent to a package
    pub fn serialize_element(
        &self,
        element: &Element,
        output_format: &OutputFormat,
    ) -> Result<String, CoreError> {
        let entry = self.element_to_entry(element, output_format)?;
        serde_json::to_string_pretty(&entry).map_err(|e| e.into())
    }

    /// Deserialize a compound (i.e a list of `JsonEntries`) that are recived from a package
    pub fn deserialize_compound(input: &str) -> Result<Element, CoreError> {
        let entries: Vec<JsonEntry> =
            serde_json::from_str(input).map_err(|error| CoreError::DeserializationError {
                string: input.to_string(),
                error,
            })?;

        // Convert the parsed entries into real Elements
        let elements: Vec<Element> = entries.into_iter().map(Self::entry_to_element).collect();
        Ok(Element::Compound(elements))
    }

    /// Convert a `JsonEntry` to an `Element`
    fn entry_to_element(entry: JsonEntry) -> Element {
        match entry {
            JsonEntry::ParentNode {
                name,
                arguments,
                children,
            } => Element::Parent {
                name,
                args: arguments,
                children: children.into_iter().map(Self::entry_to_element).collect(),
            },
            JsonEntry::Module {
                name,
                data,
                arguments,
                inline,
            } => Element::Module {
                name,
                args: ModuleArguments {
                    positioned: None,
                    named: Some(arguments),
                },
                body: data,
                inline,
            },
        }
    }

    /// Convert an `Element` into a `JsonEntry`.
    fn element_to_entry(
        &self,
        element: &Element,
        output_format: &OutputFormat,
    ) -> Result<JsonEntry, CoreError> {
        match element {
            // When the eval function naivly evaluates all children before a parent compund
            // nodes should never be present here. This may however change in the future.
            Element::Compound(_) => unreachable!(),
            Element::Parent {
                name,
                args,
                children,
            } => {
                let converted_children: Result<Vec<JsonEntry>, CoreError> = children
                    .iter()
                    .map(|child| self.element_to_entry(child, output_format))
                    .collect();

                let collected_args = self.collect_parent_arguments(args, name, output_format)?;

                Ok(JsonEntry::ParentNode {
                    name: name.clone(),
                    arguments: collected_args,
                    children: converted_children?,
                })
            }
            Element::Module {
                name: module_name,
                args,
                body,
                inline: one_line,
            } => {
                let collected_args =
                    self.collect_module_arguments(args, module_name, output_format)?;

                Ok(JsonEntry::Module {
                    name: module_name.clone(),
                    arguments: collected_args,
                    data: body.clone(),
                    inline: *one_line,
                })
            }
        }
    }

    fn collect_parent_arguments(
        &self,
        args: &HashMap<String, String>,
        parent_name: &str,
        output_format: &OutputFormat,
    ) -> Result<HashMap<String, String>, CoreError> {
        // Collect the arguments and add default values for unspecifed arguments
        let mut collected_args = HashMap::new();
        let mut given_args = args.clone();

        // Get info about what args this parent node
        let empty_vec = vec![];
        let args_info: &Vec<ArgInfo> = self
            .get_transform_info(parent_name, output_format)
            .map_or(&empty_vec, |info| info.arguments.as_ref());

        for arg_info in args_info {
            let ArgInfo {
                name,
                default,
                description: _,
            } = arg_info;

            if let Some(value) = given_args.remove(name) {
                collected_args.insert(name.clone(), value);
                continue;
            }

            if let Some(value) = default {
                collected_args.insert(name.clone(), value.clone());
            }

            return Err(CoreError::MissingArgument(
                name.clone(),
                parent_name.to_owned(),
            ));
        }

        // Check if there are any stray arguments left that should not be there
        if let Some((key, _)) = given_args.into_iter().next() {
            return Err(CoreError::InvalidArgument(key, parent_name.to_owned()));
        }

        Ok(collected_args)
    }

    fn collect_module_arguments(
        &self,
        args: &ModuleArguments,
        module_name: &str,
        output_format: &OutputFormat,
    ) -> Result<HashMap<String, String>, CoreError> {
        let empty_vec = vec![];
        let mut pos_args = args.positioned.as_ref().unwrap_or(&empty_vec).iter();
        let mut named_args = args.named.clone().unwrap_or_default();
        let mut collected_args = HashMap::new();

        // Get info about what args this parent node supports
        let empty_vec = vec![];
        let args_info = self
            .get_transform_info(module_name, output_format)
            .map_or(&empty_vec, |info| info.arguments.as_ref());

        for arg_info in args_info {
            let ArgInfo {
                name,
                default,
                description: _,
            } = arg_info;

            // First empty the positional arguments
            if let Some(value) = pos_args.next() {
                // Check that this key is not repeated later too
                if named_args.contains_key(name) {
                    return Err(CoreError::RepeatedArgument(
                        name.to_string(),
                        module_name.to_string(),
                    ));
                }
                collected_args.insert(name.to_string(), value.clone());
                continue;
            }

            // Check if it was specified as a named key=value pair
            if let Some(value) = named_args.remove(name) {
                collected_args.insert(name.to_string(), value.clone());
                continue;
            }

            // Use the default value as a fallback
            if let Some(value) = default {
                collected_args.insert(name.to_string(), value.clone());
                continue;
            }

            // Oops, the user seem to be missing a required argument
            return Err(CoreError::MissingArgument(
                name.to_string(),
                module_name.to_string(),
            ));
        }

        if let Some(value) = pos_args.next() {
            return Err(CoreError::InvalidArgument(
                value.to_string(),
                module_name.to_string(),
            ));
        }

        if let Some((key, _)) = named_args.iter().next() {
            return Err(CoreError::InvalidArgument(
                key.clone(),
                module_name.to_string(),
            ));
        }
        Ok(collected_args)
    }
}

impl Default for Context {
    /// A Context with all default packages lodaded
    fn default() -> Self {
        let mut ctx = Self::new();
        ctx.load_default_packages().unwrap();
        ctx
    }
}

/// This enum is in the same shape as the json objects that
/// will be sent and recieved when communicating with packages
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum JsonEntry {
    ParentNode {
        name: String,
        arguments: HashMap<String, String>,
        children: Vec<Self>,
    },
    Module {
        name: String,
        #[serde(default)]
        data: String,
        #[serde(default)]
        arguments: HashMap<String, String>,
        #[serde(default = "default_inline")]
        inline: bool,
    },
}

/// This is just a helper to ensure that omited "inline" fields
/// default to true.
fn default_inline() -> bool {
    true
}
