use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, digit1, multispace0},
    combinator::{opt, recognize},
    error::ParseError,
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, pair},
    Err, IResult,
};

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Term<'a> {
    name: &'a str,
    ports: Option<Vec<Term<'a>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Def<'a> {
    Agent(Vec<(&'a str, usize)>),
    Rule(Term<'a>, Term<'a>),
    Eq(Term<'a>, Term<'a>),
}

pub fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)
}

fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub fn term(input: &str) -> IResult<&str, Term> {
    let (input, name) = identifier(input)?;
    let (input, ports) = opt(delimited(
        tag("("),
        separated_list0(ws(tag(",")), term),
        tag(")"),
    ))(input)?;
    Ok((input, Term { name, ports }))
}

pub fn agent_def_atom(input: &str) -> IResult<&str, (&str, usize)> {
    let (input, name) = identifier(input)?;
    let (input, _) = ws(tag(":"))(input)?;
    let (input, arity) = digit1(input)?;
    let arity = arity.parse();
    match arity {
        Ok(arity) => Ok((input, (name, arity))),
        Err(_) => Err(Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        ))),
    }
}

pub fn agent_def(input: &str) -> IResult<&str, Def> {
    let (input, _) = ws(tag("#agent"))(input)?;
    let (input, agent_defs) = separated_list1(ws(tag(",")), agent_def_atom)(input)?;
    Ok((input, Def::Agent(agent_defs)))
}

pub fn rule_def(input: &str) -> IResult<&str, Def> {
    let (input, _) = ws(tag("#rule"))(input)?;
    let (input, term_l) = term(input)?;
    let (input, _) = ws(tag("><"))(input)?;
    let (input, term_r) = term(input)?;
    Ok((input, Def::Rule(term_l, term_r)))
}

pub fn eq_def(input: &str) -> IResult<&str, Def> {
    let (input, term_l) = ws(term)(input)?;
    let (input, _) = ws(tag("="))(input)?;
    let (input, term_r) = term(input)?;
    Ok((input, Def::Eq(term_l, term_r)))
}

pub fn def(input: &str) -> IResult<&str, Vec<Def>> {
    many1(alt((agent_def, rule_def, eq_def)))(input)
}

#[cfg(test)]
mod tests {
    use crate::parser::{Def, Term};

    use super::def;

    #[test]
    fn parser_multiline_statement_test() {
        let result = def("#agent Add:2, Z: 1 , E :0\n#agent A:2\r\nA(c)=A(r)");
        assert_eq!(
            result,
            Ok((
                "",
                vec![
                    Def::Agent(vec![("Add", 2), ("Z", 1), ("E", 0)]),
                    Def::Agent(vec![("A", 2)]),
                    Def::Eq(
                        Term {
                            name: "A",
                            ports: Some(vec![Term {
                                name: "c",
                                ports: None,
                            }]),
                        },
                        Term {
                            name: "A",
                            ports: Some(vec![Term {
                                name: "r",
                                ports: None,
                            }]),
                        },
                    )
                ]
            ))
        );
    }
}
