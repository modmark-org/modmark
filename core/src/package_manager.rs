use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::error::Error;
use std::hash::Hash;
use std::mem;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use thiserror::Error;
#[cfg(feature = "native")]
use wasmer::Engine;

use parser::config::{Config, Hide, Import};

use crate::context::{ModuleImport, ModuleImportConfig, TransformVariant};
use crate::package::PackageImplementation;
use crate::{std_packages,  CoreError, OutputFormat, Package, Transform};

#[cfg(feature = "native")]
macro_rules! package_new {
    ($source:expr, $engine:tt) => {
        Package::new($source, $engine)
    };
}

#[cfg(not(feature = "native"))]
macro_rules! package_new {
    ($source:expr, $engine:expr) => {
        Package::new($source)
    };
}

#[derive(Debug)]
pub struct PackageManager {
    pub(crate) native_packages: HashMap<String, Package>,
    pub(crate) standard_packages: HashMap<PackageID, Package>,
    pub(crate) external_packages: HashMap<PackageID, Package>,
    pub(crate) awaited_packages: HashSet<PackageID>,
    pub(crate) new_packages: HashMap<PackageID, Vec<u8>>,
    pub(crate) failed_packages: Vec<CoreError>,
    pub(crate) transforms: HashMap<String, TransformVariant>,
}

impl Default for PackageManager {
    fn default() -> Self {
        Self {
            native_packages: Default::default(),
            standard_packages: Default::default(),
            external_packages: Default::default(),
            awaited_packages: Default::default(),
            new_packages: Default::default(),
            failed_packages: Default::default(),
            transforms: Default::default(),
        }
    }
}

impl PackageManager {
    /// Clears the loaded external packages and transforms, possibly saving memory, forcing the
    /// Resolve to fetch any external packages again, essentially clearing the internal package
    /// cache of the Context
    pub fn clear_packages(&mut self) {
        self.transforms = Default::default();
        self.external_packages = Default::default();
    }

    //noinspection RsUnreachableCode
    // Might want to change this to Vec<CoreError>?
    pub fn finalize(
        &mut self,
        #[cfg(feature = "native")] engine: &Engine,
    ) -> Result<(), CoreError> {
        // First, get all successful fetches and add them to external_packages, propagating any
        // error when creating the package itself
        let result: Vec<(PackageID, Package)> = self
            .new_packages
            .drain()
            .map(|(k, v)| package_new!(v.as_slice(), engine).map(|p| (k, p)))
            .collect::<Result<Vec<_>, _>>()?;

        for (package_name, package) in result {
            self.external_packages.insert(package_name, package);
        }

        // Then, clear all failed packages and if any failed, return that error
        if let Some(error) = self.failed_packages.drain(..).next() {
            return Err(error);
        }

        Ok(())
    }

    /// This function loads the default packages to the Context. First, it loads all native
    /// packages, retrieved from `std_packages::native_package_list()`, and then it loads all
    /// standard packages by passing this Context to `std_packages::load_standard_packages()`. This
    /// will be run when constructing the Context, and may only be run once.
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

