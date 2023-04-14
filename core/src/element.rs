use std::collections::HashMap;

use parser::{Ast, MaybeArgs, ModuleArguments};

use crate::CoreError;

pub type GranularId = granular_id::GranularId<usize>;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Parent {
        name: String,
        args: HashMap<String, String>,
        children: Vec<Element>,
        id: GranularId,
    },
    Module {
        name: String,
        args: ModuleArguments,
        body: String,
        inline: bool,
        id: GranularId,
    },
    Compound(Vec<Self>),
    Raw(String),
}

impl Element {
    pub fn get_by_id(&self, id: GranularId) -> Option<Self> {
        let components: Vec<usize> = id.into();
        components
            .into_iter()
            .fold(Some(self), |current, id| {
                current.and_then(|c| match c {
                    Element::Parent { children, .. } => children.get(id),
                    Element::Compound(children) => children.get(id),
                    _ => None,
                })
            })
            .cloned()
    }

    pub fn get_by_id_mut(&mut self, id: GranularId) -> Option<&mut Self> {
        let components: Vec<usize> = id.into();
        components.into_iter().fold(Some(self), |current, id| {
            current.and_then(|c| match c {
                Element::Parent { children, .. } => children.get_mut(id),
                Element::Compound(children) => children.get_mut(id),
                _ => None,
            })
        })
    }

    /// Checks if this element can be flattened to a string using `flatten`. If this returns `true`,
    /// `flatten` will result in `Some`.
    pub fn is_flat(&self) -> bool {
        match self {
            Element::Parent { .. } | Element::Module { .. } => false,
            Element::Compound(c) => c.iter().all(Element::is_flat),
            Element::Raw(_) => true,
        }
    }

    /// Attempt to flatten the element by merging raw elements and compounds. If any other element
    /// is in the structure, this function returns `None`
    pub fn flatten(self) -> Option<Vec<String>> {
        match self {
            // Note that the collect ensures we have an early return if any of the elements can't
            // be flattened
            Element::Compound(children) => children
                .into_iter()
                .map(Self::flatten)
                .collect::<Option<Vec<Vec<String>>>>()
                .map(|x| x.into_iter().flatten().collect()),
            Element::Raw(s) => Some(vec![s]),
            // Parent and module nodes can't be flattened and must be evaluated
            _ => None,
        }
    }

    /// Get the name of a element (if it has one).
    pub fn name(&self) -> Option<&str> {
        match self {
            Element::Parent { name, .. } | Element::Module { name, .. } => Some(name),
            Element::Raw(_) | Element::Compound(_) => None,
        }
    }
}

impl Element {
    /// Tries to create an Element from an Ast. Elements may contain ID:s, but Ast:s doesn't, which
    /// means that we do need to have a root ID for the Ast. The Ast itself may be assigned that ID
    /// and if the Ast has children, they will be assigned IDs of children to the root ID.
    pub fn try_from_ast(value: Ast, id: GranularId) -> Result<Self, CoreError> {
        macro_rules! zip_elems {
            ($elems:expr, $id:expr) => {
                $elems
                    .into_iter()
                    .zip($id.children())
                    .map(|(ast, id)| Self::try_from_ast(ast, id))
                    .collect::<Result<Vec<Element>, CoreError>>()
            };
        }

        match value {
            Ast::Text(s) => Ok(Element::Module {
                name: "__text".to_string(),
                args: ModuleArguments {
                    positioned: None,
                    named: None,
                },
                body: s,
                inline: true,
                id,
            }),
            Ast::Document(document) => Ok(Element::Parent {
                name: "__document".to_string(),
                args: HashMap::new(),
                children: zip_elems!(document.elements, id)?,
                id,
            }),
            Ast::Paragraph(paragraph) => Ok(Element::Parent {
                name: "__paragraph".to_string(),
                args: HashMap::new(),
                children: zip_elems!(paragraph.elements, id)?,
                id,
            }),
            Ast::Tag(tag) => Ok(Element::Parent {
                name: format!("__{}", tag.tag_name.to_lowercase()),
                args: HashMap::new(),
                children: zip_elems!(tag.elements, id)?,
                id,
            }),
            Ast::Module(module) => {
                if &module.name.to_ascii_lowercase() == "config" {
                    Err(CoreError::UnexpectedConfigModule)
                } else {
                    match module.args {
                        MaybeArgs::ModuleArguments(args) => Ok(Element::Module {
                            name: module.name,
                            args,
                            body: module.body,
                            inline: module.one_line,
                            id,
                        }),
                        MaybeArgs::Error(error) => Err(error.into()),
                    }
                }
            }
            Ast::Heading(heading) => Ok(Element::Parent {
                name: "__heading".to_string(),
                args: {
                    let mut map = HashMap::new();
                    map.insert("level".to_string(), heading.level.to_string());
                    map
                },
                children: zip_elems!(heading.elements, id)?,
                id,
            }),
        }
    }
}
