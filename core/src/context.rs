use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Formatter;
use std::iter::once;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::{
    collections::HashMap,
    fmt,
    fmt::Debug,
    io::{Read, Write},
};

use crate::package::{ArgValue, PackageImplementation};
use crate::{std_packages, AccessPolicy, DefaultAccessManager, DenyAllResolver, Element, Resolve};
use crate::{ArgInfo, CoreError, OutputFormat, Package, PackageInfo, Transform};

use either::{Either, Left};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "native")]
use wasmer::{Cranelift, Engine, EngineBuilder};
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use crate::fs::CoreFs;
use parser::config::{Config, HideConfig, ImportConfig};
use parser::ModuleArguments;

use crate::package::{ArgValue, PackageImplementation};
use crate::package_manager::{DenyAllResolver, PackageID, PackageManager, Resolve};
use crate::{std_packages, Element};
use crate::{ArgInfo, CoreError, OutputFormat, Package, PackageInfo, Transform};

pub struct Context<T,U> {
    pub package_manager: Arc<Mutex<PackageManager>>,
    pub(crate) resolver: T,
    #[cfg(feature = "native")]
    engine: Engine,
    pub(crate) state: CompilationState,
    pub filesystem: CoreFs<U>,
    policy: Arc<Mutex<U>>,
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

impl<T, U> Debug for Context<T, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            //            .field("native packages", &self.native_packages)
            //            .field("standard packages", &self.standard_packages)
            //            .field("external packages", &self.external_packages)
            //            .field("transforms", &self.transforms)
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

impl Context<DenyAllResolver, DefaultAccessManager> {
    pub fn default() -> Result<Self, CoreError> {
        Self::new(DenyAllResolver, DefaultAccessManager)
    }
}

impl<T, U> Context<T, U> {
    pub fn new(resolver: T, policy: U) -> Result<Self, CoreError>
    where
        T: Resolve,
        U: AccessPolicy,
    {
        let policy = Arc::new(Mutex::new(policy));
        let mut ctx = Context {
            package_manager: Arc::default(),
            resolver,
            #[cfg(feature = "native")]
            engine: EngineBuilder::new(Cranelift::new()).engine(),
            state: CompilationState::default(),
            filesystem: CoreFs::new(Arc::clone(&policy)),
            policy,
        };
        ctx.load_default_packages()?;
        Ok(ctx)
    }
}

impl<T, U> Context<T, U>
where
    T: Resolve,
{
    /*pub(crate) fn import_missing_packages(&mut self, config: &Config) -> Result<(), CoreError> {
        let missing: Vec<&str> = config
            .imports
            .iter()
            .map(|i| i.name.as_str())
            .chain(config.hides.iter().map(|h| h.name.as_str()))
            .filter(|&name| {
                !self.standard_packages.contains_key(name)
                    && !self.external_packages.contains_key(name)
            })
            .collect();

        // The .enumerate()-.map() may seem ugly but it is needed to be able to retrieve the
        // package which errored (first). Essentially, instead of having just an dyn Error, we have
        // (idx, dyn Error) where idx is the idx to the package (in missing) that errored
        let resolves: Vec<Vec<u8>> = self
            .resolver
            .resolve_all(&missing)
            .into_iter()
            .enumerate()
            .map(|(idx, pkg)| pkg.map_err(|e| (idx, e)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|(idx, e)| CoreError::Resolve(missing[idx].to_string(), Box::new(e)))?;

        let res: Result<Vec<()>, CoreError> = missing
            .into_iter()
            .zip(resolves.into_iter())
            .map(|(name, data)| self.load_external_package(name, data.as_slice()))
            .collect();
        res.map(|_| ())
    }*/

    // This function configures the context with the given config, so that it is appropriate to
    // evaluate a document having that configuration with it. It also resolves packages if needed
    // If this returns "true", it had everything it needed to compile, if "false" it is waiting for
    // more packages
    pub(crate) fn configure(&mut self, config: Option<Config>) -> Result<bool, CoreError> {
        let config = config.unwrap_or_default();
        let mut lock = self.package_manager.lock().unwrap();

        #[cfg(feature = "native")]
        lock.finalize(&self.engine).unwrap();

        #[cfg(feature = "web")]
        lock.finalize().unwrap();

        let arc_mutex = Arc::clone(&self.package_manager);
        let missings = lock.get_missing_packages(arc_mutex, &config);
        println!("Missings: {:?}", missings);
        if missings.is_empty() {
            lock.expose_transforms(config.try_into()?)?;
            Ok(true)
        } else {
            self.resolver.resolve_all(missings);
            Ok(false)
        }
    }
}

impl<T, U> Context<T, U>
where
    U: AccessPolicy,
{
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
                let Some((_, package)) = self.get_transform_to(name, output_format) else {
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

    //noinspection RsLiveness
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

        // Create pipes for stdin, stdwasmerout, stderr
        let mut input = Pipe::new();
        let mut output = Pipe::new();
        let mut err_out = Pipe::new();

        // Generate the input data (by serializing elements)
        let input_data = self.serialize_element(from, output_format)?;
        write!(&mut input, "{}", input_data)?;

        // Function to create an issue given a body text and if it is an error or not. This closure
        // captures references to the appropriate variables from this scope to generate correct
        // issues.
        let create_issue = |error: bool, body: String, data: &str| -> Element {
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
                            map.insert("input".to_string(), data.to_string());
                        }
                        map
                    }),
                },
                body,
                inline: false,
            }
        };

        let fs = self.filesystem.clone_for_module(name.to_string());

        // check the access policy
        let (read, write, create, root) = {
            let policy = self.policy.lock().unwrap();
            (
                policy.allowed_to_read(),
                policy.allowed_to_write(),
                policy.allowed_to_create(),
                policy.root(),
            )
        };

        let wasi_env = if root.is_none() || !(read || write || create) {
            WasiState::new("")
                .stdin(Box::new(input))
                .stdout(Box::new(output.clone()))
                .stderr(Box::new(err_out.clone()))
                .args(["transform", name, &output_format.to_string()])
                .finalize(&mut store)?
        } else {
            let path = Path::new(root.as_ref().unwrap());
            WasiState::new("")
                .stdin(Box::new(input))
                .stdout(Box::new(output.clone()))
                .stderr(Box::new(err_out.clone()))
                .set_fs(Box::new(fs))
                .preopen(|p| {
                    p.directory(path)
                        .alias(".")
                        .read(read)
                        .write(write)
                        .create(create)
                })?
                .args(["transform", name, &output_format.to_string()])
                .finalize(&mut store)?
        };

        let import_object = wasi_env.import_object(&mut store, module)?;
        let instance = Instance::new(&mut store, module, &import_object)?;

        // Attach the memory export
        let memory = instance.exports.get_memory("memory")?;
        wasi_env.data_mut(&mut store).set_memory(memory.clone());

        // Call the main entry point of the program
        let main_fn = instance
            .exports
            .get_function("_start")
            .expect("Unable to find main function");
        let fn_res = main_fn.call(&mut store, &[]);

        if let Err(e) = fn_res {
            // TODO: See if this can be done without string comparison
            let error_msg = e.to_string();
            if !error_msg.contains("WASI exited with code: 0") {
                // An error occurred when executing Wasm module =>
                // it probably crashed, so just insert an error node
                return Ok(Left(create_issue(
                    true,
                    format!("Wasm module crash: {error_msg}"),
                    &input_data,
                )));
            }
        }

        // Read (possible) warnings and errors
        let err_str = {
            let mut buffer = String::new();
            err_out.read_to_string(&mut buffer)?;
            buffer
        };

        let result = {
            let mut buffer = String::new();
            output.read_to_string(&mut buffer)?;
            Self::deserialize_compound(&buffer)
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
                    &input_data,
                ))),
            };
        }

        // If we have stderr, check if result is successful or not
        // If successful, we treat the messages in stderr as warnings
        // If not, we treat them as if they are errors
        if let Ok(elem) = result {
            let elems = err_str
                .lines()
                .map(|line| create_issue(false, format!("Logged warning: {line}"), &input_data))
                // Here, in the warnings case, chain the result and emit it as well
                .chain(once(elem))
                .collect();
            Ok(Left(Element::Compound(elems)))
        } else {
            let errors = err_str
                .lines()
                .map(|line| create_issue(true, format!("Logged error: {line}"), &input_data))
                .collect();
            Ok(Left(Element::Compound(errors)))
        }
    }
}

