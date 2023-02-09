extern crate core;

use std::collections::HashMap;
use std::mem;

use nom::bytes::complete::{take, take_till, take_until, take_until1, take_while1};
use nom::character::complete::{
    char, line_ending, multispace0, multispace1, none_of, space0, space1,
};
use nom::error::Error;
use nom::multi::{fold_many1, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated};
use nom::{
    branch::*, bytes::complete::tag, combinator::*, FindSubstring, Finish, IResult, InputTake,
    Parser,
};

use Element::Node;

use crate::Ast::Text;
use crate::Element::{Data, ModuleInvocation};

mod or;

#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    Data(String),
    Node {
        name: String,
        environment: HashMap<String, String>,
        children: Vec<Element>,
    },
    ModuleInvocation {
        name: String,
        args: ModuleArguments,
        body: String,
        one_line: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModuleArguments {
    pub positioned: Option<Vec<String>>,
    pub named: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    Text(String),
    Document(Document),
    Paragraph(Paragraph),
    Tag(Tag),
    Module(Module),
}

impl Ast {
    pub fn tree_string(&self) -> String {
        pretty_ast(self).join("\n")
    }
}

impl From<Ast> for Element {
    fn from(value: Ast) -> Self {
        match value {
            Text(s) => Data(s),
            Ast::Document(doc) => Node {
                name: "Document".to_string(),
                environment: HashMap::new(),
                children: doc.elements.into_iter().map(|e| e.into()).collect(),
            },
            Ast::Paragraph(paragraph) => Node {
                name: "Paragraph".to_string(),
                environment: HashMap::new(),
                children: paragraph.elements.into_iter().map(|e| e.into()).collect(),
            },
            Ast::Tag(tag) => Node {
                name: tag.tag_name,
                environment: HashMap::new(),
                children: tag.elements.into_iter().map(|e| e.into()).collect(),
            },
            Ast::Module(module) => ModuleInvocation {
                name: module.name,
                args: module.args,
                body: module.body,
                one_line: module.one_line,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    pub tag_name: String,
    pub elements: Vec<Ast>,
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
    pub args: ModuleArguments,
    pub body: String,
    pub one_line: bool,
}

/// A trait implemented by data types which contains a Vec of `Ast`s. It contains two methods:
/// one for getting a reference to that vec, and one for getting a mutable reference to that vec.
trait CompoundAST {
    fn elements(&self) -> &Vec<Ast>;
    fn elements_mut(&mut self) -> &mut Vec<Ast>;
}

impl CompoundAST for Tag {
    fn elements(&self) -> &Vec<Ast> {
        &self.elements
    }

    fn elements_mut(&mut self) -> &mut Vec<Ast> {
        &mut self.elements
    }
}

impl CompoundAST for Paragraph {
    fn elements(&self) -> &Vec<Ast> {
        &self.elements
    }

    fn elements_mut(&mut self) -> &mut Vec<Ast> {
        &mut self.elements
    }
}

impl CompoundAST for Document {
    fn elements(&self) -> &Vec<Ast> {
        &self.elements
    }

    fn elements_mut(&mut self) -> &mut Vec<Ast> {
        &mut self.elements
    }
}

impl CompoundAST for Vec<Ast> {
    fn elements(&self) -> &Vec<Ast> {
        self
    }

    fn elements_mut(&mut self) -> &mut Vec<Ast> {
        self
    }
}

impl Element {
    /// Gets a string representation of this element and the (possible) tree-formed structure
    /// within
    ///
    /// # Arguments
    ///
    /// * `include_environment`: whether or not the environment variables of the node
    ///         should be printed out individually. If false, only the amount of variables
    ///         will be printed.
    ///
    /// returns: a string representing the tree
    ///
    /// # Examples
    ///
    /// ```text
    /// Document {
    ///   env: { <empty> }
    ///   children: [
    ///     Paragraph {
    ///       env: { <empty> }
    ///       children: [
    ///         > I love the equation
    ///         math(form=latex){x^2}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    pub fn tree_string(&self, include_environment: bool) -> String {
        pretty_rows(self, include_environment).join("\n")
    }
}

/// Parses the source document. If the parser errors out, a placeholder `Document` is returned
/// with the error inserted
///
/// # Arguments
///
/// * `source`: The source text to parse
///
/// returns: Element The parsed element
pub fn parse(source: &str) -> Element {
    parse_to_ast(source).into()
}

pub fn parse_to_ast(source: &str) -> Ast {
    Ast::Document(parse_to_ast_document(source))
}

/// Parses the source document and returns it as a `document`. If the parser errors out, a
/// placeholder `document` is returned with the error inserted
///
/// # Arguments
///
/// * `source`: The source text to parse
///
/// returns: Element The parsed element
pub fn parse_to_ast_document(source: &str) -> Document {
    parse_document(source)
        .finish()
        .map(|(_, x)| x)
        .map_err(|e| dbg!(e))
        .unwrap_or_else(|e: Error<&str>| Document {
            elements: vec![
                Text("Document failed to parse".to_string()),
                Text(format!("Error: {e}")),
            ],
        })
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

/// Parses multiple paragraphs or multiline modules, separated by two or more line endings.
/// The result will be a vector of the elements parsed, where each element is either a
/// multiline module invocation or a `Paragraph` node.
///
/// # Arguments
///
/// * `input`: The text to parse
///
/// returns: Result<(&str, Vec<Element, Global>), Err<Error<I>>>
fn parse_document_blocks(input: &str) -> IResult<&str, Vec<Ast>> {
    separated_list0(
        preceded(line_ending, many1(line_ending)),
        map(parse_multiline_module, Ast::Module).or(map(parse_paragraph, Ast::Paragraph)),
    )(input)
}

/// Parses a paragraph which consists of multiple paragraph elements, and puts all those into a
/// `Paragraph` node.
///
/// # Arguments
///
/// * `input`:
///
/// returns: Result<(&str, Element), Err<Error<I>>>
fn parse_paragraph(input: &str) -> IResult<&str, Paragraph> {
    map(parse_paragraph_elements, |elems| Paragraph {
        elements: elems,
    })(input)
}

fn parse_paragraph_elements(input: &str) -> IResult<&str, Vec<Ast>> {
    map(
        map(
            map(
                fold_many1(
                    or::or5(
                        parse_inline_module,
                        preceded(char('\\'), line_ending),
                        tag(r"\["),
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
                        opt_escape_mod,
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
                        } else if opt_escape_mod.is_some() {
                            string.push('[');
                        } else if let Some(n_char) = opt_char {
                            string.push(n_char);
                        } else if let Some(line_ending) = opt_line_ending {
                            string.push_str(line_ending);
                        }
                        (elems, string)
                    },
                ),
                |(a, b)| {
                    let mut elems = a;
                    if !b.is_empty() {
                        elems.push(Text(b))
                    }
                    elems
                },
            ),
            extract_tags,
        ),
        |mut x| {
            remove_escape_chars(&mut x);
            x
        },
    )(input)
}

#[test]
fn test_second_pass() {
    dbg!(extract_tags(vec![
        Text("abc def ** ghi".to_string()),
        Text("middle".to_string()),
        Text("abc def ** ghi".to_string())
    ]));
    remove_escape_chars(&mut dbg!(extract_tags(vec![Text(
        "abc def **ghi** jkl".to_string()
    ),])));
    remove_escape_chars(&mut dbg!(extract_tags(vec![Text(
        "abc def **ghi** jkl".to_string()
    ),])));

    print!(
        "{}",
        pretty_ast(&parse_to_ast("this //is some **text** yeah//")).join("\n")
    );

    //dbg!(remove_escape_chars(x));

    ()
}

fn remove_escape_chars(input: &mut [Ast]) {
    input.iter_mut().for_each(|e| match e {
        Text(str) => {
            let mut escaped = false;
            str.retain(|c| {
                if escaped {
                    escaped = false;
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

fn extract_tags(mut input: Vec<Ast>) -> Vec<Ast> {
    let bold = TagDefinition::new("Bold", ("**", "**"), true);
    let italic = TagDefinition::new("Italic", ("//", "//"), true);
    let subscript = TagDefinition::new("Subscript", ("__", "__"), true);
    let superscript = TagDefinition::new("Superscript", ("^^", "^^"), true);
    let verbatim = TagDefinition::new("Verbatim", ("``", "``"), false);
    let underlined = TagDefinition::new("Underlined", ("==", "=="), true);
    let strikethrough = TagDefinition::new("Strikethrough", ("~~", "~~"), true);

    let defs = vec![
        &bold,
        &italic,
        &subscript,
        &superscript,
        &verbatim,
        &underlined,
        &strikethrough,
    ];
    extract_all_tags(&defs, &mut input);
    input
}

fn extract_all_tags<T>(tags: &[&TagDefinition], input: &mut T)
where
    T: CompoundAST,
{
    let mut search_idx = (0usize, 0usize);
    while let Some(((start_elem_idx, start_str_idx), tag)) =
        find_opening_tag(search_idx, input, tags)
    {
        if let Some((end_elem_idx, end_str_idx)) = find_closing_tag(
            (start_elem_idx, start_str_idx + tag.delimiters.0.len()),
            tag,
            input,
        ) {
            let tag_idx = extract_tag(
                tag,
                (start_elem_idx, start_str_idx),
                (end_elem_idx, end_str_idx),
                input,
            );
            if tag.recurse {
                let tag = &mut input.elements_mut()[tag_idx];
                match tag {
                    Ast::Tag(tag) => extract_all_tags(tags, tag),
                    x => panic!("Expected tag at tag position, got {x:?}"),
                }
            }
        }
        search_idx = (start_elem_idx, start_str_idx + tag.delimiters.0.len())
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TagDefinition {
    name: String,
    delimiters: (String, String),
    recurse: bool, //maybe add an option span_invocations?
}

impl TagDefinition {
    fn new(name: &str, (opening, closing): (&str, &str), recurse: bool) -> Self {
        Self {
            name: name.to_string(),
            delimiters: (opening.to_string(), closing.to_string()),
            recurse,
        }
    }
}

type CompoundPos = (usize, usize);

// returns index of the extracted thing
// cant return a ref since its already moved
fn extract_tag<T>(
    tag: &TagDefinition,
    (idx_elem_start, idx_str_start): CompoundPos,
    (idx_elem_end, idx_str_end): CompoundPos,
    ast: &mut T,
) -> usize
where
    T: CompoundAST,
{
    // in the simplest case, idx_elem_start == idx_elem_end, which means that the tag is just
    // in the same text. we then extract prefix, middle and suffix:
    // aaa**bb**ccc
    // prefix: aaa
    // middle: bb
    // suffix: ccc
    // and we can then replace the original node by the tag, insert the suffix after, then insert prefix

    if idx_elem_start == idx_elem_end {
        let original_text = ast
            .elements()
            .iter()
            .nth(idx_elem_start)
            .map(|e| match e {
                Text(text) => text,

                _ => panic!("Expected Text(...) at index {idx_elem_start} (start), got: {e:?}"),
            })
            .expect("Expected string at tag element indices (start==end)");
        let prefix = original_text[..idx_str_start].to_string();
        let middle = original_text[idx_str_start + tag.delimiters.0.len()..idx_str_end].to_string();
        let suffix = original_text[idx_str_end + tag.delimiters.0.len()..].to_string();

        let content = if middle.is_empty() {
            vec![]
        } else {
            vec![Text(middle)]
        };
        let tag_element = Tag {
            tag_name: tag.name.to_string(),
            elements: content,
        };

        ast.elements_mut().push(Ast::Tag(tag_element));
        ast.elements_mut().swap_remove(idx_elem_start);
        if !suffix.is_empty() {
            ast.elements_mut().insert(idx_elem_start + 1, Text(suffix));
        }
        if !prefix.is_empty() {
            ast.elements_mut().insert(idx_elem_start, Text(prefix));
            // one thing is inserted before the tag element, so the tag element is shifted by 1
            return idx_elem_start + 1;
        }
        return idx_elem_start;
    }

    let (start_prefix, start_suffix) = ast
        .elements()
        .iter()
        .nth(idx_elem_start)
        .map(|e| match e {
            Text(text) => (
                text[..idx_str_start].to_string(),
                text[idx_str_start + tag.delimiters.0.len()..].to_string(),
            ),
            _ => panic!("Expected Text(...) at index {idx_elem_start} (start), got: {e:?}"),
        })
        .expect("Expected string at tag start indices");

    let (end_prefix, end_suffix) = ast
        .elements()
        .iter()
        .nth(idx_elem_end)
        .map(|e| match e {
            Text(text) => (
                text[..idx_str_end].to_string(),
                text[idx_str_end + tag.delimiters.1.len()..].to_string(),
            ),
            _ => panic!("Expected Text(...) at index {idx_elem_end} (end), got: {e:?}"),
        })
        .expect("Expected string at tag end indices");

    // lets say we want to extract around *, in this example:
    // "before" "ab*c" [module] "de*f" "after"
    // we have found start_prefix="ab" start_suffix="c"
    // end_prefix = "de" end_suffix = "f"
    // first we drain the elements:
    // elems: "before" "after"
    // drain: "ab*c" [module] "de*f"
    // then, we remove the last element and add the end prefix if non-empty (constant both ways)
    // then, if we have a start suffix, we swap "ab*c" for "c"
    // else we just remove it (constant time if swap, linear if remove)
    // then, we add the end_suffix to the drain spot (if non-empty)
    // then, we add the tag
    // then, we add start_prefix to the drain spot (if non-empty)

    let elems = ast.elements_mut();
    let mut removed_elems: Vec<Ast> = elems.drain(idx_elem_start..=idx_elem_end).collect();

    removed_elems.remove(removed_elems.len() - 1);
    if !end_prefix.is_empty() {
        removed_elems.push(Text(end_prefix))
    }
    if start_suffix.is_empty() {
        removed_elems.remove(0);
    } else {
        removed_elems.push(Text(start_suffix));
        removed_elems.swap_remove(0);
    }

    let tag = Tag {
        tag_name: tag.name.clone(),
        elements: removed_elems,
    };

    if !end_suffix.is_empty() {
        elems.insert(idx_elem_start, Text(end_suffix))
    }

    // the tag is initially inserted into idx_elem_start
    elems.insert(idx_elem_start, Ast::Tag(tag));

    if !start_prefix.is_empty() {
        elems.insert(idx_elem_start, Text(start_prefix));
        // one thing is inserted before the tag element, so the tag element is shifted by 1
        idx_elem_start + 1
    } else {
        // nothing is inserted before tag element
        idx_elem_start
    }
}

// Searches an compound AST for the first opening tag
fn find_opening_tag<'a, T>(
    (start_elem_idx, start_str_idx): CompoundPos,
    ast: &T,
    tags: &'a [&TagDefinition],
) -> Option<(CompoundPos, &'a TagDefinition)>
where
    T: CompoundAST,
{
    ast.elements()
        .iter()
        .enumerate()
        .skip(start_elem_idx)
        .find_map(|(elem_idx, elem)| match elem {
            Text(str) => {
                let offset = if elem_idx == start_elem_idx {
                    start_str_idx
                } else {
                    0
                };
                find_first_matching_tag(offset, str, tags, true)
                    .map(|(i, tag)| ((elem_idx, i), tag))
            }
            _ => None,
        })
}

///
///
/// # Arguments
///
/// * `(elem, idx)`:
/// * `tag`:
/// * `ast`:
///
/// returns: Option<(usize, usize)>
fn find_closing_tag<T>(
    (start_elem_idx, start_str_idx): CompoundPos,
    tag: &TagDefinition,
    ast: &T,
) -> Option<CompoundPos>
where
    T: CompoundAST,
{
    ast.elements()
        .iter()
        .enumerate()
        .skip(start_elem_idx)
        .find_map(|e| match e {
            (elem_idx, Text(str)) => {
                let offset = if elem_idx == start_elem_idx {
                    start_str_idx
                } else {
                    0
                };
                find_first_matching_tag(offset, str, &[tag], false).map(|(idx, _)| (elem_idx, idx))
            }
            _ => None,
        })
}

/// Finds the first matching tag in the given string. The tags are retrieved from a slice of
/// `TagDefinition`s, and either opening or closing tags are used based on the `opening` parameter.
/// It also takes the char index to start searching from, and it doesn't include tags escaped
/// by backslashes `\`
///
/// # Arguments
///
/// * `from`: The char index to start the search from (0 to search from the start of the string)
/// * `str`: The string to search in
/// * `tags`: A list of `TagDefinition`s to search for
/// * `opening`: `true` if it should search for opening tags, `false` if to search for closing tags
///
/// returns: The index where a tag is found and a reference to the tag definition it found, if any
///          tag is found, otherwise None.
fn find_first_matching_tag<'a>(
    from: usize,
    str: &str,
    tags: &'a [&'a TagDefinition],
    opening: bool,
) -> Option<(usize, &'a TagDefinition)> {
    let extract = |tag: &'a TagDefinition| {
        if opening {
            &tag.delimiters.0
        } else {
            &tag.delimiters.1
        }
    };
    let mut is_escaped = false;
    str.char_indices().skip(from).find_map(|(i, c)| {
        if is_escaped {
            is_escaped = false;
            return None;
        }
        if c == '\\' {
            is_escaped = !is_escaped;
        }
        tags.iter()
            .map(|t| (*t, extract(t)))
            .find_map(|(t, d)| str[i..].starts_with(d).then_some((i, t)))
    })
}

fn parse_inline_module(input: &str) -> IResult<&str, Module> {
    map(
        pair(get_module_invocation_parser(true), parse_inline_module_body),
        |((name, args), body)| Module {
            name,
            args,
            body: body.to_string(),
            one_line: true,
        },
    )(input)
}

fn parse_inline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim(true), get_inline_body_parser)(input)
}

fn get_inline_body_parser<'a>(
    delim: Option<&'_ str>,
) -> impl Parser<&'a str, &'a str, Error<&'a str>> + '_ {
    move |i: &'a str| {
        if let Some(opening_delim) = delim {
            let closing = closing_delim(opening_delim);
            let res = terminated(
                take_until_no_newlines(closing.as_str()),
                tag(closing.as_str()),
            )(i);
            res
        } else {
            preceded(space0, take_till(|c: char| c.is_ascii_whitespace()))(i)
        }
    }
}

/// This gives a parser which works just like `take_until`, but fails if `take_until` would take a
/// newline. See the documentation for `complete::take_until`. Note that this will use `fail` to
/// generate errors, and thus won't be as useful as the implementation of `take_until`
///
/// # Arguments
///
/// * `tag`: The tag to take
///
/// returns: a parser according to the description above
///
/// # Examples
/// For the tag `eof`:
/// | Input                           | Match         |
/// |---------------------------------|---------------|
/// | `hello, world!eof`              |`hello, world!`|
/// | `hello, \n world!eof"`          |`<Fail>`       |
/// | `hello, world!`                 |`<Fail>`       |
/// | `eof`                           |(empty string) |
///
// this will use take_until to take a substring until a given tag, but won't take any newlines.
// if a newline occurs before the tag, this will fail
// don't mention the body, it is copied from the definition of take_until
fn take_until_no_newlines(tag: &str) -> impl Fn(&str) -> IResult<&str, &str, Error<&str>> + '_ {
    move |i: &str| match i.find_substring(tag) {
        None => fail(i),
        Some(index) => {
            if i.find('\n').map_or(true, |i| i > index) {
                Ok(i.take_split(index))
            } else {
                fail(i)
            }
        }
    }
}

/// Parses optional delimiters for opening and closing modules. If the module is
/// inline, at most one character is allowed, and if it is multiline, any amount
/// of characters can be used.
///
/// # Arguments
///
/// * `inline`: whether the module is inline
///
/// returns: Result<(&str, Option<&str>), Err<Error<I>>>
///
fn parse_opening_delim<'a>(
    inline: bool,
) -> impl Fn(&'a str) -> IResult<&'a str, Option<&'a str>, Error<&'a str>> {
    move |i: &'a str| {
        if inline {
            opt(verify(take(1usize), |s: &str| {
                let c = s.chars().next().unwrap();
                !c.is_alphanumeric() && !c.is_whitespace()
            }))(i)
        } else {
            opt(take_while1(|c: char| {
                !c.is_alphanumeric() && !c.is_whitespace()
            }))(i)
        }
    }
}

/// Gets the appropriate closing delimiter for an opening delimiter for a body of a module
///
/// # Arguments
///
/// * `string`: The opening delimiter
///
/// returns: String
///
/// # Examples
///
/// | Input | Output |
/// |-------|--------|
/// |`---`  | `---`  |
/// |`((`   | `))`   |
/// |`({<*<`| `>*>})`|
fn closing_delim(string: &str) -> String {
    string
        .chars()
        .rev()
        .map(|c| match c {
            '(' => ')',
            '{' => '}',
            '[' => ']',
            '<' => '>',
            '»' => '«',
            '›' => '‹',
            ')' => '(',
            '}' => '{',
            ']' => '[',
            '>' => '<',
            '«' => '»',
            '‹' => '›',
            x => x,
        })
        .collect()
}

fn parse_multiline_module(input: &str) -> IResult<&str, Module> {
    map(
        pair(
            get_module_invocation_parser(false),
            parse_multiline_module_body,
        ),
        |((name, args), body)| Module {
            name,
            args,
            body: body.to_string(),
            one_line: false,
        },
    )(input)
}

fn parse_multiline_module_body(input: &str) -> IResult<&str, &str> {
    flat_map(parse_opening_delim(false), get_multiline_body_parser)(input)
}

fn get_multiline_body_parser<'a>(
    delim: Option<&'_ str>,
) -> impl Parser<&'a str, &'a str, Error<&'a str>> + '_ {
    move |i: &'a str| {
        if let Some(opening_delim) = delim {
            let closing = closing_delim(opening_delim);
            let res = delimited(
                line_ending,
                take_until(closing.as_str()),
                tag(closing.as_str()),
            )(i);
            res
        } else {
            preceded(
                line_ending,
                take_until1("\r\n\r\n").or(take_until1("\n\n").or(rest)),
            )(i)
        }
    }
}

/// Returns a parser for module invocations
///
/// # Arguments
///
/// * `input`: a bool indicating whether the returned parser is used as inline.
///
/// returns: impl Parser<&'a str, (String, ModuleArguments), Error<&'a str>>
///
fn get_module_invocation_parser<'a>(
    inline: bool,
) -> impl Parser<&'a str, (String, ModuleArguments), Error<&'a str>> {
    map(
        delimited(
            char('['),
            pair(
                parse_module_name,
                opt(delimited(
                    get_arg_separator_parser(inline),
                    get_module_args_parser(inline),
                    opt(get_arg_separator_parser(inline)),
                )),
            ),
            char(']'),
        ),
        |(name, args)| (name.to_string(), args.unwrap_or_default()),
    )
}

