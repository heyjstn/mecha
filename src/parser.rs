use crate::ast::{ColumnAttribute, ColumnDef, Index, RefOperator, ReferenceDef, Schema, TableDef};
use crate::lexer;
use crate::lexer::Token;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::prelude::{SimpleSpan, end};
use chumsky::{IterParser, Parser, extra, select};

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
        .map(|opt| opt.unwrap_or(false))
        .labelled("'abstract'");

    let extends_clause = select! { Token::Extends => () }
        .ignore_then(ident_string())
        .or_not()
        .labelled("'extends'");

    abstract_flag
        .then_ignore(select! { Token::Table => () })
        .then(ident_string())
        .then(extends_clause)
        .then_ignore(select! { Token::LeftBrace => () })
        .then(column_list_parser())
        .then(index_section_parser().or_not())
        .then_ignore(select! { Token::RightBrace => () })
        .map(
            |((((is_abstract, name), extends), columns), _indexes_opt)| TableDef {
                name,
                is_abstract,
                columns,
                extended_by: extends,
            },
        )
}

fn column_definition_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, ColumnDef, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    ident_string()
        .then_ignore(select! { Token::Colon => () }.labelled("':'"))
        .then(ident_string())
        .then(column_attribute_parser().or_not())
        .then(reference_parser().or_not())
        .map(|(((name, typ), attr_opt), ref_opt)| ColumnDef {
            name,
            typ,
            attribute: attr_opt,
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
    }.labelled("'primary' or 'unique'")
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
    .labelled("id")
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
    .labelled("ref operators given '==>, ==, <>'")
}

fn reference_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, ReferenceDef, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::LeftParen => () }
        .labelled("'('")
        .ignore_then(select! { Token::Ref => () }.labelled("'ref'"))
        .ignore_then(ref_operator_parser())
        .then(ident_string())
        .then_ignore(select! { Token::Dot => () }.labelled("'.'"))
        .then(ident_string())
        .then_ignore(select! { Token::RightParen => () }.labelled("')'"))
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
    select! { Token::LeftParen => () }.labelled("'('")
        .ignore_then(
            ident_string()
                .separated_by(select! { Token::Comma => () }.labelled("','"))
                .at_least(2)
                .collect::<Vec<_>>(),
        )
        .then_ignore(select! { Token::RightParen => () }.labelled("')'"))
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

pub fn parse(src: &'_ str) -> Result<Schema, Vec<Rich<'_, Token<'_>>>> {
    let tokens = lexer::tokenize(src);
    schema_parser().parse(tokens).into_result()
    // match schema_parser().parse(tokens).into_result() {
    //     Ok(schema) => println!("{:?}", schema),
    //     Err(errs) => {
    //         for err in errs {
    //             Report::build(ReportKind::Error, ((), err.span().into_range()))
    //                 .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
    //                 .with_code(3)
    //                 .with_message(err.to_string())
    //                 .with_label(
    //                     Label::new(((), err.span().into_range()))
    //                         .with_message(err.reason().to_string())
    //                         .with_color(Color::Red),
    //                 )
    //                 .finish()
    //                 .eprint(Source::from(src))
    //                 .unwrap();
    //         }
    //     }
    // }
}

#[test]
fn test_simple_table() {
    let schema: &str = r"
        table foo {
            id: string
        }
    ";
    match parse(schema) {
        Ok(schema) => assert!(schema.tables.len() > 0),
        Err(_) => panic!("test failed"),
    }
}

#[test]
fn test_abstract_table() {
    let schema: &str = r"
        abstract table foo {
            id: string
        }
    ";
    match parse(schema) {
        Ok(schema) => assert!(schema.tables.len() > 0),
        Err(_) => panic!("test failed"),
    }
}

#[test]
fn test_abstract_table_extends() {
    let schema: &str = r"
        abstract table foo extends bar {
            id: string
        }
    ";
    match parse(schema) {
        Ok(schema) => assert!(schema.tables.len() > 0),
        Err(_) => panic!("test failed"),
    }
}

#[test]
fn test_multiple_table() {
    let schema: &str = r"
        abstract table bar {
            created_at: timestamp,
            updated_at: timestamp
        }

        table foo extends bar {
            id: string primary
        }
    ";
    match parse(schema) {
        Ok(schema) => assert!(schema.tables.len() > 0),
        Err(_) => panic!("test failed"),
    }
}

#[test]
fn test_invalid_table_should_false() {
    let schema: &str = r"
        abstract table foo extends bar {
            created_at timestamp,
            updated_at: timestamp
        }
    ";
    match parse(schema) {
        Ok(schema) => assert!(schema.tables.len() > 0),
        Err(errs) => {
            for err in errs {
                Report::build(ReportKind::Error, ((), err.span().into_range()))
                    .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                    .with_code(11)
                    .with_message(err.to_string())
                    .with_label(
                        Label::new(((), err.span().into_range()))
                            .with_message(err.reason().to_string())
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint(Source::from(schema))
                    .unwrap();
            }
        }
    }
}
