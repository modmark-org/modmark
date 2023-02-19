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
    let mut open_elem_sg: Option<usize> = None;
    let mut open_elem_db: Option<usize> = None;

    for i in 0..elems.len() {
        let (prev, rest) = elems.as_mut_slice().split_at_mut(i);
        if let Some(e) = rest.get_mut(0) {
            match e {
                Text(str) => {
                    let mut res = String::new();
                    let mut curr = String::new();
                    let mut open_sg = open_elem_sg.is_some();
                    let mut open_db = open_elem_db.is_some();

                    if let Some(ii) = open_elem_sg {
                        if let Some(e) = prev.get_mut(ii) {
                            try_close_quote(&str, e, "\'", LSQUO)
                        }
                    }

                    if let Some(ii) = open_elem_db {
                        if let Some(e) = prev.get_mut(ii) {
                            try_close_quote(&str, e, "\"", LDQUO)
                        }
                    }

                    for c in str.chars() {
                        match c {
                            '\r' | '\n' => {
                                curr.push(c);
                                res = format!("{}{}", res, curr);
                                curr = String::new();
                                open_sg = false;
                                open_db = false;
                            }
                            '\"' => {
                                if open_db {
                                    curr = curr.replace("\"", LDQUO);
                                    curr.push_str(RDQUO)
                                } else {
                                    curr.push(c)
                                }
                                open_db = !open_db
                            }
                            '\'' => {
                                if open_sg {
                                    curr = curr.replace("\'", LSQUO);
                                    curr.push_str(RSQUO)
                                } else {
                                    curr.push(c)
                                }
                                open_sg = !open_sg
                            }
                            _ => {
                                curr.push(c);
                            }
                        }
                    }

                    res = format!("{}{}", res, curr)
                        .replace("...", ELLIP)
                        .replace("---", EMDASH)
                        .replace("--", ENDASH);

                    mem::swap(str, &mut res);

                    open_elem_sg = if open_sg { Some(i) } else { None };
                    open_elem_db = if open_db { Some(i) } else { None };
                }
                Ast::Document(d) => {
                    smart_punctuate(&mut d.elements);
                }
                Ast::Paragraph(p) => {
                    smart_punctuate(&mut p.elements);
                }
                Ast::Tag(t) => {
                    smart_punctuate(&mut t.elements);
                }
                Ast::Heading(h) => {
                    smart_punctuate(&mut h.elements);
                }
                _ => {}
            }
        }
    }
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
