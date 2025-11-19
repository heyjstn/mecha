use crate::lexer::Token;
use crate::parser::parse;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Rich;
use chumsky::span::{SimpleSpan, Span};
use std::collections::HashMap;

pub type Spanned<T> = (T, SimpleSpan);

#[derive(Debug, Clone)]
pub enum RefOperator {
    OneToMany,
    OneToOne,
    ManyToMany,
}

#[derive(Debug)]
pub enum Index {
    Single(Ident, SimpleSpan),
    Composite(Vec<Ident>, SimpleSpan),
}

#[derive(Debug, Clone)]
pub enum ColumnAttribute {
    Primary,
    Unique,
}

#[derive(Debug)]
pub struct Schema {
    pub tables: Vec<TableDef>,
    pub span: SimpleSpan,
}

#[derive(Debug, Clone)]
pub struct TableDef {
    pub id: Ident,
    pub is_abstract: bool,
    pub extended_by: Option<Ident>,
    pub columns: Vec<ColumnDef>,
    pub span: SimpleSpan,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub id: Ident,
    pub typ: Ident,
    pub attribute: Option<ColumnAttribute>,
    pub reference: Option<ReferenceDef>,
    pub span: SimpleSpan,
}

#[derive(Debug, Clone)]
pub struct IndexDef {}

#[derive(Debug, Clone)]
pub struct ReferenceDef {
    pub operator: RefOperator,
    pub table: Ident,
    pub column: Ident,
    pub span: SimpleSpan,
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: SimpleSpan,
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
        let table_name_ref = &(*table.id.name);

        // errors propagate in case [Table] is redeclared
        if map.contains_key(table_name_ref) {
            let prev_table = map.get(table_name_ref).unwrap();
            let errs = vec![
                Rich::custom(
                    prev_table.id.span,
                    format!("table {} is declared here", prev_table.id.name),
                ),
                Rich::custom(table.id.span, "but redeclared here"),
            ];
            return Err(errs);
        }

        map.insert(&table.id.name, table);
    }
    Ok(map)
}

#[test]
fn test_duplicated_tables() {
    let src = r"
        table foo {
            id: string
        }

        table foo extends bar {
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
