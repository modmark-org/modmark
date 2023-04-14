use std::cmp::Ordering;

// This is, for now, a placeholder file for variables. The data types it contains is (some) of the
// once it will contain when the variable store is actually implemented.

// The type of a variable
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum VarType {
    // Set<String>, a collection of unordered unique strings, like for imports and such
    Set,
    // List<String>, a list of ordered strings ordered top-to-bottom in order of writes in the
    // document, for headings and such
    List,
    // Constant is a variable type that may only be written once
    Constant,
}

// This is the type of accesses that a transform may request to a certain variable
#[derive(Copy, Clone, Debug, PartialEq)]
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

// Note we can have two different variables with the same name
// if they have different types
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Variable(pub String, pub VarType);

// The ordering of these enum variants is very important. For determining what variable access types
// must occur before others, VarAccess::partial_cmp is used which in turn uses the ordering of this
// accesses. #derive Ord makes an ordering based on the order they are defined. Lesser accesses
// always occur before greater accesses, so defining SetAccess::Add before SetAccess::Read ensures
// that, for any given set, all Add operations occur before any Read operation does.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq)]
pub enum SetAccess {
    Add,
    Read,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq)]
pub enum ListAccess {
    Push,
    Read,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq)]
pub enum ConstantAccess {
    Declare,
    Read,
}