/// A parser for module names.
///
/// # Arguments
///
/// * `input`: the slice containing the name
///
/// returns: IResult<&str, &str>
///
fn parse_module_name(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c == '-' || c == '_' || c.is_ascii_alphanumeric())(input)
}

/// Returns a parser for parsing module arguments. Works both for named and positional.
/// Both are optional.
///
/// # Arguments
///
/// * `inline`: a bool indicating if the module is inline.
///
/// returns: impl Parser<&str, ModuleArguments, Error<&str>>+Sized
///
fn get_module_args_parser<'a>(
    inline: bool,
) -> impl Parser<&'a str, ModuleArguments, Error<&'a str>> {
    map(
        opt(alt((
            map(
                separated_pair(
                    separated_list1(
                        get_arg_separator_parser(inline),
                        get_unnamed_arg_parser(inline),
                    ),
                    get_arg_separator_parser(inline),
                    separated_list1(
                        get_arg_separator_parser(inline),
                        get_named_arg_parser(inline),
                    ),
                ),
                |(unnamed, named)| ModuleArguments {
                    positioned: Some(unnamed),
                    named: Some(named.into_iter().collect()),
                },
            ),
            map(
                separated_list1(
                    get_arg_separator_parser(inline),
                    get_unnamed_arg_parser(inline),
                ),
                |unnamed| ModuleArguments {
                    positioned: Some(unnamed),
                    named: None,
                },
            ),
            map(
                separated_list1(
                    get_arg_separator_parser(inline),
                    get_named_arg_parser(inline),
                ),
                |named| ModuleArguments {
                    positioned: None,
                    named: Some(named.into_iter().collect()),
                },
            ),
        ))),
        |x| x.unwrap_or_default(),
    )
}

