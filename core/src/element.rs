use std::collections::HashMap;

use parser::{Ast, MaybeArgs, ModuleArguments};

use crate::CoreError;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Parent {
        name: String,
        args: HashMap<String, String>,
        children: Vec<Element>,
    },
    Module {
        name: String,
        args: ModuleArguments,
        body: String,
        inline: bool,
    },
    Compound(Vec<Self>),
}

impl TryFrom<Ast> for Element {
    type Error = CoreError;

    fn try_from(value: Ast) -> Result<Self, Self::Error> {
        match value {
            Ast::Text(s) => Ok(Element::Module {
                name: "__text".to_string(),
                args: ModuleArguments {
                    positioned: None,
                    named: None,
                },
                body: s,
                inline: true,
            }),
            Ast::Document(doc) => Ok(Element::Parent {
                name: "__document".to_string(),
                args: HashMap::new(),
                children: doc
                    .elements
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Element>, CoreError>>()?,
            }),
            Ast::Paragraph(paragraph) => Ok(Element::Parent {
                name: "__paragraph".to_string(),
                args: HashMap::new(),
                children: paragraph
                    .elements
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Element>, CoreError>>()?,
            }),
            Ast::Tag(tag) => Ok(Element::Parent {
                name: format!("__{}", tag.tag_name.to_lowercase()),
                args: HashMap::new(),
                children: tag
                    .elements
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Element>, CoreError>>()?,
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
                children: heading
                    .elements
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<Element>, CoreError>>()?,
            }),
        }
    }
}