        let name = pkg.info.name.as_str();
        let id = PackageID {
            name: name.to_string(),
            target: PackageSource::Standard,
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

    /// This is a helper function to load a package directly from its wasm source. It will be
    /// compiled using `Package::new` to become a `Package` and then loaded using `load_package`
    #[allow(dead_code)]
    pub(crate) fn load_standard_package(
        &mut self,
        wasm_source: &[u8],
        #[cfg(feature = "native")] engine: &Engine,
    ) -> Result<(), CoreError> {
        let pkg = package_new!(wasm_source, engine)?;

        let name = pkg.info.name.as_str();
        let id = PackageID {
            name: name.to_string(),
            target: PackageSource::Standard,
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

    pub fn load_native_package(&mut self, pkg: Package) -> Result<(), CoreError> {
        debug_assert_eq!(pkg.implementation, PackageImplementation::Native);
        let name = &pkg.info.as_ref().name;
        let entry = self.native_packages.entry(name.to_string());

        match entry {
            Entry::Occupied(_) => return Err(CoreError::OccupiedName(name.to_string())),
            Entry::Vacant(entry) => entry.insert(pkg),
        };
        Ok(())
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

    /// Gets the transform and package the transform is in, for a transform from a specific element
    /// to a specific output format. Returns None if no such transform exists
    pub fn get_transform_to(
        &self,
        element_name: &str,
        output_format: &OutputFormat,
    ) -> Option<(Transform, Package)> {
        self.transforms
            .get(element_name)
            .and_then(|t| t.find_transform_to(output_format))
            .cloned()
    }

    pub(crate) fn get_missing_packages(
        &mut self,
        arc_mutex: Arc<Mutex<Self>>,
        config: &Config,
    ) -> Vec<ResolveTask> {
        config
            .imports
            .iter()
            .map(|i| i.into())
            .chain(config.hides.iter().map(|h| h.into()))
            .filter(|name| {
                !self.standard_packages.contains_key(name)
                    && !self.external_packages.contains_key(name)
            })
            .inspect(|id| {
                self.awaited_packages.insert(id.clone());
            })
            .map(|id| ResolveTask {
                manager: Arc::clone(&arc_mutex),
                package: id.clone(),
                resolved: false,
            })
            .collect()
    }

    // This function makes sure the transforms that should be exposed according to the given
    // ModuleImport is exposed, and that no other transforms are exposed.
    pub(crate) fn expose_transforms(&mut self, mut config: ModuleImport) -> Result<(), CoreError> {
        self.transforms.clear();

        // First, expose all native packages
        for (name, pkg) in &self.native_packages {
            for transform in &pkg.info.transforms {
                let Transform {
                    from,
                    to: _,
                    description: _,
                    arguments: _,
                } = transform;
                if self.transforms.contains_key(from) {
                    return Err(CoreError::OccupiedNativeTransform(
                        from.clone(),
                        name.clone(),
                    ));
                }
                self.transforms.insert(
                    from.to_string(),
                    TransformVariant::Native((transform.clone(), pkg.clone())),
                );
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
            Self::insert_transforms(
                &mut self.transforms,
                pkg,
                include_entries,
                include_list.as_slice(),
            )?;
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
            Self::insert_transforms(
                &mut self.transforms,
                pkg,
                include_entries,
                include_list.as_slice(),
            )?;
        }

        if config.0.is_empty() {
            Ok(())
        } else {
            Err(CoreError::UnusedConfigs(
                config.0.drain().map(|(k, _)| k.name).collect(),
            ))
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
        for transform @ Transform {
            from,
            to,
            description: _,
            arguments: _,
        } in &pkg.info.transforms
        {
            if include_entries == include_list.contains(from) {
                for output_format in to {
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

        Ok(())
    }

    pub(crate) fn is_missing_packages(&self) -> bool {
        !self.awaited_packages.is_empty()
    }

    pub(crate) fn resolve_request(&mut self, request: PackageID, response: Vec<u8>) {
        if self.awaited_packages.remove(&request) {
            self.new_packages.insert(request, response);
        }
    }

    pub(crate) fn reject_request<E>(&mut self, request: PackageID, response: E)
    where
        E: Error + Send + 'static,
    {
        if self.awaited_packages.remove(&request) {
            self.failed_packages
                .push(CoreError::Resolve(request.name.clone(), Box::new(response)));
        }
    }
}

impl From<&str> for PackageID {
    fn from(s: &str) -> Self {
        #[inline]
        fn prefix<'a, T>(s: &'a str, prefix: &'static str, t: T) -> Option<(&'a str, T)> {
            s.starts_with(prefix)
                .then(|| (s.split_at(prefix.len()).1, t))
        }

        let (name, target) = None
            .or(prefix(s, "pkg:", PackageSource::Registry))
            .or(prefix(s, "pkgs:", PackageSource::Registry))
            .or(prefix(s, "prelude:", PackageSource::Standard))
            .or(prefix(s, "std:", PackageSource::Standard))
            .or_else(|| {
                (s.starts_with("http://") | s.starts_with("https://"))
                    .then_some((s, PackageSource::Url))
            })
            .unwrap_or((s, PackageSource::Local));
        let name = name.to_string();
        PackageID { name, target }
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
    fn resolve_all(&self, paths: Vec<ResolveTask>);
}

pub struct ResolveWrapper {
    task: ResolveTask,
}

#[derive(Debug)]
pub struct ResolveTask {
    manager: Arc<Mutex<PackageManager>>,
    pub package: PackageID,
    resolved: bool,
}

impl ResolveTask {
    pub fn complete<E>(self, result: Result<Vec<u8>, E>)
    where
        E: Error + Send + 'static,
    {
        println!("Found it, error: {}", result.is_err());
        match result {
            Ok(result) => self.resolve(result),
            Err(error) => self.reject(error),
        }
    }

    pub fn resolve(mut self, result: Vec<u8>) {
        self.resolved = true;
        let mut manager = self.manager.lock().unwrap();
        let package = mem::take(&mut self.package);
        manager.resolve_request(package, result);
    }

    pub fn reject<E>(mut self, error: E)
    where
        E: Error + Send + 'static,
    {
        self.resolved = true;
        let mut manager = self.manager.lock().unwrap();
        let package = mem::take(&mut self.package);
        manager.reject_request(package, error);
    }
}

impl Drop for ResolveTask {
    fn drop(&mut self) {
        if !self.resolved {
            let mut manager = self.manager.lock().unwrap();
            let package = mem::take(&mut self.package);
            manager.reject_request(package.clone(), CoreError::DroppedRequest);
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct PackageID {
    pub name: String,
    pub target: PackageSource,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub enum PackageSource {
    Local,
    Registry,
    Url,
    #[default]
    Standard,
}

#[derive(Error, Debug)]
#[error("Deny all resolve attempts")]
pub struct DenyAllResolverError;

pub struct DenyAllResolver;

impl Resolve for DenyAllResolver {
    fn resolve_all(&self, paths: Vec<ResolveTask>) {
        paths
            .into_iter()
            .for_each(|p| p.reject(DenyAllResolverError))
    }
}