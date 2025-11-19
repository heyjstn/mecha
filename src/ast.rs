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

impl Schema {
    pub fn validate<'a>(&'a self) -> Result<(), Vec<Rich<'a, Token<'a>>>> {
        let table_map = self.collect_tables()?;

        self.validate_inheritance(&table_map)?;

        Ok(())
    }

    fn validate_inheritance<'a>(
        &self,
        table_map: &HashMap<&str, &TableDef>,
    ) -> Result<(), Vec<Rich<'a, Token<'a>, SimpleSpan>>> {
        for (_, table) in table_map {
            if let Some(parent_ident) = table.extended_by.as_ref() {
                let parent_name = parent_ident.name.as_str();

                match table_map.get(parent_name) {
                    Some(parent_table) => {
                        // errors referenced [`Table`] is existed but not abstract
                        if !parent_table.is_abstract {
                            let errs = vec![
                                Rich::custom(
                                    table.span,
                                    format!("table {} is referenced here", parent_name),
                                ),
                                Rich::custom(parent_table.span, "but it's not abstract"),
                            ];
                            return Err(errs);
                        }
                    }
                    None => {
                        // errors referenced [`Table`] is not existed
                        let errs = vec![Rich::custom(
                            parent_ident.span,
                            format!("table {} is not existed", parent_name),
                        )];
                        return Err(errs);
                    }
                }
            }
        }

        Ok(())
    }

    // collect_tables also check for duplicated table declaration
    fn collect_tables(
        &'_ self,
    ) -> Result<HashMap<&'_ str, &'_ TableDef>, Vec<Rich<'_, Token<'_>, SimpleSpan>>> {
        let mut map: HashMap<&str, &TableDef> = HashMap::new();

        for table in &self.tables {
            let table_name = table.id.name.as_str();

            if let Some(prev_table) = map.get(table_name) {
                // errors [`Table`] is redeclared
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
    match parse(src).unwrap().validate() {
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

#[test]
fn test_extend_non_existed_table() {
    let src = r"
        table foo extends bar {
            name: uuid4
        }
    ";
    match parse(src).unwrap().validate() {
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

#[test]
fn test_extend_non_abstract_table() {
    let src = r"
        table bar {
            id: string
        }

        table foo extends bar {
            name: uuid4
        }
    ";
    match parse(src).unwrap().validate() {
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
