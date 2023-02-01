use nom::Parser;

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
        Err(nom::Err::Error(e1)) => match p2.parse(i.clone()) {
            Err(nom::Err::Error(e2)) => Err(nom::Err::Error(e1.or(e2))),
            res => res.map(|(a, r2)| (a, (None, Some(r2)))),
        },
        res => res.map(|(a, r1)| (a, (Some(r1), None))),
    }
}

pub fn or2<I: Clone, O1, O2, E, F, G>(p1: F, p2: G) -> impl Parser<I, (Option<O1>, Option<O2>), E>
where
    E: nom::error::ParseError<I>,
    F: Parser<I, O1, E>,
    G: Parser<I, O2, E>,
{
    or_other(p1, p2)
}

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

fn flatten<A, B, C>(
    (fst, snd): (Option<A>, Option<(Option<B>, Option<C>)>),
) -> (Option<A>, Option<B>, Option<C>) {
    if let Some((fst_2, snd_2)) = snd {
        (fst, fst_2, snd_2)
    } else {
        (fst, None, None)
    }
}
