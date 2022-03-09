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
pub enum Statement<'a> {
    AgentDef(Vec<(&'a str, usize)>),
    RuleDef(Term<'a>, Term<'a>),
    EqDef(Term<'a>, Term<'a>),
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

pub fn agent_def(input: &str) -> IResult<&str, Statement> {
    let (input, _) = ws(tag("#agent"))(input)?;
    let (input, agent_defs) = separated_list1(ws(tag(",")), agent_def_atom)(input)?;
    Ok((input, Statement::AgentDef(agent_defs)))
}

pub fn rule_def(input: &str) -> IResult<&str, Statement> {
    let (input, _) = ws(tag("#rule"))(input)?;
    let (input, term_l) = term(input)?;
    let (input, _) = ws(tag("><"))(input)?;
    let (input, term_r) = term(input)?;
    Ok((input, Statement::RuleDef(term_l, term_r)))
}

pub fn eq_def(input: &str) -> IResult<&str, Statement> {
    let (input, term_l) = ws(term)(input)?;
    let (input, _) = ws(tag("="))(input)?;
    let (input, term_r) = term(input)?;
    Ok((input, Statement::EqDef(term_l, term_r)))
}

pub fn statement(input: &str) -> IResult<&str, Vec<Statement>> {
    many1(alt((agent_def, rule_def, eq_def)))(input)
}

#[cfg(test)]
mod tests {
    use crate::syntax::{Statement, Term};

    use super::statement;

    #[test]
    fn parser_multiline_statement_test() {
        let result = statement("#agent Add:2, Z: 1 , E :0\n#agent A:2\r\nA(c)=A(r)");
        assert_eq!(
            result,
            Ok((
                "",
                vec![
                    Statement::AgentDef(vec![("Add", 2), ("Z", 1), ("E", 0)]),
                    Statement::AgentDef(vec![("A", 2)]),
                    Statement::EqDef(
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
