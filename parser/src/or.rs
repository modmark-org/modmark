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
