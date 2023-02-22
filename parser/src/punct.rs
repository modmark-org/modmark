use crate::tag::CompoundAST;
use crate::Ast;
use crate::Ast::{Text};
use std::mem;

const ENDASH: &str = "\u{2013}";
const EMDASH: &str = "\u{2014}";
const ELLIP: &str = "\u{2026}";
const LSQUO: &str = "\u{2018}";
const RSQUO: &str = "\u{2019}";
const LDQUO: &str = "\u{201C}";
const RDQUO: &str = "\u{201D}";

pub fn smart_punctuate<T>(input: &mut T)
where
    T: CompoundAST,
{
    let elems: &mut Vec<Ast> = input.elements_mut();
    let mut open_single: Option<(usize, usize)> = None;
    let mut open_double: Option<(usize, usize)> = None;

    for elem_index in 0..elems.len() {
        let (prev, rest) = elems.as_mut_slice().split_at_mut(elem_index);
        let curr_elem = rest.get_mut(0);
        if let Some(Text(str)) = curr_elem {
            let mut chars = str.chars().peekable();
            let mut acc = String::new();
            let mut escaped = false;
            let mut last_escape = None;

            while let Some(c) = chars.next() {
                let last_char = acc.chars().rev().next().unwrap_or(' ');
                let left_flanking = last_char.is_whitespace();
                let right_flanking = chars.peek().unwrap_or(&' ').is_whitespace();
                let len = acc.len();

                if c != last_char {
                    try_smart_sequence(&mut acc, last_escape);
                }

                if c == '\n' || c == '\r' {
                    open_single = None;
                    open_double = None;
                }

                if c == '\"' && !escaped {
                    if let Some((ei, ci)) = open_double {
                        if ei == elem_index {
                            acc.replace_range(ci..ci + 1, LDQUO);
                        } else if let Some(Text(other)) = prev.get_mut(ei) {
                            other.replace_range(ci..ci + 1, LDQUO);
                        }
                        open_double = None;
                        acc.push_str(RDQUO);
                        continue;
                    } else {
                        open_double = Some((elem_index, len))
                    }
                }

                if c == '\'' && !escaped {
                    if left_flanking {
                        open_single = Some((elem_index, len))
                    } else if open_single.is_some() && right_flanking {
                        let (ei, ci) = open_single.unwrap();
                        if ei == elem_index {
                            acc.replace_range(ci..ci + 1, LSQUO);
                        } else if let Some(Text(other)) = prev.get_mut(ei) {
                            other.replace_range(ci..ci + 1, LSQUO);
                        }
                        open_single = None;
                        acc.push_str(RSQUO);
                        continue;
                    }
                }

                if escaped {
                    last_escape = Some(acc.len())
                }
                escaped = if c == '\\' { !escaped } else { false };
                acc.push(c)
            }
            try_smart_sequence(&mut acc, last_escape);
            mem::swap(str, &mut acc);
        } else {
            match curr_elem {
                Some(Ast::Document(d)) => smart_punctuate(d),
                Some(Ast::Paragraph(p)) => smart_punctuate(p),
                Some(Ast::Heading(h)) => smart_punctuate(h),
                Some(Ast::Tag(t)) => {
                    if t.recurse {
                        smart_punctuate(t)
                    }
                },
                _ => {}
            }
        }
    }
}

fn try_smart_sequence(str: &mut String, last_escape: Option<usize>) {
    if let Some(last_char) = str.chars().rev().next() {
        let mut seq = str
            .chars()
            .rev()
            .take_while(|ch| ch == &last_char)
            .collect::<String>();
        if let Some(i) = last_escape {
            seq.truncate(str.len()-i-1);
        }
        let range = str.len() - seq.len()..str.len();
        match seq.as_str() {
            "..." => str.replace_range(range, ELLIP),
            "--" => str.replace_range(range, ENDASH),
            "---" => str.replace_range(range, EMDASH),
            _ => {}
        }
    }
}
