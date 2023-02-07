use std::collections::HashMap;

use parser::Element;
#[cfg(feature = "native")]
use wasmer::Cranelift;
use wasmer::Store;

use crate::{CoreError, NodeName, Package, PackageInfo, Transform};

#[derive(Debug)]
pub struct Context {
    packages_by_transform: HashMap<Transform, Package>,
    packages_by_name: HashMap<String, Package>,
    all_transforms_for_node: HashMap<NodeName, Vec<(Transform, Package)>>,
    store: Store,
}

impl Context {
    pub fn new() -> Self {
        Context {
            packages_by_transform: HashMap::new(),
            all_transforms_for_node: HashMap::new(),
            packages_by_name: HashMap::new(),
            store: get_new_store(),
        }
    }

    pub fn load_default_packages(&mut self) {
        self.load_package(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/test-module/wasm32-wasi/release/test-module.wasm"
        )))
        .expect("Failed to load test-module module");
    }

    pub fn load_package(&mut self, wasm_source: &[u8]) -> Result<(), CoreError> {
        let pkg = Package::new(wasm_source, &mut self.store)?;

        self.packages_by_name
            .insert(pkg.info.as_ref().name.to_string(), pkg.clone());

        // Go through all transforms that the package supports and add them
        // to the Context.
        for transform in &pkg.info.transforms {
            let Transform {
                from,
                to,
                arguments: _,
            } = transform;

            // Ensure that there are no other packages responsible for this transform.
            if self.packages_by_transform.contains_key(transform) {
                return Err(CoreError::OccupiedTransform(
                    from.clone(),
                    to.clone(),
                    pkg.info.name.clone(),
                ));
            }

            // Insert the transform into the context
            self.packages_by_transform
                .insert(transform.clone(), pkg.clone());

            // We also want to update another hashtable to get quick lookups when
            // trying to transform a given node
            if let Some(transforms) = self.all_transforms_for_node.get_mut(from) {
                transforms.push((transform.clone(), pkg.clone()));
            } else {
                self.all_transforms_for_node
                    .insert(from.clone(), vec![(transform.clone(), pkg.clone())]);
            }
        }

        Ok(())
    }

    /// Transform a node into another node by using the available transforms
    pub fn transform(&mut self, _elem: Element) -> Result<Element, CoreError> {
        // FIXME: implement this
        todo!()
    }

    /// Borrow information about a package with a given name
    pub fn get_package_info(&self, name: &str) -> Option<&PackageInfo> {
        self.packages_by_name.get(name).map(|pkg| pkg.info.as_ref())
    }
}

impl Default for Context {
    /// A Context with all default packages lodaded
    fn default() -> Self {
        let mut ctx = Self::new();
        ctx.load_default_packages();
        ctx
    }
}

/// Get a new using different compilers depending
/// if we are using the "web" or "native" feature
fn get_new_store() -> Store {
    #[cfg(feature = "web")]
    return Store::new();

    #[cfg(feature = "native")]
    return Store::new(Cranelift::new());
}
