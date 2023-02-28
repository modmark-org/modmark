extern crate core;

use std::collections::HashMap;
use std::fmt;
use std::mem;

use nom::bytes::complete::{take_till, take_while1};
use nom::character::complete::{char, line_ending, none_of, space0};
use nom::error::Error;
use nom::multi::{fold_many0, many0, many1, separated_list0};
use nom::sequence::{pair, preceded, terminated};
use nom::{combinator::*, Finish, IResult, Parser};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::punct::smart_punctuate;
use crate::tag::CompoundAST;
use crate::Ast::Text;

mod module;
mod or;
mod punct;
mod tag;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ModuleArguments {
    pub positioned: Option<Vec<String>>,
    pub named: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MaybeArgs {
    ModuleArguments(ModuleArguments),
    Error(ParseError),
}

impl Default for MaybeArgs {
    fn default() -> Self {
        MaybeArgs::ModuleArguments(ModuleArguments::default())
    }
}

/// This enum represents an Ast, an Abstract Syntax Tree. It is essentially a tree-like structure
/// representing the structure and content of a parsed document. `Text` and `Module` are leaf-nodes;
/// they do not contain any other nodes, and all other nodes are inner nodes (they may contain
/// other nodes).
#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    Text(String),
    Document(Document),
    Paragraph(Paragraph),
    Tag(Tag),
    Module(Module),
    Heading(Heading),
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum ParseError {
    #[error("Unnamed argument after named argument")]
    ArgumentOrderError,
    #[error("Nom error: '{0}'")]
    NomError(String),
}

impl<T> From<Error<T>> for ParseError
where
    T: fmt::Debug,
{
    fn from(value: Error<T>) -> Self {
        ParseError::NomError(format!("{value:?}"))
    }
}

impl Ast {
    /// Gets a string representation of this Ast and the (possible) tree-formed structure
    /// within
    ///
    /// # Arguments
    ///
    /// returns: a string representing the tree
    ///
    /// # Examples
    /// ```text
    /// Document:
    ///   Paragraph:
    ///     > I love the equation
    ///     math(form=latex){x^2}
    /// ```
    pub fn tree_string(&self) -> String {
        pretty_ast(self).join("\n")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    pub tag_name: String,
    pub elements: Vec<Ast>,
    pub recurse: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paragraph {
    pub elements: Vec<Ast>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub elements: Vec<Ast>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub name: String,
    pub args: MaybeArgs,
    pub body: String,
    pub one_line: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub elements: Vec<Ast>,
}

/// Parses the source document. If the parser errors out, a placeholder `Document` is returned
/// with the error inserted
///
/// # Arguments
///
/// * `source`: The source text to parse
///
/// returns: The parsed document tree
pub fn parse(source: &str) -> Result<Ast, ParseError> {
    Ok(Ast::Document(parse_to_ast_document(source)?))
}

/// Parses the source document and returns it as a `document`. If the parser errors out, a
/// placeholder `document` is returned with the error inserted
///
/// # Arguments
///
/// * `source`: The source text to parse
///
/// returns: Element The parsed element
pub fn parse_to_ast_document(source: &str) -> Result<Document, ParseError> {
    parse_document(source)
        .finish()
        .map(|(_, x)| x)
        .map_err(|e| e.into())
}

/// Parses a document, which consists of multiple paragraphs and block modules, and returns a
/// `Node` with the name `Document` containing all paragraphs
///
/// # Arguments
///
/// * `input`: The input to parse
///
/// returns: Result<(&str, Element), Err<Error<I>>>
fn parse_document(input: &str) -> IResult<&str, Document> {
    map(parse_document_blocks, |blocks| Document {
        elements: blocks,
    })(input)
}

pub fn parse_blocks(input: &str) -> Result<Vec<Ast>, ParseError> {
    let (_, blocks) = parse_document_blocks(input).finish()?;
    Ok(blocks)
}

pub fn parse_inline(input: &str) -> Result<Vec<Ast>, ParseError> {
    let (_, inline) = parse_paragraph_elements(input).finish()?;
    Ok(inline)
}

/// Parses multiple paragraphs or multiline modules, separated by two or more line endings.
/// The result will be a vector of the elements parsed, where each element is either a
/// multiline module invocation or a `Paragraph` node.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: A vector of ASTs where each AST is either a multiline module or a paragraph
fn parse_document_blocks(input: &str) -> IResult<&str, Vec<Ast>> {
    preceded(
        many0(line_ending),
        separated_list0(
            many1(line_ending),
            map(module::parse_multiline_module, Ast::Module)
                .or(map(parse_heading, Ast::Heading))
                .or(map(parse_nonempty_paragraph, Ast::Paragraph)),
        ),
    )(input)
}

/// Parses a heading which consists of a sequence of hashtags to indicate heading level
/// followed by text. The level and text are put into a `Heading` node.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: The heading node, if a successful parse occurs, otherwise the parse error
fn parse_heading(input: &str) -> IResult<&str, Heading> {
    map(
        pair(
            verify(take_while1(|c| c == '#'), |s: &str| {
                s.len() <= u8::MAX as usize
            }),
            preceded(space0, parse_heading_text).and_then(parse_paragraph_elements),
        ),
        |(start, elements)| Heading {
            level: start.len() as u8,
            elements,
        },
    )(input)
}

/// Parses the text for a heading, consuming until a line ending is found.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: The parsed text, if a successful parse occurs, otherwise the parse error
fn parse_heading_text(input: &str) -> IResult<&str, &str> {
    take_till(|c| c == '\r' || c == '\n')(input)
}

/// Parses a paragraph which consists of multiple paragraph elements, and puts all those into a
/// `Paragraph` node. In addition to `parse_paragraph`, this ensures that it has at least one
/// paragraph element. This is due to a change introduced in `parse_paragraph_elements` which allows
/// it to parse the empty string, which results in a `Paragraph` with no elements.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: The paragraph node, if a successful parse occurs, otherwise the parse error
fn parse_nonempty_paragraph(input: &str) -> IResult<&str, Paragraph> {
    verify(parse_paragraph, |p| !p.elements.is_empty())(input)
}

/// Parses a paragraph which consists of multiple paragraph elements, and puts all those into a
/// `Paragraph` node.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: The paragraph node, if a successful parse occurs, otherwise the parse error
fn parse_paragraph(input: &str) -> IResult<&str, Paragraph> {
    map(parse_paragraph_elements, |elems| Paragraph {
        elements: elems,
    })(input)
}

/// Gets the Ast elements for the paragraph starting at the start of the string. A paragraph runs
/// until two line endings following each other, and may thus span multiple lines.
///
/// The parsing is done in three steps:
///  1. Each position of the string is parsed. These elements are attempted at being parsed,
///     and the first one matching succeeds, in order:
///     * An inline module is attempted at parsing
///     * An escaped newline, in which case both the backslash and newline are removed
///     * An escaped character, in which case both the character and backslash is retained
///     * An character which isn't a newline
///     * A newline not immediately following another newline (the following char is not consumed)
///     During this step, the result is folded into a (Vec<Ast>, String) after each parse, pushing
///     to the accumulator string if appropriate and if, let's say, a module is found, this happens:
///     * The accumulator string is turned into a Text element (if non-empty)
///     * The Text element is pushed to the accumulator vector
///     * The module is pushed to the accumulator vector
///     After this, if the accumulator string is non-empty, it gets added to the end of the Ast.
///  2. A tag search is started, finding all tags (like **, // etc) in all text nodes in the tree.
///     When a tag pair is found, the element it encases are drained and added into a Tag node.
///     The tag node is then added to the Ast where the elements were drained. After that, the
///     string where the tags was found is split at the position of the tags, and the prefix and
///     suffix are added back as text nodes. Depending on the tag type and configuration, the tag
///     search may continue recursively. See [tag::extract_tags] for more information.
///  3. All text nodes are traversed once again, removing all escaping backslashes. The
///     backslashes have been respected up to this point, and it was needed for them to be retained
///     in the string as to allow the different steps to find them (since we don't tokenize), but
///     since the parsing is done, we remove them.
///
/// # Arguments
///
/// * `input`: The input to parse
///
/// returns: A list of the elements that the paragraph contains, or a parsing error
fn parse_paragraph_elements(input: &str) -> IResult<&str, Vec<Ast>> {
    map(
        map(
            map(
                fold_many0(
                    or::or5(
                        module::parse_inline_module,
                        preceded(char('\\'), line_ending),
                        preceded(char('\\'), none_of("\r\n")),
                        none_of("\r\n"),
                        // note: do NOT use not_line_ending, it matches successfully on empty string
                        // so that would break this
                        terminated(line_ending, peek(none_of("\r\n"))),
                    ),
                    || (Vec::new(), String::new()),
                    |(acc_vec, acc_str),
                     (
                        opt_inline,
                        _opt_esc_line_ending,
                        opt_escaped_char,
                        opt_char,
                        opt_line_ending,
                    )| {
                        let mut elems = acc_vec;
                        let mut string = acc_str;

                        if let Some(module) = opt_inline {
                            if !string.is_empty() {
                                elems.push(Text(mem::take(&mut string)))
                            }
                            elems.push(Ast::Module(module));
                        } else if let Some(char) = opt_escaped_char {
                            string.push('\\');
                            string.push(char)
                        } else if let Some(n_char) = opt_char {
                            string.push(n_char);
                        } else if let Some(line_ending) = opt_line_ending {
                            string.push_str(line_ending);
                        }

                        // If there is an escaped newline, we can remove both the backslash
                        // and the newline. This means that "pre\LFpost" becomes "prepost",
                        // and since this won't touch other backslashes or already-escaped
                        // backslashes, this will work. If we have "\\LF", both "\\" will be
                        // caught by opt_escaped_char and thus \LF won't be caught by
                        // _opt_esc_line_ending

                        (elems, string)
                    },
                ),
                |(mut a, b)| {
                    if !b.is_empty() {
                        a.push(Text(b))
                    }
                    a
                },
            ),
            tag::extract_tags,
        ),
        |mut x| {
            smart_punctuate(&mut x);
            remove_escape_chars(&mut x);
            x
        },
    )(input)
}

/// Remove all appropriate characters related to escaping characters from the string. Currently,
/// this includes removing the backslashes escaping another character, like this:
///
/// |input | output|
/// |------|-------|
/// |`\**` | `**`  |
/// |`\\`  | `\`   |
/// |`\\\a`| `\a`  |
///
/// The function takes a mutable `CompoundAST` and walks through it, mutating its texts in-place
fn remove_escape_chars<T>(input: &mut T)
where
    T: CompoundAST,
{
    input.elements_mut().iter_mut().for_each(|e| match e {
        Text(str) => {
            let mut escaped = false;
            str.retain(|c| {
                if escaped {
                    escaped = false;
                    // this "true" returns on an escaped character, saying that it should be
                    // retained. if some escaped characters are to be deleted, this is the place to
                    // delete them. if we decide on that backslashes shouldn't be deleted at all
                    // when preceded by [a-zA-Z0-9], we have to change this .retain to something
                    // else since it doesn't support looking ahead
                    true
                } else if c == '\\' {
                    escaped = true;
                    false
                } else {
                    true
                }
            });
        }
        Ast::Document(d) => {
            remove_escape_chars(&mut d.elements);
        }
        Ast::Paragraph(p) => {
            remove_escape_chars(&mut p.elements);
        }
        Ast::Tag(t) => {
            remove_escape_chars(&mut t.elements);
        }
        _ => {}
    });
}

/// Converts an Ast into a vector of strings suitable for a text representation.
///
/// # Arguments
///
/// * `ast`: The Ast to convert
///
/// returns: a vector of strings suitable for printing row by row
fn pretty_ast(ast: &Ast) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    fn children_ast(elements: &[Ast]) -> Vec<String> {
        let mut strs = vec![];
        let indent = "  ";
        if elements.is_empty() {
            strs.push(format!("{indent}[no elements]"));
        } else {
            elements.iter().for_each(|c| {
                pretty_ast(c)
                    .iter()
                    .for_each(|s| strs.push(format!("{indent}{s}")))
            });
        }
        strs
    }

    match ast {
        Text(str) => str.lines().enumerate().for_each(|(idx, line)| {
            strs.push(format!("{} {line}", if idx == 0 { '>' } else { '|' }))
        }),
        Ast::Document(Document { elements }) => {
            strs.push("Document:".to_string());
            strs.append(&mut children_ast(elements));
        }

        Ast::Paragraph(Paragraph { elements }) => {
            strs.push("Paragraph:".to_string());
            strs.append(&mut children_ast(elements));
        }

        Ast::Tag(Tag {
            tag_name,
            elements,
            recurse: _,
        }) => {
            strs.push(format!("{tag_name}:"));
            strs.append(&mut children_ast(elements));
        }

        Ast::Module(Module {
            name,
            args,
            body,
            one_line,
        }) => {
            let args = match args {
                MaybeArgs::ModuleArguments(arguments) => {
                    let p1 = &arguments.positioned;
                    let p2 = arguments.named.as_ref().map(|args| {
                        args.iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<String>>()
                    });

                    let mut args_vec = p1.clone().unwrap_or_default();
                    args_vec.extend_from_slice(&p2.unwrap_or_default());
                    args_vec.join(", ")
                }
                MaybeArgs::Error(_) => "ERR".to_string(),
            };
            if *one_line {
                strs.push(format!("{name}({args}){{{body}}}"));
            } else {
                strs.push(format!("{name}({args}){{"));
                body.lines().enumerate().for_each(|(idx, line)| {
                    strs.push(format!(
                        "{indent}{} {line}",
                        if idx == 0 { '>' } else { '|' }
                    ))
                });
                strs.push("} [multiline invocation]".to_string());
            };
        }

        Ast::Heading(Heading { level, elements }) => {
            strs.push(format!("Heading [level {level}]:"));
            elements.iter().for_each(|c| {
                pretty_ast(c)
                    .iter()
                    .for_each(|s| strs.push(format!("{indent}{s}")))
            });
        }
    }

    strs
}
