//! Or combinator for the Nom library.
//!
//! There is a default or combinator (with method chaining) in the Nom library already, but it
//! does only work outputs of the same type, such as `char('a').or(char('b'))`, which then
//! results in a parser returning that same type. This works fine if all parsers handles strings,
//! however if you want to chain a parser which maps to one type with another parser which maps
//! to a different type, you will need to make an enum just for that case.
//!
//! This file contains functions to combine parsers with .or semantics of different types. The
//! first parser is attempted to match the input, and if it doesn't match, the second parser is
//! attempted at matching. The result of the parser is a tuple of `Option`s, one for each parser,
//! with all of them being `None` but the one matching being `Some` holding the result. If none
//! of the parsers match, the or combinator fails.
//!
//! The functions are called `or{n}` with `n` being the amount of parsers to be combined, currently
//! implemented from 2 up to 7.
//!
//! Example
//! ```rust,ignore
//! let num_parser = map(alphanumeric1, |n| usize::from_str(n).unwrap()); // parse int as usize
//! let abc_parser = is_a("abc"); // parse long string containing "a"s, "b"s and "c"s
//! let parser = or2(num_parser, abc_parser);
//! ```
//! |Input|Output            |
//! |-----|------------------|
//! |"123"|(Some(123), None) |
//! |"aba"|(None, Some("aba")|
//! |"eed"|Error             |
use nom::Parser;

#[allow(dead_code)]
pub fn or_other<I: Clone, O1, O2, E, F, G>(
    mut p1: F,
    mut p2: G,
) -> impl Parser<I, (Option<O1>, Option<O2>), E>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
{
    move |i: I| match p1.parse(i.clone()) {
        Err(nom::Err::Error(e1)) => match p2.parse(i) {
            Err(nom::Err::Error(e2)) => Err(nom::Err::Error(e1.or(e2))),
            res => res.map(|(a, r2)| (a, (None, Some(r2)))),
        },
        res => res.map(|(a, r1)| (a, (Some(r1), None))),
    }
}

#[allow(dead_code)]
pub fn or2<I: Clone, O1, O2, E, F, G>(p1: F, p2: G) -> impl Parser<I, (Option<O1>, Option<O2>), E>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
{
    or_other(p1, p2)
}

#[allow(dead_code)]
pub fn or3<I: Clone, O1, O2, O3, E, F, G, H>(
    p1: F,
    p2: G,
    p3: H,
) -> impl Parser<I, (Option<O1>, Option<O2>, Option<O3>), E>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
    H: Parser<I, O3, E>,
{
    or_other(p1, or_other(p2, p3)).map(flatten)
}

#[allow(dead_code)]
pub fn or4<I: Clone, O1, O2, O3, O4, E, F, G, H, J>(
    p1: F,
    p2: G,
    p3: H,
    p4: J,
) -> impl Parser<I, (Option<O1>, Option<O2>, Option<O3>, Option<O4>), E>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
    H: Parser<I, O3, E>,
    J: Parser<I, O4, E>,
{
    or_other(p1, or_other(p2, or_other(p3, p4)))
        .map(flatten)
        .map(|(a, b, c)| {
            let (b, c, d) = flatten((b, c));
            (a, b, c, d)
        })
}

#[allow(dead_code)]
pub fn or5<I: Clone, O1, O2, O3, O4, O5, E, F, G, H, J, K>(
    p1: F,
    p2: G,
    p3: H,
    p4: J,
    p5: K,
) -> impl Parser<I, (Option<O1>, Option<O2>, Option<O3>, Option<O4>, Option<O5>), E>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
    H: Parser<I, O3, E>,
    J: Parser<I, O4, E>,
    K: Parser<I, O5, E>,
{
    or_other(p1, or_other(p2, or_other(p3, or_other(p4, p5))))
        .map(flatten)
        .map(|(a, b, c)| {
            let (b, c, d) = flatten((b, c));
            (a, b, c, d)
        })
        .map(|(a, b, c, d)| {
            let (c, d, e) = flatten((c, d));
            (a, b, c, d, e)
        })
}

#[allow(dead_code)]
pub fn or6<I: Clone, O1, O2, O3, O4, O5, O6, E, F, G, H, J, K, L>(
    p1: F,
    p2: G,
    p3: H,
    p4: J,
    p5: K,
    p6: L,
) -> impl Parser<
    I,
    (
        Option<O1>,
        Option<O2>,
        Option<O3>,
        Option<O4>,
        Option<O5>,
        Option<O6>,
    ),
    E,
>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
    H: Parser<I, O3, E>,
    J: Parser<I, O4, E>,
    K: Parser<I, O5, E>,
    L: Parser<I, O6, E>,
{
    or_other(
        p1,
        or_other(p2, or_other(p3, or_other(p4, or_other(p5, p6)))),
    )
    .map(flatten)
    .map(|(a, b, c)| {
        let (b, c, d) = flatten((b, c));
        (a, b, c, d)
    })
    .map(|(a, b, c, d)| {
        let (c, d, e) = flatten((c, d));
        (a, b, c, d, e)
    })
    .map(|(a, b, c, d, e)| {
        let (d, e, f) = flatten((d, e));
        (a, b, c, d, e, f)
    })
}

#[allow(dead_code)]
pub fn or7<I: Clone, O1, O2, O3, O4, O5, O6, O7, E, F, G, H, J, K, L, M>(
    p1: F,
    p2: G,
    p3: H,
    p4: J,
    p5: K,
    p6: L,
    p7: M,
) -> impl Parser<
    I,
    (
        Option<O1>,
        Option<O2>,
        Option<O3>,
        Option<O4>,
        Option<O5>,
        Option<O6>,
        Option<O7>,
    ),
    E,
>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
    H: Parser<I, O3, E>,
    J: Parser<I, O4, E>,
    K: Parser<I, O5, E>,
    L: Parser<I, O6, E>,
    M: Parser<I, O7, E>,
{
    or_other(
        p1,
        or_other(
            p2,
            or_other(p3, or_other(p4, or_other(p5, or_other(p6, p7)))),
        ),
    )
    .map(flatten)
    .map(|(a, b, c)| {
        let (b, c, d) = flatten((b, c));
        (a, b, c, d)
    })
    .map(|(a, b, c, d)| {
        let (c, d, e) = flatten((c, d));
        (a, b, c, d, e)
    })
    .map(|(a, b, c, d, e)| {
        let (d, e, f) = flatten((d, e));
        (a, b, c, d, e, f)
    })
    .map(|(a, b, c, d, e, f)| {
        let (e, f, g) = flatten((e, f));
        (a, b, c, d, e, f, g)
    })
}

type Optt<A, B> = Option<(Option<A>, Option<B>)>;

fn flatten<A, B, C>((fst, snd): (Option<A>, Optt<B, C>)) -> (Option<A>, Option<B>, Option<C>) {
    if let Some((fst_2, snd_2)) = snd {
        (fst, fst_2, snd_2)
    } else {
        (fst, None, None)
    }
}