/// Returns a parser parsing the separator of arguments. The separators are whitespace and optional comma.
///
/// # Arguments
///
/// * `inline`: a bool indicating if the parser is for inline
///
/// returns: impl Parser<&str, (), Error<&str>>+Sized
///

fn get_arg_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space1 } else { multispace1 };
    map(space, |_| ())
}

/// Parses the optional unnamed args on all unnamed arguments removing arg separators
///
/// # Arguments
///
/// * `inline`: All unnamed args
///
/// returns: impl Parser<&str, String, Error<&str>>+Sized
///
fn get_unnamed_arg_parser<'a>(inline: bool) -> impl Parser<&'a str, String, Error<&'a str>> {
    map(
        terminated(arg_value_parser, peek(not(get_kv_separator_parser(inline)))),
        |s| s.to_string(),
    )
}

/// Gets a parser which parses a named argument, eg `lang = python`, and returns a key-value pair
/// of owned Strings.
///
/// See `arg_name_parser`, `arg_value_parser` and `get_kv_separator_parser` for more info about
/// rules for argument names, values and separators
///
/// # Arguments
///
/// * `inline`: if the inline or multiline ruleset should be followed
///
/// # Examples
///
/// | Input                           | Match             |
/// |---------------------------------|-------------------|
/// | `apple=pie`                     | `(apple, pie)`    |
/// | `delim = "yes box"`             | `(delim, yes box)`|
/// | `"fake" = news`                 | `<Fail>`          |
/// | `<space>`                       | `<Fail>`          |
/// returns: a parser parsing one named argument
///
fn get_named_arg_parser<'a>(
    inline: bool,
) -> impl Parser<&'a str, (String, String), Error<&'a str>> {
    map(
        separated_pair(
            arg_key_parser,
            get_kv_separator_parser(inline),
            arg_value_parser,
        ),
        |(a, b)| (a.to_string(), b.to_string()),
    )
}

