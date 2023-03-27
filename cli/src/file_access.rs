use crate::Args;
use modmark_core::AccessPolicy;
use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use PermissionType::*;

enum PermissionType {
    Read,
    Write,
    Create,
}

#[derive(Default)]
struct Permissions {
    read: Option<bool>,
    write: Option<bool>,
    create: Option<bool>,
}

impl Permissions {
    fn denied() -> Self {
        Permissions {
            read: Some(false),
            write: Some(false),
            create: Some(false),
        }
    }

    fn allowed() -> Self {
        Permissions {
            read: Some(true),
            write: Some(true),
            create: Some(true),
        }
    }
}

#[derive(Default)]
struct ModulePermissions {
    global_read: Option<bool>,
    global_write: Option<bool>,
    global_create: Option<bool>,
    permissions: HashMap<String, Permissions>,
}

pub struct CliAccessManager {
    root: String,
    deny_read: bool,
    deny_write: bool,
    deny_create: bool,
    no_prompts: bool,
    modules: HashMap<String, ModulePermissions>,
}

impl AccessPolicy for CliAccessManager {
    fn root(&self) -> String {
        self.root.clone()
    }

    fn allowed_to_read(&mut self, path: &Path, module_name: &String) -> bool {
        if self.deny_read {
            return false;
        } else if self.no_prompts {
            return true;
        }

        let module = self.modules.entry(module_name.clone()).or_default();
        let path_str = path.to_str().expect("Could not convert path to &str");

        if let Some(global_read) = module.global_read {
            return global_read;
        } else if let Some(permissions) = module.permissions.get(path_str) {
            if let Some(read) = permissions.read {
                return read;
            }
        }

        let prompt = format!(
            "\nModule [{module_name}] requests read access to \"{path_str}\". Options are listed below.\n\
            Use a capital letter to apply the decision to all requests from this module.\n\
            grant read [g], grant all [a], deny read [d], deny all [p]"
        );

        let input = self.prompt_user(prompt);
        self.set_permissions(module_name.clone(), path_str.to_string(), input, Read)
    }

    fn allowed_to_write(&mut self, path: &Path, module_name: &String) -> bool {
        if self.deny_write {
            return false;
        } else if self.no_prompts {
            return true;
        }

        let module = self.modules.entry(module_name.clone()).or_default();
        let path_str = path.to_str().expect("Could not convert path to &str");

        if let Some(global_write) = module.global_write {
            return global_write;
        } else if let Some(permissions) = module.permissions.get(path_str) {
            if let Some(write) = permissions.write {
                return write;
            }
        }

        let prompt = format!(
            "Module [{module_name}] requests write access to \"{path_str}\". Options are listed below.\n\
            Use a capital letter to apply the decision to all requests from this module.\n\
            grant write [g], grant all [a], deny write [d], deny all [p]"
        );

        let input = self.prompt_user(prompt);
        self.set_permissions(module_name.clone(), path_str.to_string(), input, Write)
    }

    fn allowed_to_create(&mut self, path: &Path, module_name: &String) -> bool {
        if self.deny_create {
            return false;
        } else if self.no_prompts {
            return true;
        }

        let module = self.modules.entry(module_name.clone()).or_default();
        let path_str = path.to_str().expect("Could not convert path to &str");

        if let Some(global_create) = module.global_create {
            return global_create;
        } else if let Some(permissions) = module.permissions.get(path_str) {
            if let Some(create) = permissions.create {
                return create;
            }
        }

        let prompt = format!(
            "Module [{module_name}] requests create access to \"{path_str}\". Options are listed below.\n\
            Use a capital letter to apply the decision to all requests from this module.\n\
            grant create [g], grant all [a], deny create [d], deny all [p]"
        );

        let input = self.prompt_user(prompt);
        self.set_permissions(module_name.clone(), path_str.to_string(), input, Create)
    }
}

impl CliAccessManager {
    pub(crate) fn new_with_args(args: &Args) -> Self {
        Self {
            root: args.assets.clone().unwrap_or(String::from("assets")),
            deny_read: args.deny_read,
            deny_write: args.deny_write,
            deny_create: args.deny_create,
            no_prompts: args.no_prompts,
            modules: HashMap::new(),
        }
    }

    fn prompt_user(&self, prompt: String) -> String {
        println!("{prompt}");

        let allowed_input = vec!["d", "p", "g", "a", "D", "P", "G", "A"];
        let allowed_input_str = allowed_input.join(", ");

        loop {
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).expect("");

            let input = buffer.trim();
            if allowed_input.contains(&input) {
                return input.to_string();
            }

            print!("Unexpected input. Please enter [{allowed_input_str}]: ");
            stdout().flush().expect("Could not flush output");
        }
    }

    fn set_permissions(
        &mut self,
        module_name: String,
        path: String,
        input: String,
        perm_type: PermissionType,
    ) -> bool {
        let module = self.modules.entry(module_name).or_default();
        match input.as_str() {
            "d" => {
                module
                    .permissions
                    .entry(path)
                    .and_modify(|p| match perm_type {
                        Read => p.read = Some(false),
                        Write => p.write = Some(false),
                        Create => p.create = Some(false),
                    })
                    .or_default();
                false
            }
            "g" => {
                module
                    .permissions
                    .entry(path)
                    .and_modify(|p| match perm_type {
                        Read => p.read = Some(true),
                        Write => p.write = Some(true),
                        Create => p.create = Some(true),
                    })
                    .or_default();
                true
            }
            "p" => {
                module.permissions.insert(path, Permissions::denied());
                false
            }
            "a" => {
                module.permissions.insert(path, Permissions::allowed());
                true
            }
            "D" => {
                match perm_type {
                    Read => module.global_read = Some(false),
                    Write => module.global_write = Some(false),
                    Create => module.global_create = Some(false),
                }
                false
            }
            "G" => {
                match perm_type {
                    Read => module.global_read = Some(true),
                    Write => module.global_write = Some(true),
                    Create => module.global_create = Some(true),
                }
                true
            }
            "P" => {
                module.global_read = Some(false);
                module.global_write = Some(false);
                module.global_create = Some(false);
                false
            }
            "A" => {
                module.global_read = Some(true);
                module.global_write = Some(true);
                module.global_create = Some(true);
                true
            }
            _ => panic!("Unexpected input"),
        }
    }
}
