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
//! implemented from 2 up to 26. The functions are generated using macros, and comments are attached
//! to the macros themselves, as well as code leftovers from before using macros.
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
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use nom::Parser;
use paste::paste;

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

// Here is the previous code written by hand without macros, that was used before we added macros
// to generate such methods. It is left here as a remainder why it was a tedious, long and error-
// prone process, and hopefully serves as a reason to use the macros :)
/*
pub fn or7_old<I: Clone, O1, O2, O3, O4, O5, O6, O7, E, F, G, H, J, K, L, M>(
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
*/

type Optt<A, B> = Option<(Option<A>, Option<B>)>;

/// This function flattens a tuple of the form `(Optional<A>, Optional<(Optional<B>, Optional<C>)>)`
/// to a tuple of the form `(Optional<A>, Optional<B>, Optional<C>)` which is much easier to handle.
/// If the second tuple doesn't exist, both `Option<B>` and `Option<C>` will be `None`.
fn flatten<A, B, C>((fst, snd): (Option<A>, Optt<B, C>)) -> (Option<A>, Option<B>, Option<C>) {
    if let Some((fst_2, snd_2)) = snd {
        (fst, fst_2, snd_2)
    } else {
        (fst, None, None)
    }
}

/// This macro generates an or function combining multiple parsers, testing them one by one until
/// one of them succeeds, and returns a tuple of optionals where at most one is `Some`, containing
/// the result of the successful parser. The macro takes a pattern, which will be appended to the
/// name, and a list of identifiers, as many as the number of parsers you want to support. The
/// identifiers should be unique and in lowercase. If you do `or_func!(3, a b c)`, this macro will
/// generate a function with the given signature:
/// ```rust,ignore
/// pub fn or3<Inp: Clone, AA, BB, CC, Err, A, B, C>(
///     a: A,
///     b: B,
///     c: C,
/// ) -> impl Parser<Inp, (Option<AA>, Option<BB>, Option<CC>), Err>
/// where
///     Err: nom::error::ParseError<Inp>,
///     A: Parser<Inp, AA, Err>,
///     B: Parser<Inp, BB, Err>,
///     C: Parser<Inp, CC, Err>;
/// ```
macro_rules! or_func {
    ($name:pat, $($vars:ident)*) => {
        paste! {
            pub fn [<or $name>] <Inp: Clone, $([<$vars:upper $vars:upper>],)* Err, $([<$vars:upper>],)*>(
                $($vars : [<$vars:upper>],)*
            ) -> impl Parser<Inp, ($(Option<[<$vars:upper $vars:upper>]>, )*), Err>
            where
            Err: nom::error::ParseError<Inp>,
            $([<$vars:upper>]: Parser<Inp, [<$vars:upper $vars:upper>], Err>,)*
            {
                or_body!($($vars)*)
            }
        }
    }
}

/// This macro generates the body to be placed inside the generated function, which is first one or
/// multiple calls to `or_other` (see `or_call!`) followed by a one or multiple calls to `flatten`
/// (see `flatten!`)
macro_rules! or_body {
    ($($args:ident)*) => {{
        let call = or_call!($($args)*);
        flatten!(call, $($args)*)
    }};
}

/// This macro generates a nested `or_other` call corresponding to the given identifiers. If the
/// identifiers a, b, c, d and e are given, it will generate the code
/// `or_other(a, or_other(b, or_other(c, or_other(d, e))))`. The identifiers should be given
/// in lowercase.
macro_rules! or_call {
    ($first:ident $second:ident) => {
        or_other($first, $second)
    };
    ($first:ident $($others:ident)*) => {
        or_other($first, or_call!($($others)*))
    };
}

/// This macro generates the "flatten" part of the function. If you have chained 5 parsers, the
/// (unflattened) result out of the types A, B, C, D and E will be:
/// ```rust,ignore
/// let result: (Option<A>, Option<(Option<B>, Option<(Option<C>, Option<(Option<D>,  Option<E>)>)>)>)
/// ```
/// To flatten this, we need to do it in three steps. The first call to `flatten` removes one level
/// on B, C, D and E. B is then at the root level. The second call to `flatten` removes one level
/// on C, D and E. C is then at the root level, and D and E one level in. The third and final call
/// to flatten moves D and E to the root level as well. If it was implemented by chaining `.map` on
/// parsers, it would be equivalent to:
/// ```rust,ignore
///     or_other(p1, or_other(p2, or_other(p3, or_other(p4, p5))))
///         .map(flatten)
///         .map(|(a, b, c)| {
///             let (b, c, d) = flatten((b, c));
///             (a, b, c, d)
///         })
///         .map(|(a, b, c, d)| {
///             let (c, d, e) = flatten((c, d));
///             (a, b, c, d, e)
///         })
/// ```
/// For each additional parser, one more call to `flatten` is needed. The `flatten!` macro takes
/// an variable holding the parser and a list of multiple unique identifiers, as many as the amount
/// of parsers combined in the given parser, and generates code equivalent to the code above which
/// flattens the parser. The identifiers should be in lowercase.
macro_rules! flatten {
    ($e:expr, $first:ident $second:ident) => {$e};
    ($e:expr, $first:ident $second:ident $third:ident) => {
        $e.map(flatten)
    };
    ($e:expr, $x: ident $($args: ident)*) => {{
        let flat = flatten!($e, $($args)*);
        flat.map(|($($args,)*)| {
            let (next_last!($($args)*), last!($($args)*), new) = flatten((next_last!($($args)*), last!($($args)*)));
            ($($args, )* new)
        })
    }}
}

/// This macro gives the last identifier out of all the identifiers supplied
macro_rules! last {
    ($last:ident) => {
        $last
    };
    ($a: ident $($rest:ident)+) => {last!($($rest)*)}
}

/// This macro gives the next to last identifier out of all the identifiers supplied
macro_rules! next_last {
    ($next_last:ident $last:ident) => {
        $next_last
    };
    ($a: ident $($rest:ident)+) => {next_last!($($rest)*)}
}

or_func!(2, a b);
or_func!(3, a b c);
or_func!(4, a b c d);
or_func!(5, a b c d e);
or_func!(6, a b c d e f);
or_func!(7, a b c d e f g);
or_func!(8, a b c d e f g h);
or_func!(9, a b c d e f g h i);
or_func!(10, a b c d e f g h i j);
or_func!(11, a b c d e f g h i j k);
or_func!(12, a b c d e f g h i j k l);
or_func!(13, a b c d e f g h i j k l m);
or_func!(14, a b c d e f g h i j k l m n);
or_func!(15, a b c d e f g h i j k l m n o);
or_func!(16, a b c d e f g h i j k l m n o p);
or_func!(17, a b c d e f g h i j k l m n o p q);
or_func!(18, a b c d e f g h i j k l m n o p q r);
or_func!(19, a b c d e f g h i j k l m n o p q r s);
or_func!(20, a b c d e f g h i j k l m n o p q r s t);
or_func!(21, a b c d e f g h i j k l m n o p q r s t u);
or_func!(22, a b c d e f g h i j k l m n o p q r s t u v);
or_func!(23, a b c d e f g h i j k l m n o p q r s t u v w);
or_func!(24, a b c d e f g h i j k l m n o p q r s t u v w x);
or_func!(25, a b c d e f g h i j k l m n o p q r s t u v w x y);
or_func!(26, a b c d e f g h i j k l m n o p q r s t u v w x y z);
