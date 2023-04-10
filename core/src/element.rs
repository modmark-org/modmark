use std::collections::HashMap;

use granular_id::GranularId;

use parser::{Ast, MaybeArgs, ModuleArguments};

use crate::CoreError;

pub type GranId = GranularId<usize>;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Parent {
        name: String,
        args: HashMap<String, String>,
        children: Vec<Element>,
        id: GranId,
    },
    Module {
        name: String,
        args: ModuleArguments,
        body: String,
        inline: bool,
        id: GranId,
    },
    Compound(Vec<Self>),
}

impl Element {
    pub fn get_by_id(&self, id: GranId) -> Option<Self> {
        let components: Vec<u32> = id.into();
        components
            .into_iter()
            .fold(Some(self), |current, id| {
                current.and_then(|c| match c {
                    Element::Parent { children, .. } => children.get(id as usize),
                    Element::Compound(children) => children.get(id as usize),
                    _ => None,
                })
            })
            .cloned()
    }

    pub fn get_by_id_mut(&mut self, id: GranId) -> Option<&mut Self> {
        let components: Vec<u32> = id.into();
        components.into_iter().fold(Some(self), |current, id| {
            current.and_then(|c| match c {
                Element::Parent { children, .. } => children.get_mut(id as usize),
                Element::Compound(children) => children.get_mut(id as usize),
                _ => None,
            })
        })
    }

    /*fn with_id(mut self, new_id: GranId) -> Self {
        match self {
            Element::Parent(ref mut id, _, _) => *id = new_id,
            Element::Module(ref mut id, _) => *id = new_id,
            Element::Compound(ref mut id, _) => *id = new_id,
            Element::Raw(ref mut id, _) => *id = new_id,
        }
        self
    }*/

    /// Attempt to flatten the element by merging raw elements and compounds
    pub fn flatten(self) -> Option<Vec<String>> {
        match self {
            Element::Compound(children) => children.into_iter().map(Self::flatten).fold(
                Some(Vec::new()),
                |mut vec, mut flat| {
                    if let Some(ref mut v) = flat {
                        vec.as_mut().map(|x| x.append(v));
                    }
                    vec
                },
            ),
            // TODO: Add a raw kind
            Element::Raw(_, value) => Some(vec![value]),
            // Parent and module nodes can't be flattened and must be evaluated
            _ => None,
        }
    }
}

impl Element {
    pub(crate) fn id(&self) -> Option<&GranId> {
        if let Element::Parent { id, .. } | Element::Module { id, .. } = self {
            Some(id)
        } else {
            None
        }
    }

    pub(crate) fn try_from_ast(value: Ast, id: GranId) -> Result<Self, CoreError> {
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