impl<T, U> Context<T, U> {
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
    /// standard packages by passing this Context to `std_packages::load_standard_packages()`. This
    /// will be run when constructing the Context, and may only be run once.
    fn load_default_packages(&mut self) -> Result<(), CoreError> {
        for pkg in std_packages::native_package_list() {
            self.load_native_package(Package::new_native(pkg)?)?
        }
        std_packages::load_standard_packages(self)?;
        Ok(())
    }

    pub fn load_native_package(&mut self, pkg: Package) -> Result<(), CoreError> {
        debug_assert_eq!(pkg.implementation, PackageImplementation::Native);
        let name = &pkg.info.as_ref().name;
        let mut lock = self.package_manager.lock().unwrap();
        let entry = lock.native_packages.entry(name.to_string());

        match entry {
            Entry::Occupied(_) => return Err(CoreError::OccupiedName(name.to_string())),
            Entry::Vacant(entry) => entry.insert(pkg),
        };
        Ok(())
    }

    /// This is a helper function to load a package directly from its wasm source. It will be
    /// compiled using `Package::new` to become a `Package` and then loaded using `load_package`
    pub fn load_external_package(
        &mut self,
        external_name: &str,
        wasm_source: &[u8],
    ) -> Result<(), CoreError> {
        if wasm_source.is_empty() {
            return Ok(());
        }
        let pkg = self.package_from_wasm(wasm_source)?;
        debug_assert_ne!(pkg.implementation, PackageImplementation::Native);
        let mut lock = self.package_manager.lock().unwrap();
        let entry = lock
            .external_packages
            .entry(PackageID::from_str(external_name).unwrap());

        match entry {
            Entry::Occupied(_) => return Err(CoreError::OccupiedName(external_name.to_string())),
            Entry::Vacant(entry) => entry.insert(pkg),
        };
        Ok(())
    }

