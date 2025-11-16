extern crate core;

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::input::Stream;
use chumsky::input::{Input, ValueInput};
use chumsky::prelude::{end, SimpleSpan};
use chumsky::{extra, IterParser};
use chumsky::{select, Parser};
use logos::Logos;

#[derive(Logos, Clone, PartialEq)]
enum Token<'a> {
    Err,

    #[token("abstract")]
    Abstract,
    #[token("table")]
    Table,
    #[token("extends")]
    Extends,

    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*")]
    Id(&'a str),

    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,

    #[token("primary")]
    Primary,
    #[token("unique")]
    Unique,

    #[token("ref")]
    Ref,
    #[token("=>")]
    RefOneToMany,
    #[token("==")]
    RefOneToOne,
    #[token("<>")]
    RefManyToMany,

    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    #[token("indexes")]
    Indexes,

    #[regex(r"[ \t\f\n]+", logos::skip)]
    Whitespace,
}

// AST structs
#[derive(Debug)]
enum RefOperator {
    OneToMany,
    OneToOne,
    ManyToMany,
}

#[derive(Debug)]
pub enum Index {
    Single(String),
    Composite(Vec<String>),
}

#[derive(Debug)]
pub enum ColumnAttribute {
    Primary,
    Unique,
}

#[derive(Debug)]
struct Schema {
    tables: Vec<TableDef>,
}

#[derive(Debug)]
struct TableDef {
    name: String,
    is_abstract: bool,
    extended_by: String,
    columns: Vec<ColumnDef>,
}

#[derive(Debug)]
struct ColumnDef {
    name: String,
    typ: String,
    index: IndexDef,
    reference: Option<ReferenceDef>,
}

#[derive(Debug)]
struct IndexDef {}

#[derive(Debug)]
struct ReferenceDef {
    operator: RefOperator,
    table: String,
    column: String,
}

fn schema_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Schema, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    table_parser()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map(|table| Schema { tables: table })
}

fn table_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, TableDef, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    let abstract_flag = select! { Token::Abstract => true }
        .or_not()
        .map(|opt| opt.unwrap_or(false));

    let extends_clause = select! { Token::Extends => () }
        .ignore_then(ident_string())
        .or_not();

    abstract_flag
        .then_ignore(select! { Token::Table => () })
        .then(ident_string()) // table name
        .then(extends_clause) // Option<String>
        .then_ignore(select! { Token::LeftBrace => () })
        .then(column_list_parser()) // Vec<ColumnDefinition>
        .then(index_section_parser().or_not()) // Option<Vec<Index>>
        .then_ignore(select! { Token::RightBrace => () })
        .map(
            |((((is_abstract, name), extends), columns), indexes_opt)| TableDef {
                name,
                is_abstract,
                columns,
                extended_by: "".to_string(),
            },
        )
}

fn column_definition_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, ColumnDef, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    ident_string()
        .then_ignore(select! { Token::Colon => () })
        .then(ident_string())
        .then(column_attribute_parser().or_not())
        .then(reference_parser().or_not())
        .map(|(((name, typ), attr_opt), ref_opt)| ColumnDef {
            name,
            typ,
            // attribute: attr_opt,
            index: IndexDef {},
            reference: ref_opt,
        })
}

fn column_attribute_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, ColumnAttribute, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! {
        Token::Primary => ColumnAttribute::Primary,
        Token::Unique  => ColumnAttribute::Unique,
    }
}

fn column_list_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, Vec<ColumnDef>, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    column_definition_parser()
        .separated_by(select! { Token::Comma => () })
        .at_least(1)
        .collect::<Vec<_>>()
}

fn ident_string<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, String, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! {
        Token::Id(name) => name.to_string(),
    }
}

fn ref_operator_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, RefOperator, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! {
        Token::RefOneToMany   => RefOperator::OneToMany,
        Token::RefOneToOne    => RefOperator::OneToOne,
        Token::RefManyToMany  => RefOperator::ManyToMany,
    }
}

fn reference_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, ReferenceDef, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::LeftParen => () }
        .ignore_then(select! { Token::Ref => () })
        .ignore_then(ref_operator_parser())
        .then(ident_string()) // table name
        .then_ignore(select! { Token::Dot => () })
        .then(ident_string()) // column name
        .then_ignore(select! { Token::RightParen => () })
        .map(|((operator, table), column)| ReferenceDef {
            operator,
            table,
            column,
        })
}

fn composite_index_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, Index, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::LeftParen => () }
        .ignore_then(
            ident_string()
                .separated_by(select! { Token::Comma => () })
                .at_least(2)
                .collect::<Vec<_>>(),
        )
        .then_ignore(select! { Token::RightParen => () })
        .map(Index::Composite)
}

fn index_item_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, Index, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    composite_index_parser().or(ident_string().map(Index::Single))
}

fn index_section_parser<'tokens, 'src: 'tokens, I>()
    -> impl Parser<'tokens, I, Vec<Index>, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::Indexes => () }
        .ignore_then(select! { Token::LeftBrace => () })
        .ignore_then(
            index_item_parser()
                .separated_by(select! { Token::Comma => () })
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(select! { Token::RightBrace => () })
}

const SRC: &str = r"
    abstract tablex foo extends bar {
        id: string
    }
";

fn main() {
    let token_iter = Token::lexer(SRC).spanned().map(|(tok, span)| match tok {
        Ok(tok) => {
            let simple_span: SimpleSpan = span.into();
            (tok, simple_span)
        }
        Err(()) => (Token::Err, span.into()),
    });

    let token_stream =
        Stream::from_iter(token_iter).map((0..SRC.len()).into(), |(t, s): (_, _)| (t, s));

    match schema_parser().parse(token_stream).into_result() {
        Ok(schema) => println!("{:?}", schema),
        Err(errs) => {
            for err in errs {
                Report::build(ReportKind::Error, ((), err.span().into_range()))
                    .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                    .with_code(3)
                    // .with_message(err.to_string())
                    .with_label(
                        Label::new(((), err.span().into_range()))
                            // .with_message(err.reason().to_string())
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint(Source::from(SRC))
                    .unwrap();
            }
        }
    }
}
