use std::collections::HashMap;

use granular_id::GranularId;

use parser::{Ast, MaybeArgs, ModuleArguments};

use crate::CoreError;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Parent {
        name: String,
        args: HashMap<String, String>,
        children: Vec<Element>,
        id: GranularId<u32>,
    },
    Module {
        name: String,
        args: ModuleArguments,
        body: String,
        inline: bool,
        id: GranularId<u32>,
    },
    Compound(Vec<Self>),
}

impl Element {
    pub(crate) fn id(&self) -> Option<&GranularId<u32>> {
        if let Element::Parent { id, .. } | Element::Module { id, .. } = self {
            Some(id)
        } else {
            None
        }
    }

    pub(crate) fn try_from_ast(value: Ast, id: GranularId<u32>) -> Result<Self, CoreError> {
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
