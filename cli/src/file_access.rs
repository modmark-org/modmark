use crate::Args;
use modmark_core::AccessPolicy;
use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use std::path::Path;

#[derive(Default)]
struct ModulePermissions {
    permissions: HashMap<String, bool>,
}

pub struct CliAccessManager {
    root: Option<String>,
    deny_read: bool,
    deny_write: bool,
    deny_create: bool,
    allow_every_module: bool,
    modules: HashMap<String, ModulePermissions>,
}

impl AccessPolicy for CliAccessManager {
    fn root(&self) -> Option<String> {
        self.root.clone()
    }

    fn allowed_to_read(&self) -> bool {
        !self.deny_read
    }

    fn allowed_to_write(&self) -> bool {
        !self.deny_write
    }

    fn allowed_to_create(&self) -> bool {
        !self.deny_create
    }

    fn allowed_access(&mut self, path: &Path, module_name: &String) -> bool {
        if self.allow_every_module {
            return true;
        }

        let module = self.modules.entry(module_name.clone()).or_default();
        let path_str = path.to_str().expect("Could not convert path to &str");

        if let Some(access) = module.permissions.get(path_str) {
            *access
        } else {
            // If we have not asked the user before, prompt them
            let prompt =
                format!("\nModule [{module_name}] requests access to \"{path_str}\" (y/n): ");
            let result = prompt_user(&prompt);
            module.permissions.insert(path_str.to_string(), result);
            result
        }
    }
}

// TODO LATER: make async to work better with the rest of the CLI
fn prompt_user(prompt: &str) -> bool {
    println!("{prompt}");

    let allowed_input = ["y", "n", "Y", "N"];

    loop {
        let mut buffer = String::new();
        stdin().read_line(&mut buffer).expect("");

        let input = buffer.trim();
        if allowed_input.contains(&input) {
            return input == "y" || input == "Y";
        }

        print!("Unexpected input. Please enter (y/n): ");
        stdout().flush().expect("Could not flush output");
    }
}

impl CliAccessManager {
    pub(crate) fn new(args: &Args) -> Self {
        Self {
            root: args.assets.clone(),
            deny_read: args.deny_read,
            deny_write: args.deny_write,
            deny_create: args.deny_create,
            allow_every_module: args.allow_every_module,
            modules: HashMap::new(),
        }
    }
}