/// Parses an argument key and returns it. It may contain any alphanumeric characters and
/// underscores. It consumes the captured argument name, and nothing more
///
/// # Arguments
///
/// * `input`: The string to parse
///
/// # Examples
///
/// | Input                           | Match     |
/// |---------------------------------|-----------|
/// | `apple=pie`                     | `apple`   |
/// | `delim = "yes box"`             | `delim`   |
/// | `"fake" = news`                 | `<Fail>`  |
/// | `<space>`                       | `<Fail>`  |
///
/// returns: The parsing result capturing the argument name
fn arg_key_parser(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

/// Parses the argument to a function removing optional quotation marks and returning the value.
///
/// # Arguments
///
/// * `input`: The string to parse
///
/// # Examples
///
/// | Input                           | Match          |
/// |---------------------------------|----------------|
/// | `python 3`                      | `python`       |
/// | `"Alice Parker" "Matt Steward"` | `Alice Parker` |
/// | `a_b_c_d e_f_g_h`               | `a_b_c_d`      |
/// | `!"#!€/"(`                      | `<Error>`      |
///
/// returns: a parser consuming and returning the match
///
fn arg_value_parser(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(char('"'), take_until1(r#"""#), char('"')),
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '_'),
    ))(input)
}

/// Gets a parser which consumes the key-value separator, `=` in `lang=python`, without returning
/// anything, and failing if the consumption failed
///
/// For inline, this is defined as `[ \t]*=[ \t]*`, and for multiline, this is defined as
/// `[ \t\r\n]*=[ \t\r\n]*`
///
/// # Arguments
///
/// * `inline`: a bool indicating if the parser is using the ruleset for inline modules
///
/// # Examples:
///
/// | Input (il=inline, ml=multiline) | Match          |
/// |---------------------------------|----------------|
/// | `banana`                        | `<Fail>`       |
/// | `<space>=<space>` (il/ml)       | `<Success>`    |
/// | `\n\n<space>=<space>\t\n` (ml)  | `<Success>`    |
/// | `<space>`                       | `<Fail>`       |
///
/// returns: a parser consuming but not returning the match
///
fn get_kv_separator_parser<'a>(inline: bool) -> impl Parser<&'a str, (), Error<&'a str>> {
    let space = if inline { space0 } else { multispace0 };
    map(delimited(space, char('='), space), |_| ())
}