    /// This is a helper function to load a package directly from its wasm source. It will be
    /// compiled using `Package::new` to become a `Package` and then loaded using `load_package`
    #[allow(dead_code)]
    pub(crate) fn load_standard_package(&mut self, wasm_source: &[u8]) -> Result<(), CoreError> {
        let pkg = self.package_from_wasm(wasm_source)?;

        let name = pkg.info.name.as_str();
        let mut lock = self.package_manager.lock().unwrap();
        let entry = lock.standard_packages.entry(name.into());

        match entry {
            Entry::Occupied(_) => Err(CoreError::OccupiedName(name.to_string())),
            Entry::Vacant(entry) => {
                entry.insert(pkg);
                Ok(())
            }
        }
    }

    #[allow(unreachable_code)]
    fn package_from_wasm(&mut self, wasm_source: &[u8]) -> Result<Package, CoreError> {
        #[cfg(feature = "native")]
        return Package::new(wasm_source, &self.engine);

        #[cfg(feature = "web")]
        return Package::new(wasm_source);

        #[cfg(not(any(feature = "native", feature = "web")))]
        compile_error!("'native' and 'web' features are both disabled")
    }

    /// This function loads a package from the serialized format retrieved from Module::serialize.
    /// This is only available on native when using the precompile_wasm feature.
    #[cfg(all(feature = "native", feature = "precompile_wasm"))]
    pub(crate) fn load_precompiled_package_from_wasm(
        &mut self,
        wasm_source: &[u8],
    ) -> Result<(), CoreError> {
        let pkg = Package::new_precompiled(wasm_source, &self.engine)?;

        let name = pkg.info.name.as_str();
        let mut lock = self.package_manager.lock().unwrap();
        let entry = lock.standard_packages.entry(name.into());

        match entry {
            Entry::Occupied(_) => Err(CoreError::OccupiedName(name.to_string())),
            Entry::Vacant(entry) => {
                entry.insert(pkg);
                Ok(())
            }
        }
    }

