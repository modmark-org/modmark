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
    let mut open_single: Option<usize> = None;
    let mut open_double: Option<usize> = None;

    for i in 0..elems.len() {
        let (prev, rest) = elems.as_mut_slice().split_at_mut(i);
        if let Some(e) = rest.get_mut(0) {
            match e {
                Text(str) => {
                    let mut chars = str.chars().peekable();
                    let mut acc = String::new();
                    let mut row = String::new();
                    let mut seq = String::new();
                    let mut escaped = false;
                    let mut right_flanking = true;

                    while let Some(c) = chars.next() {
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
                                let left_flanking =
                                    chars.peek().unwrap_or(&' ').is_ascii_whitespace();
                                if !escaped {
                                    if open_single.is_some() && left_flanking {
                                        let ii = open_single.unwrap();
                                        if ii != i {
                                            if let Some(ee) = prev.get_mut(ii) {
                                                close_prev_quote(ee, "\'", LSQUO);
                                            }
                                        } else {
                                            row = row.replace("\'", LSQUO);
                                        }
                                        row.push_str(RSQUO);
                                        open_single = None;
                                    } else {
                                        row.push(c);
                                        open_single = right_flanking.then_some(i);
                                    }
                                } else {
                                    row.push(c)
                                }
                                escaped = false;
                            }
                            '\"' => {
                                if !escaped {
                                    if let Some(ii) = open_double {
                                        if ii != i {
                                            if let Some(ee) = prev.get_mut(ii) {
                                                close_prev_quote(ee, "\"", LDQUO);
                                            }
                                        } else {
                                            row = row.replace("\"", LDQUO);
                                        }
                                        row.push_str(RDQUO);
                                        open_double = None;
                                    } else {
                                        row.push(c);
                                        open_double = Some(i);
                                    }
                                } else {
                                    row.push(c)
                                }
                                escaped = false;
                            }
                            '.' | '-' => {
                                if !escaped {
                                    if seq.is_empty() || seq.contains(c) {
                                        seq.push(c);
                                    } else {
                                        row = format!("{}{}", row, smart_sequence(seq));
                                        seq = String::from(c)
                                    }
                                } else {
                                    row.push(c)
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
                        right_flanking = c.is_ascii_whitespace();
                    }
                    mem::swap(str, &mut format!("{}{}{}", acc, row, smart_sequence(seq)));
                }
                Ast::Document(d) => smart_punctuate(d),
                Ast::Paragraph(p) => smart_punctuate(p),
                Ast::Tag(t) => smart_punctuate(t),
                Ast::Heading(h) => smart_punctuate(h),
                _ => {}
            }
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

fn close_prev_quote(e: &mut Ast, pat: &str, to: &str) {
    if let Text(other) = e {
        let mut index = 0;
        let mut escaped = false;

        for (i, c) in other.chars().enumerate() {
            if c == '\\' {
                escaped = !escaped;
            } else if pat.contains(c) && !escaped {
                index = i
            } else {
                escaped = false;
            }
        }
        other.replace_range(index..index + 1, to);
    }
}
