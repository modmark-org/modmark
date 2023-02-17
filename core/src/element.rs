use parser::{Ast, MaybeArgs, ModuleArguments, ParseError};

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Node {
        name: String,
        children: Vec<Element>,
    },
    ModuleInvocation {
        name: String,
        args: ModuleArguments,
        body: String,
        one_line: bool,
    },
    Compound(Vec<Self>),
}

impl TryFrom<Ast> for Element {
    type Error = ParseError;

    fn try_from(value: Ast) -> Result<Self, Self::Error> {
        match value {
            Ast::Text(s) => Ok(Element::ModuleInvocation {
                name: "__text".to_string(),
                args: ModuleArguments {
                    positioned: None,
                    named: None,
                },
                body: s,
                one_line: true,
            }),
            Ast::Document(doc) => Ok(Element::Node {
                name: "__document".to_string(),
                children: doc
                    .elements
                    .into_iter()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<Element>, ParseError>>()?,
            }),
            Ast::Paragraph(paragraph) => Ok(Element::Node {
                name: "__paragraph".to_string(),
                children: paragraph
                    .elements
                    .into_iter()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<Element>, ParseError>>()?,
            }),
            Ast::Tag(tag) => Ok(Element::Node {
                name: format!("__{}", tag.tag_name.to_lowercase()),
                children: tag
                    .elements
                    .into_iter()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<Element>, ParseError>>()?,
            }),
            Ast::Module(module) => match module.args {
                MaybeArgs::ModuleArguments(args) => Ok(Element::ModuleInvocation {
                    name: module.name,
                    args,
                    body: module.body,
                    one_line: module.one_line,
                }),
                MaybeArgs::Error(error) => Err(error),
            },
            Ast::Heading(heading) => Ok(Element::Node {
                name: format!("Heading{}", heading.level),
                children: heading
                    .elements
                    .into_iter()
                    .map(|e| e.try_into())
                    .collect::<Result<Vec<Element>, ParseError>>()?,
            }),
        }
    }
}
