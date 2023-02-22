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

pub fn smart_punctuate<T>(input: &mut T)
where
    T: CompoundAST,
{
    let elems: &mut Vec<Ast> = input.elements_mut();
    let mut open_single: Option<(usize, usize)> = None;
    let mut open_double: Option<(usize, usize)> = None;
    let mut paired_single: Option<((usize, usize), (usize, usize))> = None;

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
                    if let Some(pair) = paired_single {
                        close_paired_single(prev, &mut acc, pair);
                    }
                }

                if c == '\"' && !escaped {
                    if let Some((ei, ci)) = open_double {
                        if ei == elem_index {
                            acc.replace_range(ci..ci + 1, LDQUO);
                        } else if let Some(Text(other)) = prev.get_mut(ei) {
                            other.replace_range(ci..ci + 1, LDQUO);
                        }
                        open_double = None;
                        paired_single = None;
                        acc.push_str(RDQUO);
                        escaped = false;
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
                        let (captured_str, str) = if let Some(Text(other)) = prev.get_mut(ei) {
                            (format!("{}{}", &other.as_str()[ci..], acc), other)
                        } else {
                            ((&acc.as_str()[ci..]).to_string(), &mut acc)
                        };

                        let smart_double_count = captured_str.matches(RDQUO).count()
                            + captured_str.matches(LDQUO).count();
                        let (_, di) = open_double.unwrap_or((0, 0));

                        if smart_double_count % 2 == 0 && ci >= di {
                            str.replace_range(ci..ci + 1, LSQUO);
                            acc.push_str(RSQUO);
                            open_single = None;
                            escaped = false;
                            continue;
                        } else {
                            open_single = None;
                            if smart_double_count % 2 == 0 {
                                paired_single = Some(((ei, ci), (elem_index, acc.len())));
                            }
                        }
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
                }
                _ => {}
            }
        }
    }

    if let Some(pair) = paired_single {
        let len = elems.len();
        let (prev, rest) = elems.as_mut_slice().split_at_mut(len - 1);
        if let Some(Text(str)) = rest.get_mut(0) {
            close_paired_single(prev, str, pair);
        }
    }
}

fn close_paired_single(prev: &mut [Ast], str: &mut String, pair: ((usize, usize), (usize, usize))) {
    let ((ei, ci), (ej, cj)) = pair;
    if ei == ej {
        str.replace_range(cj..cj + 1, RSQUO);
        str.replace_range(ci..ci + 1, LSQUO);
    } else if let Some(Text(other)) = prev.get_mut(ei) {
        str.replace_range(cj..cj + 1, RSQUO);
        other.replace_range(ci..ci + 1, LSQUO);
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
            seq.truncate(str.len() - i - 1);
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
