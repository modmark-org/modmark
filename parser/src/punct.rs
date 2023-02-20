use crate::tag::CompoundAST;
use crate::Ast;
use crate::Ast::Text;
use std::mem;

const ENDASH: &str = "\u{2013}";
const EMDASH: &str = "\u{2014}";
const ELLIP: &str = "\u{2026}";
const LSQUO: &str = "\u{2018}";
const RSQUO: &str = "\u{2019}";
const LDQUO: &str = "\u{201C}";
const RDQUO: &str = "\u{201D}";

/// Replace certain sequences with unicode characters according to our specification for
/// smart punctuation.
///
/// The function takes a mutable `CompoundAST` and walks through it, mutating its texts in-place
pub fn smart_punctuate<T>(input: &mut T)
where
    T: CompoundAST,
{
    let elems: &mut Vec<Ast> = input.elements_mut();
    let mut open_single: Option<(usize, usize)> = None;
    let mut open_double: Option<(usize, usize)> = None;

    for elem_index in 0..elems.len() {
        let (prev, rest) = elems.as_mut_slice().split_at_mut(elem_index);
        match rest.get_mut(0) {
            Some(Text(str)) => {
                let mut chars = str.chars().peekable();
                let mut acc = String::new();
                let mut row = String::new();
                let mut seq = String::new();
                let mut escaped = false;
                let mut last_char = ' ';
                let mut left_flanking;
                let mut right_flanking;

                while let Some(c) = chars.next() {
                    left_flanking = last_char.is_ascii_whitespace();
                    right_flanking = chars.peek().unwrap_or(&' ').is_ascii_whitespace();

                    if c != '.' && c != '-' && !seq.is_empty() {
                        row = format!("{}{}", row, smart_sequence(seq));
                        seq = String::new();
                    }
                    match c {
                        '\r' | '\n' => {
                            row.push(c);
                            acc = format!("{}{}", acc, row);
                            row = String::new();
                            open_single = None;
                            open_double = None;
                            escaped = false;
                        }
                        '\'' => {
                            if escaped {
                                row.push(c);
                            } else if left_flanking {
                                open_single = Some((elem_index, acc.len() + row.len()));
                                row.push(c);
                            } else if open_single.is_some() && right_flanking {
                                let (ei, i) = open_single.unwrap();
                                if ei != elem_index {
                                    if let Some(Text(str)) = prev.get_mut(ei) {
                                        str.replace_range(i..i + 1, LSQUO);
                                    }
                                } else {
                                    let curr_i = i - acc.len();
                                    row.replace_range(curr_i..curr_i + 1, LSQUO);
                                }
                                row.push_str(RSQUO);
                                open_single = None;
                            } else {
                                row.push(c);
                            }
                            escaped = false;
                        }
                        '\"' => {
                            if escaped {
                                row.push(c)
                            } else if open_double.is_some() {
                                let (ei, i) = open_double.unwrap();
                                if ei != elem_index {
                                    if let Some(Text(str)) = prev.get_mut(ei) {
                                        str.replace_range(i..i + 1, LDQUO);
                                    }
                                } else {
                                    let curr_i = i - acc.len();
                                    row.replace_range(curr_i..curr_i + 1, LDQUO);
                                }
                                row.push_str(RDQUO);
                                open_double = None;
                            } else {
                                open_double = Some((elem_index, acc.len() + row.len()));
                                row.push(c);
                            }
                            escaped = false;
                        }
                        '.' | '-' => {
                            if escaped {
                                row.push(c);
                            } else if seq.is_empty() || seq.contains(c) {
                                seq.push(c);
                            } else {
                                row = format!("{}{}", row, smart_sequence(seq));
                                seq = String::from(c)
                            }
                        }
                        '\\' => {
                            row.push(c);
                            escaped = !escaped;
                        }
                        _ => {
                            row.push(c);
                            escaped = false;
                        }
                    }
                    last_char = c;
                }
                mem::swap(str, &mut format!("{}{}{}", acc, row, smart_sequence(seq)));
            }
            Some(Ast::Document(d)) => smart_punctuate(d),
            Some(Ast::Paragraph(p)) => smart_punctuate(p),
            Some(Ast::Tag(t)) => smart_punctuate(t),
            Some(Ast::Heading(h)) => smart_punctuate(h),
            _ => {}
        }
    }
}

fn smart_sequence(seq: String) -> String {
    return match seq.as_str() {
        "..." => ELLIP.to_string(),
        "--" => ENDASH.to_string(),
        "---" => EMDASH.to_string(),
        _ => seq,
    };
}
