use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Default)]
pub struct VariableStore(HashMap<(String, VarType), Value>);

impl VariableStore {
    pub fn get(&self, name: &str, ty: &VarType) -> Option<&Value> {
        self.0.get(&(name, ty) as &dyn AsVariable)
    }

    /// Clears the variable store
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Ensure that a valid name is used.
    /// Only ASCII alphanumerics characters and "_" is allowed. The name may also not start with a number.
    fn valid_name(&self, name: &str) -> Result<(), CoreError> {
        // Ensure that the first character is not a number and that the name
        // is at least 1 character long
        if name.chars().next().map(char::is_numeric).unwrap_or(true) {
            return Err(CoreError::ForbiddenVariableName(name.to_string()));
        }

        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            Ok(())
        } else {
            Err(CoreError::ForbiddenVariableName(name.to_string()))
        }
    }

    /// declare a constant
    pub fn constant_declare(&mut self, name: &str, value: &str) -> Result<(), CoreError> {
        self.valid_name(name)?;
        let value = Value::Constant(value.to_string());
        let prev_value = self.0.insert((name.to_string(), VarType::Constant), value);

        if prev_value.is_some() {
            Err(CoreError::ConstantRedeclaration(name.to_string()))
        } else {
            Ok(())
        }
    }

    /// Push a string to a list (if the list does not exist, a new one is created)
    pub fn list_push(&mut self, name: &str, value: &str) -> Result<(), CoreError> {
        self.valid_name(name)?;

        self.0
            .entry((name.to_string(), VarType::List))
            .and_modify(|list| {
                let Value::List(list) = list else {
                    unreachable!("Should always contain a list value");
                };
                list.push(value.to_string())
            })
            .or_insert_with(|| Value::List(vec![value.to_string()]));

        Ok(())
    }

    /// Add a string to a set (if the set does not exist, a new one is created)
    pub fn set_add(&mut self, name: &str, value: &str) -> Result<(), CoreError> {
        self.valid_name(name)?;

        self.0
            .entry((name.to_string(), VarType::Set))
            .and_modify(|set| {
                let Value::Set(set) = set else {
                        unreachable!("Should always contain a set value");
                    };
                set.insert(value.to_string());
            })
            .or_insert_with(|| {
                let mut set = HashSet::new();
                set.insert(value.to_string());
                Value::Set(set)
            });

        Ok(())
    }
}

/// Variables are identified by a name and a type. Meaning that you can
/// have two different variables with the same name if they have different types
pub type Variable = (String, VarType);

/// Note: Variables are tuples of (String, VarType). The reason for implementing it as a
/// trait instead of a concrete type is to do lookups in hashmaps using a variable as a key without
/// having to clone. See: https://stackoverflow.com/questions/45786717/how-to-implement-hashmap-with-two-keys/45795699#45795699
/// We lose a bit of performance due to dynamic dispatch but I think it should be rather negligible, especially since other options
/// (nested or multiple hashmaps) has their problems performance
trait AsVariable {
    /// Get the name of the variable
    fn name(&self) -> &str;
    /// Get the type of the variable
    fn ty(&self) -> &VarType;
}

impl<'a> Borrow<dyn AsVariable + 'a> for (String, VarType) {
    fn borrow(&self) -> &(dyn AsVariable + 'a) {
        self
    }
}

impl Hash for (dyn AsVariable + '_) {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
        self.ty().hash(state);
    }
}

impl PartialEq for (dyn AsVariable + '_) {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.ty() == other.ty()
    }
}

