extern crate core;

use std::collections::HashMap;
use std::mem;

use nom::bytes::complete::{take, take_till, take_until, take_until1, take_while1};
use nom::character::complete::{
    char, line_ending, multispace0, multispace1, none_of, space0, space1,
};
use nom::error::Error;
use nom::multi::{fold_many1, many0, many1, separated_list0, separated_list1};
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
/// returns: A vector of ASTs where each AST is either a multiline module or a paragraph
fn parse_document_blocks(input: &str) -> IResult<&str, Vec<Ast>> {
    preceded(
        many0(line_ending),
        separated_list0(
            preceded(line_ending, many1(line_ending)),
            map(parse_multiline_module, Ast::Module).or(map(parse_paragraph, Ast::Paragraph)),
        ),
    )(input)
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
///     search may continue recursively. See [extract_tags] for more information.
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
                fold_many1(
                    or::or5(
                        parse_inline_module,
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
            extract_tags,
        ),
        |mut x| {
            remove_escape_chars(&mut x);
            x
        },
    )(input)
}

fn remove_escape_chars(input: &mut [Ast]) {
    input.iter_mut().for_each(|e| match e {
        Text(str) => {
            let mut escaped = false;
            str.retain(|c| {
                if escaped {
                    escaped = false;
                    // we want to return false for newlines, so we remove them
                    // all other escaped characters are retained as-is
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

/// This function extracts tags in all text nodes in the input. It delegates the work to
/// [extract_all_tags], see that comment for more information.
/// Currently, the tags are defined in here and [extract_all_tags] uses them. If new tags are to
/// be added, this would be the place to add them.
fn extract_tags(mut input: Vec<Ast>) -> Vec<Ast> {
    let bold = TagDefinition::new("Bold", ("**", "**"), true);
    let italic = TagDefinition::new("Italic", ("//", "//"), true);
    let subscript = TagDefinition::new("Subscript", ("__", "__"), true);
    let superscript = TagDefinition::new("Superscript", ("^^", "^^"), true);
    let underlined = TagDefinition::new("Underlined", ("==", "=="), true);
    let strikethrough = TagDefinition::new("Strikethrough", ("~~", "~~"), true);
    let verbatim = TagDefinition::new("Verbatim", ("``", "``"), false);
    let math = TagDefinition::new("Math", ("$$", "$$"), false);

    let defs = vec![
        &bold,
        &italic,
        &subscript,
        &superscript,
        &verbatim,
        &underlined,
        &strikethrough,
        &math,
    ];
    extract_all_tags(&defs, &mut input);
    input
}

/// Extracts all tags from the given compound Ast, matching the given tag definition. The term
/// "extracting" means taking some elements previously laying flat in the tree, removing them from
/// the tree, create a new node for those elements and then inserting it in the position where the
/// elements were extracted.
///
/// This function has a cursor (`search_index`) which starts at the first character of the first
/// element and moves further along the Ast the more the process continues. The function is mainly
/// one loop, using helper functions for searching and the actual extraction. This is what it does:
///  1. First, [find_opening_tag] is called, which searches the Ast and, for all text nodes, tries
///     to match each tag at each position, starting at the beginning. This ensures that the tag
///     to be returned is the first tag occurrence. [find_opening_tag] takes the cursor as a
///     parameter as well, and starts the search from there. The function optionally returns the
///     position where the first tag was found together with a reference to it.
///  2. Secondly, [find_closing_tag] is called, attempting to find the matching closing somewhere
///     after the opening tag was found.
///  3. If both an opening tag and matching closing tag was found, [extract_tag] is called to
///     do the extraction, i.e slicing the strings at the tag positions, draining elements
///     in between, making a new [Tag] and inserting it into the tree.
///     If the closing tag is not found, the cursor is set to the position where the opening tag
///     was found, incremented by the length of the opening tag, and the loop starts from the top.
///  4. If the [TagDefinition] says that the tag extraction should continue recursively, it does so
///     on the extracted [Tag].
///
/// # Arguments:
/// * `tags`: The tags to extract
/// * `input`: The AST to extract the tags from. Modifications will occur in-place
///
fn extract_all_tags<T>(tags: &[&TagDefinition], input: &mut T)
where
    T: CompoundAST,
{
    let mut search_idx = (0usize, 0usize);
    while let Some(((start_elem_idx, start_str_idx), tag)) =
        find_opening_tag(search_idx, tags, input)
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
                    _x => panic!("Expected tag at tag position, got {_x:?}"),
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

/// The position of a character inside a compound AST. One compound AST consists of a list of
/// children, and those children can be any of the types defined in the AST enum. You can position
/// one character in an AST by first giving the index of the text element, and then giving the
/// character index inside that text element. This is a type alias for one such pair; it holds the
/// index of the element as the first element, and the index oc the character as the second element.
type CompoundPos = (usize, usize);

/// This function extracts an already found tag from the given Ast. See [CompoundPos] for info about
/// the positions. The function takes two positions as inputs, and extracts all text and elements
/// in between them. It also takes the matched tag definition as input, and uses that to know the
/// length of the prefixes to subtract. The two positions can be in one of two cases, and the
/// behaviour is different:
///  1. Both positions point to the same element:
///     * This means that the text to extract is from the same Text node.
///     * The function simply splits the text in three parts, one for the part before the
///         tag, one for the part between the opening and closing tag, and one for the part
///         after the closing tag.
///     * A tag is created containing the middle part, and the start part,
///         then the tag, and then the end part is added to the tree, replacing the original Text.
///     * The algorithm ensures that no empty texts are added.
///  2. The positions are in different elements:
///     * This means that the tags span several elements, possibly inline modules, texts and other
///         tags (most probably not other tags).
///     * The text of the first position is located, and is split at the tag position into two
///         Text nodes, where one should be inside the tag and one outside.
///     * The text of the second position is located, and is split at the tag position into two
///         Text nodes, where one should be inside the tag and one outside.
///     * All elements between the two different split positions are removed from the Ast and moved
///         into a new Tag-type Ast.
///     * That new Ast is inserted where the previous elements were removed.
///     * The algorithm ensures that no empty texts are added.
///
/// Note that the function behaves as if it does everything stated above, but the actions done may
/// differ slightly due to performance gains. See comments in the source code for the function for
/// more details.
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

/// Finds the first opening tag out of all opening tags in the list of the [TagDefinition]s given,
/// starting the search after a specific position.
///
/// # Arguments
///
/// * `(start_elem_idx, start_str_idx)`: The position to search from, see [CompoundPos]
/// * `tags`: A list of the definitions of the tags to search for
/// * `ast`: The Ast to search in
///
/// returns: Option<(usize, usize), &TagDefinition>, the position where the first tag was found,
///     see [CompoundPos], together with a reference to the tag found
fn find_opening_tag<'a, T>(
    (start_elem_idx, start_str_idx): CompoundPos,
    tags: &'a [&TagDefinition],
    ast: &T,
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

/// Finds the first closing tag according to the given [TagDefinition] after a specific position.
///
/// # Arguments
///
/// * `(elem, idx)`: The position to search from, see [CompoundPos]
/// * `tag`: The definition of the tag to search for
/// * `ast`: The Ast to search in
///
/// returns: Option<(usize, usize)>, the position where the closing tag was found, see [CompoundPos]
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