    /// Gets the transform and package the transform is in, for a transform from a specific element
    /// to a specific output format. Returns None if no such transform exists
    pub fn get_transform_to(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Option<(Transform, Package)> {
        let lock = self.package_manager.lock().unwrap();
        lock.transforms
            .get(element_name)
            .and_then(|t| t.find_transform_to(output_format))
            .cloned()
    }

    /// This gets the transform info for a given element and output format. If a native package
    /// supplies a transform for that element, that will be returned and the output format returned
    pub fn get_transform_info(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Option<Transform> {
        self.get_transform_to(element_name, output_format)
            .map(|(transform, _)| transform)
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

    /// Borrow information about a package with a given name
    pub fn get_package_info(&self, name: &str) -> Option<Arc<PackageInfo>> {
        let mut lock = self.package_manager.lock().unwrap();
        lock.native_packages
            .get(name)
            .or(lock
                .standard_packages
                .get(&PackageID::from_str(name).unwrap()))
            .map(|pkg| pkg.info.clone())
    }

    /// Borrow a vector with PackageInfo from every loaded package
    pub fn get_all_package_info(&self) -> Vec<Arc<PackageInfo>> {
        let lock = self.package_manager.lock().unwrap();
        lock.native_packages
            .values()
            .chain(lock.standard_packages.values())
            .chain(lock.external_packages.values())
            .map(|pkg| pkg.info.clone())
            .collect::<Vec<_>>()
            .clone()
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

    /// Deserialize a compound (i.e a list of `JsonEntries`) that are received from a package
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
        let type_erase = |mut map: HashMap<String, Value>| {
            map.drain()
                .map(|(k, v)| {
                    (
                        k,
                        if let Value::String(s) = v {
                            s
                        } else {
                            v.to_string()
                        },
                    )
                })
                .collect::<HashMap<String, String>>()
        };

        match entry {
            JsonEntry::ParentNode {
                name,
                arguments,
                children,
            } => Element::Parent {
                name,
                args: type_erase(arguments),
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
                    named: Some(type_erase(arguments)),
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
            // When the eval function naively evaluates all children before a parent compound
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

                let mut collected_args =
                    self.collect_parent_arguments(args, name, output_format)
                        .map_err(|e| CoreError::SerializeElement(name.to_string(), Box::new(e)))?;
                let type_erased_args = collected_args.drain().map(|(k, v)| (k, v.into())).collect();

                Ok(JsonEntry::ParentNode {
                    name: name.clone(),
                    arguments: type_erased_args,
                    children: converted_children?,
                })
            }
            Element::Module {
                name,
                args,
                body,
                inline: one_line,
            } => {
                let mut collected_args =
                    self.collect_module_arguments(args, name, output_format)
                        .map_err(|e| CoreError::SerializeElement(name.to_string(), Box::new(e)))?;
                let type_erased_args = collected_args.drain().map(|(k, v)| (k, v.into())).collect();

                Ok(JsonEntry::Module {
                    name: name.clone(),
                    arguments: type_erased_args,
                    data: body.clone(),
                    inline: *one_line,
                })
            }
        }
    }

    /// Gets the `ArgInfo`s associated with an element targeting the given output format, if such
    /// a transformation exists, otherwise generates an `MissingTransform` error. This is intended
    /// for use in `collect_(parent/module)_arguments` to reduce repeated code.
    fn get_args_info(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Result<Vec<ArgInfo>, CoreError> {
        self.get_transform_info(element_name, output_format)
            .map(|info| info.arguments.clone())
            .ok_or(CoreError::MissingTransform(
                element_name.to_string(),
                output_format.0.to_string(),
            ))
    }

    fn collect_parent_arguments(
        &self,
        args: &HashMap<String, String>,
        parent_name: &str,
        output_format: &OutputFormat,
    ) -> Result<HashMap<String, ArgValue>, CoreError> {
        // Collect the arguments and add default values for unspecified arguments
        let mut collected_args = HashMap::new();
        let mut given_args = args.clone();

        // Get info about what args this parent node
        let args_info = self.get_args_info(parent_name, output_format)?;

        for arg_info in args_info {
            let ArgInfo {
                name,
                default,
                description: _,
                r#type,
            } = arg_info;

            if let Some(value) = given_args.remove(&name) {
                let value = r#type.try_from_str(&value)?;
                collected_args.insert(name.clone(), value);
                continue;
            }

            if let Some(value) = default {
                collected_args.insert(name.clone(), r#type.try_from_value(&value)?);
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
    ) -> Result<HashMap<String, ArgValue>, CoreError> {
        let empty_vec = vec![];
        let mut pos_args = args.positioned.as_ref().unwrap_or(&empty_vec).iter();
        let mut named_args = args.named.clone().unwrap_or_default();
        let mut collected_args = HashMap::new();

        // Get info about what args this parent node supports
        let args_info = self.get_args_info(module_name, output_format)?;

        for arg_info in args_info {
            let ArgInfo {
                name,
                default,
                description: _,
                r#type,
            } = arg_info;

            // First empty the positional arguments
            if let Some(value) = pos_args.next() {
                // Check that this key is not repeated later too
                if named_args.contains_key(&name) {
                    return Err(CoreError::RepeatedArgument(
                        name.to_string(),
                        module_name.to_string(),
                    ));
                }
                let value = r#type.try_from_str(value)?;
                collected_args.insert(name.to_string(), value);
                continue;
            }

            // Check if it was specified as a named key=value pair
            if let Some(value) = named_args.remove(&name) {
                let value = r#type.try_from_str(&value)?;
                collected_args.insert(name.to_string(), value);
                continue;
            }

            // Use the default value as a fallback
            if let Some(value) = default {
                collected_args.insert(name.to_string(), r#type.try_from_value(&value)?);
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

/// This enum is in the same shape as the json objects that
/// will be sent and received when communicating with packages
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum JsonEntry {
    ParentNode {
        name: String,
        arguments: HashMap<String, Value>,
        children: Vec<Self>,
    },
    Module {
        name: String,
        #[serde(default)]
        data: String,
        #[serde(default)]
        arguments: HashMap<String, Value>,
        #[serde(default = "default_inline")]
        inline: bool,
    },
}

#[derive(Default)]
#[repr(transparent)]
pub(crate) struct ModuleImport(pub(crate) HashMap<PackageID, ModuleImportConfig>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModuleImportConfig {
    ImportAll,
    Include(Vec<String>),
    Exclude(Vec<String>),
    HideAll,
}

impl From<HideConfig> for ModuleImportConfig {
    fn from(value: HideConfig) -> Self {
        match value {
            HideConfig::HideAll => ModuleImportConfig::HideAll,
        }
    }
}

impl From<ImportConfig> for ModuleImportConfig {
    fn from(value: ImportConfig) -> Self {
        match value {
            ImportConfig::ImportAll => Self::ImportAll,
            ImportConfig::Include(vec) => Self::Include(vec),
            ImportConfig::Exclude(vec) => Self::Exclude(vec),
        }
    }
}

impl TryFrom<Config> for ModuleImport {
    type Error = CoreError;

    fn try_from(value: Config) -> Result<Self, Self::Error> {
        let Config {
            imports,
            hides,
            sets: _,
        } = value;
        let mut found = HashSet::new();
        let mut duplicates = vec![];
        let entries = imports
            .into_iter()
            .map(|import| (PackageID::from(import.name), import.importing.into()))
            .chain(
                hides
                    .into_iter()
                    .map(|hide| (PackageID::from(hide.name), hide.hiding.into())),
            )
            .inspect(|(name, _)| {
                if !found.insert(name.clone()) {
                    duplicates.push(name.clone());
                }
            })
            .collect();

        if !duplicates.is_empty() {
            Err(CoreError::DuplicateConfigs(
                duplicates.into_iter().map(|x| x.name).collect(),
            ))
        } else {
            Ok(ModuleImport(entries))
        }
    }
}

/// This is just a helper to ensure that omitted "inline" fields
/// default to true.
fn default_inline() -> bool {
    true
}
