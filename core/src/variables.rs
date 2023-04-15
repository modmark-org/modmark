use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
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

    /// declare a constant
    pub fn declare_constant(&mut self, name: &str, value: Value) -> Result<(), CoreError> {
        let prev_value = self.0.insert((name.to_string(), VarType::Constant), value);

        if prev_value.is_some() {
            Err(CoreError::ConstantRedeclaration(name.to_string()))
        } else {
            Ok(())
        }
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
    fn name(&self) -> &str;
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

/// This is the type of accesses that a transform may request to a certain variable
/// The enum names here are used as the "type" field in the manifest
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "access", rename_all = "lowercase")]
pub enum VarAccess {
    Set(SetAccess),
    List(ListAccess),
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