fn pretty_ast(ast: &Ast) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    match ast {
        Text(str) => str.lines().enumerate().for_each(|(idx, line)| {
            strs.push(format!("{} {line}", if idx == 0 { '>' } else { '|' }))
        }),
        Ast::Document(Document { elements }) => {
            strs.push("Document:".to_string());
            if elements.is_empty() {
                strs.push(format!("{indent}[no elements]"));
            } else {
                elements.iter().for_each(|c| {
                    pretty_ast(c)
                        .iter()
                        .for_each(|s| strs.push(format!("{indent}{s}")))
                });
            }
        }

        Ast::Paragraph(Paragraph { elements }) => {
            strs.push("Paragraph:".to_string());
            if elements.is_empty() {
                strs.push(format!("{indent}[no elements]"));
            } else {
                elements.iter().for_each(|c| {
                    pretty_ast(c)
                        .iter()
                        .for_each(|s| strs.push(format!("{indent}{s}")))
                });
            }
        }

        Ast::Tag(Tag { tag_name, elements }) => {
            strs.push(format!("{tag_name}:"));
            if elements.is_empty() {
                strs.push(format!("{indent}[no elements]"));
            } else {
                elements.iter().for_each(|c| {
                    pretty_ast(c)
                        .iter()
                        .for_each(|s| strs.push(format!("{indent}{s}")))
                });
            }
        }

        Ast::Module(Module {
            name,
            args,
            body,
            one_line,
        }) => {
            let args = {
                let p1 = &args.positioned;
                let p2 = args.named.as_ref().map(|args| {
                    args.iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<String>>()
                });

                let mut args_vec = p1.clone().unwrap_or_default();
                args_vec.extend_from_slice(&p2.unwrap_or_default());
                args_vec.join(", ")
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
            }
        }
    }

    strs
}

