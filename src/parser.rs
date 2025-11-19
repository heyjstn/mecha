use crate::ast::{
    ColumnAttribute, ColumnDef, Ident, Index, RefOperator, ReferenceDef, Schema, TableDef,
};
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
        .map_with(|table, extra| Schema {
            name: "main.mecha".to_string(),
            tables: table,
            span: extra.span(),
        })
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
        .map_with(
            |((((is_abstract, ident), extends), columns), _indexes_opt), extra| TableDef {
                id: ident,
                is_abstract,
                columns,
                extended_by: extends,
                span: extra.span(),
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
        .map_with(|(((id, typ), attr_opt), ref_opt), extra| ColumnDef {
            id,
            typ,
            attribute: attr_opt,
            reference: ref_opt,
            span: extra.span(),
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
    .labelled("'primary' or 'unique'")
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
-> impl Parser<'tokens, I, Ident, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! {
        Token::Id(name) => name.to_string(),
    }
    .labelled("id")
    .map_with(|name, extra| Ident {
        name,
        span: extra.span(),
    })
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
        .map_with(|((operator, table), column), extra| ReferenceDef {
            operator,
            table,
            column,
            span: extra.span(),
        })
        .labelled("reference expression")
}

fn composite_index_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Index, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::LeftParen => () }
        .labelled("'('")
        .ignore_then(
            ident_string()
                .separated_by(select! { Token::Comma => () }.labelled("','"))
                .at_least(2)
                .collect::<Vec<_>>(),
        )
        .then_ignore(select! { Token::RightParen => () }.labelled("')'"))
        .map_with(|ids, extra| Index::Composite(ids, extra.span()))
        .labelled("multiple indexes")
}

fn index_item_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Index, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    composite_index_parser().or(ident_string()
        .map_with(|id, extra| Index::Single(id, extra.span()))
        .labelled("single index"))
}

fn index_section_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<Index>, extra::Err<Rich<'tokens, Token<'src>>>>
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
    select! { Token::Indexes => () }
        .labelled("'indexes'")
        .ignore_then(select! { Token::LeftBrace => () }.labelled("'{'"))
        .ignore_then(
            index_item_parser()
                .separated_by(select! { Token::Comma => () }.labelled("','"))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(select! { Token::RightBrace => () }.labelled("'}'"))
}

pub fn parse(src: &'_ str) -> Result<Schema, Vec<Rich<'_, Token<'_>>>> {
    let tokens = lexer::lex(src);
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
