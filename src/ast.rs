use crate::lexer::Token;
use crate::parser::parse;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::span::{SimpleSpan, Span};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum RefOperator {
    OneToMany,
    OneToOne,
    ManyToMany,
}

#[derive(Debug)]
pub enum Index {
    Single(String),
    Composite(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum ColumnAttribute {
    Primary,
    Unique,
}

#[derive(Debug)]
pub struct Metadata<'a> {
    pub _src: String,
    pub _tokens: Vec<Rich<'a, Token<'a>>>,
}

#[derive(Debug)]
pub struct Schema {
    pub tables: Vec<TableDef>,
}

#[derive(Debug, Clone)]
pub struct TableDef {
    pub name: String,
    pub is_abstract: bool,
    pub extended_by: Option<String>,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub typ: String,
    pub attribute: Option<ColumnAttribute>,
    pub reference: Option<ReferenceDef>,
}

#[derive(Debug, Clone)]
pub struct IndexDef {}

#[derive(Debug, Clone)]
pub struct ReferenceDef {
    pub operator: RefOperator,
    pub table: String,
    pub column: String,
}

fn check(schema: &'_ Schema) -> Result<&'_ Schema, Vec<Rich<'_, Token<'_>>>> {
    let tables_by_name = collect_tables(&schema);
    if tables_by_name.is_err() {
        return Err(tables_by_name.err().unwrap());
    }

    Ok(schema)
}

// collect_tables also check for duplicated table declaration
fn collect_tables(
    schema: &Schema,
) -> Result<HashMap<&str, &TableDef>, Vec<Rich<'_, Token<'_>, SimpleSpan>>> {
    let mut map: HashMap<&str, &TableDef> = HashMap::new();
    for (_, table) in schema.tables.iter().enumerate() {
        if map.contains_key(&(*table.name)) {
            let span = Rich::custom(SimpleSpan::default(), "duplicated table");
            return Err(vec![span]);
        }
        map.insert(&table.name, table);
    }
    Ok(map)
}

#[test]
fn test_duplicated_tables() {
    let src = r"
        table foo {
            id: string
        }

        table foo {
            name: uuid4
        }
    ";
    let schema = parse(src).unwrap();
    match check(&schema) {
        Ok(_) => println!("ok"),
        Err(errs) => {
            for err in errs {
                Report::build(ReportKind::Error, ((), err.span().into_range()))
                    .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                    .with_code(10)
                    .with_message(err.to_string())
                    .with_label(
                        Label::new(((), err.span().into_range()))
                            .with_message(err.reason().to_string())
                            .with_color(Color::Red),
                    )
                    .finish()
                    .eprint(Source::from(src))
                    .unwrap();
            }
        }
    }
}
