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
                    let mut acc = String::new();
                    let mut row = String::new();
                    let mut seq = String::new();
                    //let mut escaped = false;

                    if let Some(ii) = open_single {
                        if let Some(ee) = prev.get_mut(ii) {
                            try_close_quote(&str, ee, "\'", LSQUO)
                        }
                    }

                    if let Some(ii) = open_double {
                        if let Some(ee) = prev.get_mut(ii) {
                            try_close_quote(&str, ee, "\"", LDQUO)
                        }
                    }

                    for c in str.chars() {
                        if c != '.' && c != '-' {
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
                            }
                            '\'' => {
                                if open_single.is_some() {
                                    row = row.replace("\'", LSQUO);
                                    row.push_str(RSQUO);
                                    open_single = None;
                                } else {
                                    row.push(c);
                                    open_single = Some(i);
                                }
                            }
                            '\"' => {
                                if open_double.is_some() {
                                    row = row.replace("\"", LDQUO);
                                    row.push_str(RDQUO);
                                    open_double = None;
                                } else {
                                    row.push(c);
                                    open_double = Some(i);
                                }
                            }
                            /*
                            '\\' => {
                                escaped = !escaped;
                            }
                            */
                            '.' | '-' => {
                                if seq.is_empty() || seq.contains(c) {
                                    seq.push(c);
                                }
                            }
                            _ => {
                                row.push(c);
                            }
                        }
                    }

                    mem::swap(str, &mut format!("{}{}{}", acc, row, seq));
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

fn try_close_quote(str: &String, e: &mut Ast, pat: &str, to: &str) {
    if let Text(other) = e {
        if let Some(line) = str.lines().next() {
            if line.contains(pat) {
                let mut res = other
                    .chars()
                    .rev()
                    .collect::<String>()
                    .replacen(pat, to, 1)
                    .chars()
                    .rev()
                    .collect::<String>();
                mem::swap(other, &mut res)
            }
        }
    }
}