/// Converts an Element into a vector of strings suitable for a text representation.
///
/// # Arguments
///
/// * `element`: The element to convert
/// * `include_environment`: whether or not the environment variables of the node
///         should be printed out individually. If false, only the amount of variables
///         will be printed.
///
/// returns: a vector of strings suitable for printing row by row
fn pretty_rows(element: &Element, include_environment: bool) -> Vec<String> {
    let indent = "  ";
    let mut strs = vec![];

    match element {
        Data(str) => str.lines().enumerate().for_each(|(idx, line)| {
            strs.push(format!("{} {line}", if idx == 0 { '>' } else { '|' }))
        }),
        Node {
            name,
            environment,
            children,
        } => {
            strs.push(format!("{name} {{"));
            if environment.is_empty() {
                strs.push(format!("{indent}env: {{ <empty> }}"));
            } else if include_environment {
                strs.push(format!("{indent}env: {{"));
                environment
                    .iter()
                    .for_each(|(k, v)| strs.push(format!(r#"{indent}{indent}"{k}": "{v}""#)));

                strs.push(format!("{indent}}}"));
            } else {
                strs.push(format!(
                    "{indent}env: {{ < {len} entries > }}",
                    len = &environment.len().to_string()
                ))
            }

            if children.is_empty() {
                strs.push(format!("{indent}children: [ none ]"));
            } else {
                strs.push(format!("{indent}children: ["));

                children.iter().for_each(|c| {
                    pretty_rows(c, include_environment)
                        .iter()
                        .for_each(|s| strs.push(format!("{indent}{indent}{s}")))
                });

                strs.push(format!("{indent}]"));
            }
            strs.push("}".to_string());
        }

        ModuleInvocation {
            name,
            args,
            body,
            one_line,
        } => {
            let args = {
                let p1 = &args.positioned;
                let p2 = args.named.as_ref().map(|args| {
                    args.iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<String>>()
                });

                let mut args_vec = p1.clone().unwrap_or_default();
                args_vec.extend_from_slice(&p2.unwrap_or_default());
                args_vec.join(", ")
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
            }
        }
    }
    strs
}
