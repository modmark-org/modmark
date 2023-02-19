//! This module provides functions for handling tags. When the main parser parses a paragraph,
//! escaped characters has the highest precedence, and after that the modules, and after that, the
//! tags. The tags are initially placed into the text segments containing them, and this module
//! exposes a function, [extract_tags], which goes through all text segments of an Ast, finds all
//! tags and moves the content of the tags out to a different Ast structure, [Tag]
use crate::Ast::Text;
use crate::{Ast, Document, Heading, Paragraph, Tag};

/// The position of a character inside a compound AST. One compound AST consists of a list of
/// children, and those children can be any of the types defined in the AST enum. You can position
/// one character in an AST by first giving the index of the text element, and then giving the
/// character index inside that text element. This is a type alias for one such pair; it holds the
/// index of the element as the first element, and the index oc the character as the second element.
type CompoundPos = (usize, usize);

/// The definition of a tag. It contains the tag name, a pair of delimiters where the first one is
/// the opening delimiter and the second one is the closing delimiter (not necessarily the same),
/// and whether or not the parsed content should recursively be searched for other tags.
#[derive(Debug, Clone, PartialEq)]
struct TagDefinition {
    name: String,
    delimiters: (String, String),
    recurse: bool, //maybe add an option span_invocations?
}

impl TagDefinition {
    /// A convenience constructor which takes `&str`s instead of `String`s to easier write
    /// literals.
    fn new(name: &str, (opening, closing): (&str, &str), recurse: bool) -> Self {
        Self {
            name: name.to_string(),
            delimiters: (opening.to_string(), closing.to_string()),
            recurse,
        }
    }
}

/// This function extracts tags in all text nodes in the input. It delegates the work to
/// [extract_all_tags], see that comment for more information.
/// Currently, the tags are defined in here and [extract_all_tags] uses them. If new tags are to
/// be added, this would be the place to add them.
pub fn extract_tags(mut input: Vec<Ast>) -> Vec<Ast> {
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
///
/// # Arguments:
/// * `tag`: The tag to extract
/// * `(idx_elem_start, idx_str_start)`: The position at the start of the extraction point
/// * `(idx_elem_end, idx_str_end)`: The position at the end of the extraction point
/// * `ast`: The AST to extract the tag. Modifications will occur in-place
///
/// returns: the index where the extracted tag is, within the modified ast
///
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

/// A trait implemented by data types which contains a Vec of `Ast`s. It contains two methods:
/// one for getting a reference to that vec, and one for getting a mutable reference to that vec.
/// CompoundAST is used in this module to simplify the code which requires that an input contains
/// an iterable over `Ast`s by requiring an `CompoundAST` rather than just an `Ast`, pattern-
/// matching, and `panic`ing if the wrong datatype.
///
/// The data types which implements this is:
///  * Document
///  * Paragraph
///  * Tag
///  * Vec<Ast>
pub trait CompoundAST {
    fn elements(&self) -> &Vec<Ast>;
    fn elements_mut(&mut self) -> &mut Vec<Ast>;
}

impl CompoundAST for Document {
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

impl CompoundAST for Tag {
    fn elements(&self) -> &Vec<Ast> {
        &self.elements
    }

    fn elements_mut(&mut self) -> &mut Vec<Ast> {
        &mut self.elements
    }
}

impl CompoundAST for Heading {
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
