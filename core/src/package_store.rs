use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::error::Error;
use std::hash::Hash;
use std::mem;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[cfg(feature = "native")]
use wasmer::Engine;

use parser::config::{Config, Hide, Import};

use crate::context::{ModuleImport, ModuleImportConfig, TransformVariant};
use crate::package::PackageImplementation;
use crate::{std_packages, ArgInfo, CoreError, OutputFormat, Package, PackageInfo, Transform};

// The package_new allows us to run Package::new(source, [engine]) by supplying the identifier to
// the engine that would have been used if we were compiling to native, which will be ignored
// if we aren't compiling to native
#[cfg(feature = "native")]
macro_rules! package_new {
    ($source:expr, $engine:tt) => {
        Package::new($source, $engine)
    };
}

#[cfg(not(feature = "native"))]
macro_rules! package_new {
    ($source:expr, $engine:tt) => {
        Package::new($source)
    };
}

#[derive(Debug, Default)]
pub struct PackageStore {
    pub(crate) native_packages: HashMap<String, Package>,
    pub(crate) standard_packages: HashMap<PackageID, Package>,
    pub(crate) external_packages: HashMap<PackageID, Package>,
    pub(crate) awaited_packages: HashSet<PackageID>,
    pub(crate) new_packages: HashMap<PackageID, Vec<u8>>,
    pub(crate) package_task_failures: Vec<CoreError>,
    pub(crate) transforms: HashMap<String, TransformVariant>,
}

impl PackageStore {
    /// Clears the loaded external packages and transforms, possibly saving memory, forcing the
    /// Resolve to fetch any external packages again, essentially clearing the internal package
    /// cache of the PackageStore
    pub fn clear_packages(&mut self) {
        self.transforms.clear();
        self.external_packages.clear();
    }