impl Eq for (dyn AsVariable + '_) {}

impl AsVariable for (String, VarType) {
    fn name(&self) -> &str {
        &self.0
    }
    fn ty(&self) -> &VarType {
        &self.1
    }
}

impl AsVariable for (&str, &VarType) {
    fn name(&self) -> &str {
        self.0
    }
    fn ty(&self) -> &VarType {
        self.1
    }
}

impl AsVariable for (&String, &VarType) {
    fn name(&self) -> &str {
        self.0
    }
    fn ty(&self) -> &VarType {
        self.1
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Set(HashSet<String>),
    List(Vec<String>),
    Constant(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Lists and sets are encoded as JSON values in order to escape ','
            Value::Set(items) => {
                let json_list: serde_json::Value = items.into_iter().cloned().collect();
                write!(f, "{}", json_list)
            }
            Value::List(items) => {
                let json_list: serde_json::Value = items.clone().into();
                write!(f, "{}", json_list)
            }
            Value::Constant(value) => write!(f, "{value}"),
        }
    }
}

/// The type of a variable
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum VarType {
    /// Set<String>, a collection of unordered unique strings, like for imports and such
    Set,
    /// List<String>, a list of ordered strings ordered top-to-bottom in order of writes in the
    /// document, for headings and such
    List,
    /// Constant is a variable type that may only be written once
    Constant,
}

impl fmt::Display for VarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            VarType::Set => "set",
            VarType::List => "list",
            VarType::Constant => "const",
        };
        write!(f, "{}", s)
    }
}

/// This is the type of accesses that a transform may request to a certain variable
/// The enum names here are used as the "type" field in the manifest
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "access", rename_all = "lowercase")]
pub enum VarAccess {
    Set(SetAccess),
    List(ListAccess),
    #[serde(alias = "const")]
    Constant(ConstantAccess),
}

impl VarAccess {
    /// If two access levels are deemed the same (no one should strictly be done before the other),
    /// this function is called to see if the granularid (occurrence in the source document) should
    /// determine the order of evaluation. If not, the order is arbitrarily chosen.
    pub fn order_granular(&self) -> bool {
        matches!(&self, VarAccess::List(_))
    }

    /// Gets the `VarType` corresponding to this `VarAccess`, that is, `VarType::Set` if this
    /// is a `VarAccess::Set` access etc.
    pub fn get_type(&self) -> VarType {
        match self {
            VarAccess::Set(_) => VarType::Set,
            VarAccess::List(_) => VarType::List,
            VarAccess::Constant(_) => VarType::Constant,
        }
    }

    /// Returns true if it is a read access
    pub fn is_read(&self) -> bool {
        match self {
            VarAccess::Set(SetAccess::Read) => true,
            VarAccess::List(ListAccess::Read) => true,
            VarAccess::Constant(ConstantAccess::Read) => true,
            _ => false,
        }
    }
}

impl PartialOrd for VarAccess {
    /// Two different accesses `a` and `b` may or may not need to be ordered in a certain way.
    /// The result when comparing the two determines how they are ordered:
    /// * If `a` < `b`, `a` *will* occur before `b`
    /// * If `a` > `b`, `a` *will* occur after `b`
    /// * If `a` = `b`, `a` may occur before or after `b` (see ::order_granular)
    /// * If there is no ordering (this function returns None), any may occur before the other
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use VarAccess::*;
        match &self {
            Set(a) => {
                if let Set(b) = other {
                    Some(a.cmp(b))
                } else {
                    None
                }
            }
            List(a) => {
                if let List(b) = other {
                    Some(a.cmp(b))
                } else {
                    None
                }
            }
            Constant(a) => {
                if let Constant(b) = other {
                    Some(a.cmp(b))
                } else {
                    None
                }
            }
        }
    }
}

// The ordering of these enum variants is very important. For determining what variable access types
// must occur before others, VarAccess::partial_cmp is used which in turn uses the ordering of this
// accesses. #derive Ord makes an ordering based on the order they are defined. Lesser accesses
// always occur before greater accesses, so defining SetAccess::Add before SetAccess::Read ensures
// that, for any given set, all Add operations occur before any Read operation does.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SetAccess {
    Add,
    Read,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListAccess {
    Push,
    Read,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConstantAccess {
    Declare,
    Read,
}
