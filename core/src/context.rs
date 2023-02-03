use std::collections::HashMap;

use parser::Element;
use wasmer::{Cranelift, Store};

use crate::{CoreError, LoadedModule, ModuleInfo, NodeName, Transform};

pub struct Context {
    transforms: HashMap<Transform, LoadedModule>,
    all_transforms_for_node: HashMap<NodeName, Vec<(Transform, LoadedModule)>>,
    modules_by_name: HashMap<String, LoadedModule>,
    store: Store,
}

impl Context {
    pub fn new() -> Self {
        Context {
            transforms: HashMap::new(),
            all_transforms_for_node: HashMap::new(),
            modules_by_name: HashMap::new(),
            store: Store::new(Cranelift::default()),
        }
    }

    pub fn load_default_modules(&mut self) {
        self.load_module(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/bold/wasm32-wasi/release/bold.wasm"
        )))
        .expect("Failed to load bold module");

        self.load_module(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/test-module/wasm32-wasi/release/test-module.wasm"
        )))
        .expect("Failed to load test-module module");
    }

    pub fn load_module(&mut self, wasm_source: &[u8]) -> Result<(), CoreError> {
        let module = LoadedModule::new(wasm_source, &mut self.store)?;

        self.modules_by_name
            .insert(module.info.as_ref().name.to_string(), module.clone());

        // Go through all transforms that the module supports and add them
        // to the Context.
        for transform in &module.info.transforms {
            let Transform {
                from,
                to,
                arguments: _,
            } = transform;

            // Ensure that there are no other modules responsible for this transform.
            if self.transforms.contains_key(transform) {
                return Err(CoreError::OccupiedTransform(
                    from.clone(),
                    to.clone(),
                    module.info.name.clone(),
                ));
            }

            // Insert the transform into the context
            self.transforms.insert(transform.clone(), module.clone());

            // We also want to update another hashtable to get quick lookups when
            // trying to transform a given node
            if let Some(transforms) = self.all_transforms_for_node.get_mut(from) {
                transforms.push((transform.clone(), module.clone()));
            } else {
                self.all_transforms_for_node
                    .insert(from.clone(), vec![(transform.clone(), module.clone())]);
            }
        }

        Ok(())
    }

    /// Transform a node into another node by using the available transforms
    pub fn transform(&mut self, _elem: Element) -> Result<Element, CoreError> {
        // FIXME: implement this
        todo!()
    }

    /// Borrow information about a module with a given name
    pub fn get_module_info(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules_by_name
            .get(name)
            .map(|module| module.info.as_ref())
    }
}

impl Default for Context {
    /// A Context with all default modules lodaded
    fn default() -> Self {
        let mut ctx = Self::new();
        ctx.load_default_modules();
        ctx
    }
}