    /// This function takes all packages that has been resolved but not registered yet, that are
    /// currently stored in `new_packages`, and compiles them using `Package::new` and adds them to
    /// the list of registered external packages. If any of those packages fails to compile, this
    /// function will return `Err()` with all the errors. All successfully compiled packages will
    /// still be added to the `external_packages` map.
    pub fn register_resolved_packages(
        &mut self,
        #[cfg(feature = "native")] engine: &Engine,
    ) -> Result<(), Vec<CoreError>> {
        // First, get all successful fetches and add them to external_packages, propagating any
        // error when creating the package itself
        let (successes, failures) = self
            .new_packages
            .drain()
            .map(|(k, v)| package_new!(v.as_slice(), engine).map(|p| (k, p)))
            .fold((vec![], vec![]), |(mut s, mut f), res| {
                match res {
                    Ok(ok) => s.push(ok),
                    Err(err) => f.push(err),
                };
                (s, f)
            });

        for (package_name, package) in successes {
            self.external_packages.insert(package_name, package);
        }

        // Here we check if any packages failed to compile (not if they failed to resolve)
        if !failures.is_empty() {
            return Err(failures);
        }

        // Then, clear all failed packages and if any failed, return that error
        // Note that "drain" here drains all errors, not just the one we (possibly) return
        let failed = mem::take(&mut self.package_task_failures);
        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed)
        }
    }

    /// This function loads the default packages to the PackageStore. First, it loads all native
    /// packages, retrieved from `std_packages::native_package_list()`, and then it loads all
    /// standard packages by passing this PackageStore to `std_packages::load_standard_packages()`. This
    /// will be run when constructing the PackageStore, and may only be run once.
    pub(crate) fn load_default_packages(
        &mut self,
        #[cfg(feature = "native")] engine: &Engine,
    ) -> Result<(), CoreError> {
        for pkg in std_packages::native_package_list() {
            self.load_native_package(Package::new_native(pkg)?)?
        }
        #[cfg(feature = "native")]
        std_packages::load_standard_packages(self, engine)?;
        #[cfg(not(feature = "native"))]
        std_packages::load_standard_packages(self)?;
        Ok(())
    }

    /// This function loads a package from the serialized format retrieved from Module::serialize.
    /// This is only available on native when using the precompile_wasm feature.
    #[cfg(all(feature = "native", feature = "precompile_wasm"))]
    pub(crate) fn load_precompiled_standard_package(
        &mut self,
        wasm_source: &[u8],
        engine: &Engine,
    ) -> Result<(), CoreError> {
        let pkg = Package::new_precompiled(wasm_source, engine)?;
        self.insert_standard_package(pkg)
    }

    /// This is a helper function to load a package directly from its wasm source. It will be
    /// compiled using `Package::new` to become a `Package` and then loaded using `load_package`
    #[allow(dead_code)]
    pub(crate) fn load_standard_package(
        &mut self,
        wasm_source: &[u8],
        #[cfg(feature = "native")] engine: &Engine,
    ) -> Result<(), CoreError> {
        let pkg = package_new!(wasm_source, engine)?;
        self.insert_standard_package(pkg)
    }

    /// This function tries to insert a standard package into the `standard_packages` map
    fn insert_standard_package(&mut self, pkg: Package) -> Result<(), CoreError> {
        let name = pkg.info.name.as_str();
        let id = PackageID {
            name: name.to_string(),
            source: PackageSource::Standard,
        };
        let entry = self.standard_packages.entry(id);

        match entry {
            Entry::Occupied(_) => Err(CoreError::OccupiedName(name.to_string())),
            Entry::Vacant(entry) => {
                entry.insert(pkg);
                Ok(())
            }
        }
    }

    pub(crate) fn load_native_package(&mut self, pkg: Package) -> Result<(), CoreError> {
        debug_assert_eq!(pkg.implementation, PackageImplementation::Native);
        let name = &pkg.info.as_ref().name;
        let entry = self.native_packages.entry(name.to_string());

        match entry {
            Entry::Occupied(_) => return Err(CoreError::OccupiedName(name.to_string())),
            Entry::Vacant(entry) => entry.insert(pkg),
        };
        Ok(())
    }

    /// Gets information about a package with a given name
    pub fn get_package_info(&self, name: &str) -> Option<Arc<PackageInfo>> {
        self.native_packages
            .get(name)
            .or(self.standard_packages.get(&name.into()))
            .map(|pkg| pkg.info.clone())
    }

    /// Borrow a vector with PackageInfo from every loaded package
    pub fn get_all_package_info(&self) -> Vec<Arc<PackageInfo>> {
        self.external_packages
            .values()
            .chain(self.standard_packages.values())
            .chain(self.native_packages.values())
            .map(|pkg| pkg.info.clone())
            .collect::<Vec<_>>()
    }

    /// Gets the `ArgInfo`s associated with an element targeting the given output format, if such
    /// a transformation exists, otherwise generates an `MissingTransform` error. This is intended
    /// for use in `collect_(parent/module)_arguments` to reduce repeated code.
    pub(crate) fn get_args_info(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Result<Vec<ArgInfo>, CoreError> {
        self.find_transform(element_name, output_format)
            .map(|(transform, _)| transform.arguments)
            .ok_or(CoreError::MissingTransform(
                element_name.to_string(),
                output_format.to_string(),
            ))
    }

    /// Gets the transform and package the transform is in, for a transform from a specific element
    /// to a specific output format. Returns None if no such transform exists
    pub fn find_transform(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Option<(Transform, Package)> {
        self.transforms
            .get(element_name)
            .and_then(|t| t.find_transform_to(output_format))
            .cloned()
    }

    pub(crate) fn generate_resolve_tasks(
        &mut self,
        arc_mutex: Arc<Mutex<Self>>,
        config: &Config,
    ) -> Result<Vec<ResolveTask>, Vec<CoreError>> {
        let missing_pkgs: Vec<PackageID> = config
            .imports
            .iter()
            .map(|i| i.into())
            .chain(config.hides.iter().map(|h| h.into()))
            .filter(|name| {
                !self.standard_packages.contains_key(name)
                    && !self.external_packages.contains_key(name)
            })
            .collect();
        let missing_std: Vec<_> = missing_pkgs
            .iter()
            .filter_map(|id| {
                (id.source == PackageSource::Standard)
                    .then(|| CoreError::NoSuchStdPackage(id.name.clone()))
            })
            .collect();
        if !missing_std.is_empty() {
            return Err(missing_std);
        }
        let missing_externals: Vec<_> = missing_pkgs
            .into_iter()
            .inspect(|id| {
                self.awaited_packages.insert(id.clone());
            })
            .map(|id| ResolveTask {
                package_store: Arc::clone(&arc_mutex),
                package_id: id,
                resolved: false,
            })
            .collect();
        Ok(missing_externals)
    }

    // This function makes sure the transforms that should be exposed according to the given
    // ModuleImport is exposed, and that no other transforms are exposed.
    pub(crate) fn expose_transforms(
        &mut self,
        mut config: ModuleImport,
    ) -> Result<(), Vec<CoreError>> {
        self.transforms.clear();

        let mut errors = vec![];

        // First, expose all native packages
        for (name, pkg) in &self.native_packages {
            for transform in &pkg.info.transforms {
                if self.transforms.contains_key(&transform.from) {
                    errors.push(CoreError::OccupiedNativeTransform(
                        transform.from.clone(),
                        name.clone(),
                    ));
                } else {
                    self.transforms.insert(
                        transform.from.to_string(),
                        TransformVariant::Native((transform.clone(), pkg.clone())),
                    );
                }
            }
        }

        // Then, loop through all standard packages and expose the ones needed
        for (name, pkg) in &self.standard_packages {
            let import_option = config.0.remove(name);
            // This match encodes the behaviour for import and hide statements for standard
            // packages, such as default values. include_entries is true if the entries in the vec
            // are the only entries to be included and false if they are the only entries to be
            // excluded. We have None => (false, vec![]) which means that if no import option is
            // chosen, the entries of the list is excluded (which is none, so all entries are
            // imported)
            let (include_entries, include_list) = match import_option {
                Some(ModuleImportConfig::HideAll) => continue,
                Some(ModuleImportConfig::ImportAll) => (false, vec![]),
                Some(ModuleImportConfig::Exclude(vec)) => (false, vec),
                Some(ModuleImportConfig::Include(vec)) => (true, vec),
                None => (false, vec![]),
            };
            if let Err(e) = Self::insert_transforms(
                &mut self.transforms,
                pkg,
                include_entries,
                include_list.as_slice(),
            ) {
                errors.push(e);
            }
        }

        for (name, pkg) in &self.external_packages {
            let import_option = config.0.remove(name);
            // This match encodes the behaviour for import and hide statements for external
            // packages, such as default values. include_entries is true if the entries in the vec
            // are the only entries to be included and false if they are the only entries to be
            // excluded. We have None => (true, vec![]) which means that if no import option is
            // chosen, only the entries of the list is included (which is none)
            let (include_entries, include_list) = match import_option {
                Some(ModuleImportConfig::HideAll) => continue,
                Some(ModuleImportConfig::ImportAll) => (false, vec![]),
                Some(ModuleImportConfig::Exclude(vec)) => (false, vec),
                Some(ModuleImportConfig::Include(vec)) => (true, vec),
                None => (true, vec![]),
            };
            if let Err(e) = Self::insert_transforms(
                &mut self.transforms,
                pkg,
                include_entries,
                include_list.as_slice(),
            ) {
                errors.push(e);
            }
        }

        mem::take(&mut config.0)
            .into_keys()
            .map(|id| CoreError::UnusedConfig(id.name))
            .for_each(|e| errors.push(e));

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // This function was introduced to avoid repeated code. It takes a map and a package, and adds
    // all transforms in that package which either exists in the list (include_entries=true) or
    // doesn't exist in the list (include_entries=false) into the map
    fn insert_transforms(
        map: &mut HashMap<String, TransformVariant>,
        pkg: &Package,
        include_entries: bool,
        include_list: &[String],
    ) -> Result<(), CoreError> {
        for transform @ Transform { from, to, .. } in &pkg.info.transforms {
            if include_entries == include_list.contains(from) {
                for output_format in to {
                    match output_format {
                        OutputFormat::Any => {
                            // this ensures Any does not overlap with any transforms with a
                            // specific output format
                            if map.get(from).is_some() {
                                return Err(CoreError::OccupiedTransform(
                                    from.clone(),
                                    output_format.to_string(),
                                    pkg.info.name.clone(),
                                ));
                            } else {
                                map.insert(
                                    from.clone(),
                                    TransformVariant::ExternalAny((transform.clone(), pkg.clone())),
                                );
                            }
                        }
                        OutputFormat::Name(_) => {
                            if map
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
                            let mut target = map
                                .remove(from)
                                .unwrap_or_else(|| TransformVariant::External(HashMap::new()));
                            target.insert_into_external(
                                output_format.clone(),
                                (transform.clone(), pkg.clone()),
                            );
                            // Add the modified entry back to the map
                            map.insert(from.clone(), target);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn is_missing_packages(&self) -> bool {
        !self.awaited_packages.is_empty()
    }

    pub(crate) fn resolve_task(&mut self, request: PackageID, response: Vec<u8>) {
        if self.awaited_packages.remove(&request) {
            self.new_packages.insert(request, response);
        }
    }

    pub(crate) fn reject_task<E>(&mut self, request: PackageID, response: E)
    where
        E: Error + Send + 'static,
    {
        if self.awaited_packages.remove(&request) {
            self.package_task_failures
                .push(CoreError::Resolve(request.name.clone(), Box::new(response)));
        }
    }
}

/// The suffix each local file must have (a dot and the file extension)
static LOCAL_FILE_EXTENSION: &'static str = ".wasm";

impl From<&str> for PackageID {
    fn from(s: &str) -> Self {
        #[inline]
        fn prefix<'a, T>(s: &'a str, prefix: &'static str, t: T) -> Option<(&'a str, T)> {
            s.starts_with(prefix)
                .then(|| (s.split_at(prefix.len()).1, t))
        }

        // Asserts that the name has the correct file extension. If it does, the &str is copied
        // and if it doesn't, the file extension is appended. Note that the copying here is OK since
        // we would need to copy the value once anyways, and we don't do another copy after this
        // point
        #[inline]
        fn assert_extension(name: &str) -> String {
            if name.ends_with(LOCAL_FILE_EXTENSION) {
                name.to_string()
            } else {
                format!("{name}{LOCAL_FILE_EXTENSION}")
            }
        }

        let (name, target) = None
            .or(prefix(s, "catalog:", PackageSource::Catalog))
            .or(prefix(s, "std:", PackageSource::Standard))
            .or_else(|| {
                (s.starts_with("http://") | s.starts_with("https://"))
                    .then_some((s, PackageSource::Url))
            })
            .map(|(a, b)| (a.to_string(), b))
            .unwrap_or((assert_extension(s), PackageSource::Local));

        PackageID {
            name,
            source: target,
        }
    }
}

impl From<String> for PackageID {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl FromStr for PackageID {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<&Import> for PackageID {
    fn from(value: &Import) -> Self {
        PackageID::from_str(&value.name).unwrap()
    }
}

impl From<&Hide> for PackageID {
    fn from(value: &Hide) -> Self {
        PackageID::from_str(&value.name).unwrap()
    }
}

impl From<Import> for PackageID {
    fn from(value: Import) -> Self {
        PackageID::from_str(&value.name).unwrap()
    }
}

impl From<Hide> for PackageID {
    fn from(value: Hide) -> Self {
        PackageID::from_str(&value.name).unwrap()
    }
}

pub trait Resolve {
    /// The implementor should resolve the given ResolveTasks, and may do so sync or async
    fn resolve_all(&self, tasks: Vec<ResolveTask>);
}

#[derive(Debug)]
pub struct ResolveTask {
    package_store: Arc<Mutex<PackageStore>>,
    pub package_id: PackageID,
    resolved: bool,
}

impl ResolveTask {
    pub fn complete<E>(self, result: Result<Vec<u8>, E>)
    where
        E: Error + Send + 'static,
    {
        match result {
            Ok(result) => self.resolve(result),
            Err(error) => self.reject(error),
        }
    }

    pub fn resolve(mut self, result: Vec<u8>) {
        self.resolved = true;
        let mut store_guard = self.package_store.lock().unwrap();
        let package = mem::take(&mut self.package_id);
        store_guard.resolve_task(package, result);
    }

    pub fn reject<E>(mut self, error: E)
    where
        E: Error + Send + 'static,
    {
        self.resolved = true;
        let mut store_guard = self.package_store.lock().unwrap();
        let package = mem::take(&mut self.package_id);
        store_guard.reject_task(package, error);
    }
}

impl Drop for ResolveTask {
    fn drop(&mut self) {
        if !self.resolved {
            let mut store_guard = self.package_store.lock().unwrap();
            let package = mem::take(&mut self.package_id);
            store_guard.reject_task(package, CoreError::DroppedRequest);
        }
    }
}

pub struct DenyAllResolver;
impl Resolve for DenyAllResolver {
    fn resolve_all(&self, _paths: Vec<ResolveTask>) {
        // Dropping the tasks should reject them
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct PackageID {
    pub name: String,
    pub source: PackageSource,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub enum PackageSource {
    Local,
    Catalog,
    Url,
    #[default]
    Standard,
}
