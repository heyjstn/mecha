use crate::lexer::Token;
use crate::parser::parse;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::container::{Container, Seq};
use chumsky::error::Rich;
use chumsky::span::{SimpleSpan, Span};
use serde::{Serialize, Serializer};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize)]
pub enum RefOperator {
    OneToMany,
    OneToOne,
    ManyToMany,
}

#[derive(Debug, Serialize)]
pub enum Index {
    Single(Ident, SimpleSpan),
    Composite(Vec<Ident>, SimpleSpan),
}

#[derive(Debug, Clone, Serialize)]
pub enum ColumnAttribute {
    Primary,
    Unique,
}

#[derive(Debug, Serialize)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<TableDef>,

    #[serde(skip)]
    pub span: SimpleSpan,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableDef {
    pub id: Ident,
    pub is_abstract: bool,
    pub extended_by: Option<Ident>,
    pub columns: Vec<ColumnDef>,

    #[serde(skip)]
    pub span: SimpleSpan,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnDef {
    pub id: Ident,
    pub typ: Ident,
    pub attribute: Option<ColumnAttribute>,
    pub reference: Option<ReferenceDef>,

    #[serde(skip)]
    pub span: SimpleSpan,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexDef {}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceDef {
    pub operator: RefOperator,
    pub table: Ident,
    pub column: Ident,

    #[serde(skip)]
    pub span: SimpleSpan,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ident {
    pub name: String,

    #[serde(skip)]
    pub span: SimpleSpan,
}

pub enum SemanticErr {
    TableRedeclaration,
    NonExistentParent,
    NonAbstractParent,
    CyclicRef,
    ColumnRedeclaration,
}

type CheckResult<'a, T> = Result<T, Vec<Rich<'a, Token<'a>, SimpleSpan>>>;

impl Schema {
    pub fn check<'a>(&mut self) -> CheckResult<'a, ()> {
        let inheritance_context = self.build_inheritance_context()?;

        Ok(())
    }

    fn check_reference(&'_ self) -> CheckResult<'_, ()> {
        Ok(())
    }

    /// Collects tables and resolves all inherited columns into an owned HashMap
    /// Returns `HashMap<String, Vec<ColumnDef>>` instead of references to avoid borrowing conflicts
    /// Note: 'a is the lifetime of the Error, independent of the &self borrow
    fn build_inheritance_context<'a>(&self) -> CheckResult<'a, HashMap<String, Vec<ColumnDef>>> {
        let table_map = self.collect_tables()?;

        self.check_inheritance(&table_map)?;
        self.check_cyclic_inheritance(&table_map)?;

        let mut context: HashMap<String, Vec<ColumnDef>> = HashMap::new();

        for (name, &table) in &table_map {
            let mut inherited_columns: HashMap<&str, ColumnDef> = HashMap::new();
            let mut cur_table = table;

            for column in &table.columns {
                inherited_columns.insert(column.id.name.as_str(), column.clone());
            }

            loop {
                match cur_table.extended_by.as_ref() {
                    Some(parent_ident) => {
                        let parent_name = parent_ident.name.as_str();

                        match context.get(parent_name) {
                            Some(parent_columns) => {
                                for parent_column in parent_columns {
                                    let parent_column_name = parent_column.id.name.as_str();
                                    if inherited_columns.contains_key(parent_column_name) {
                                        let errs = vec![Rich::custom(
                                            parent_column.span,
                                            format!("column {} is redeclared", parent_column_name),
                                        )];
                                        return Err(errs);
                                    }
                                    inherited_columns
                                        .insert(parent_column_name, parent_column.clone());
                                }
                                break;
                            }
                            None => {
                                let parent_table = table_map.get(parent_name).unwrap();

                                for parent_column in &parent_table.columns {
                                    let parent_column_name = parent_column.id.name.as_str();
                                    if inherited_columns.contains_key(parent_column_name) {
                                        let errs = vec![Rich::custom(
                                            parent_column.span,
                                            format!("column {} is redeclared", parent_column_name),
                                        )];
                                        return Err(errs);
                                    }
                                    inherited_columns
                                        .insert(parent_column_name, parent_column.clone());
                                }

                                cur_table = parent_table;
                            }
                        }
                    }
                    None => break,
                }
            }

            let inherited_columns_vec = inherited_columns.values().cloned().collect();
            context.insert(name.to_string(), inherited_columns_vec);
        }

        Ok(context)
    }

    /// Check for [`SemanticErr::NonAbstractParent`], [`SemanticErr::NonExistentParent`]
    /// Uses explicit lifetimes 'a (Error) and 's (Self/Map) to allow decoupling
    fn check_inheritance<'a, 's>(
        &self,
        table_map: &HashMap<&'s str, &'s TableDef>,
    ) -> CheckResult<'a, ()> {
        for (_, table) in table_map {
            if let Some(parent_ident) = table.extended_by.as_ref() {
                let parent_name = parent_ident.name.as_str();

                match table_map.get(parent_name) {
                    Some(parent_table) => {
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

    /// Check for [`SemanticErr::CyclicRef`]
    fn check_cyclic_inheritance<'a, 's>(
        &self,
        table_map: &HashMap<&'s str, &'s TableDef>,
    ) -> CheckResult<'a, ()> {
        let mut checked: HashSet<&str> = HashSet::new();

        let mut sorted_tables: Vec<&TableDef> = table_map.values().copied().collect();
        sorted_tables.sort_by_key(|t| t.id.name.as_str());

        for table in sorted_tables {
            let start_name = table.id.name.as_str();

            if checked.contains(start_name) {
                continue;
            }

            let mut visited: HashSet<&str> = HashSet::new();
            let mut stack: Vec<&TableDef> = Vec::new();
            stack.push(table);

            while !stack.is_empty() {
                if let Some(cur_table) = stack.pop() {
                    visited.insert(cur_table.id.name.as_str());
                    match cur_table.extended_by.as_ref() {
                        Some(next_table_ident) => {
                            if checked.contains(next_table_ident.name.as_str()) {
                                // this path is fine as this parent is not a part of any cyclic component
                                continue;
                            }

                            let next_table_name = next_table_ident.name.as_str();

                            let next_table = table_map.get(next_table_name).unwrap(); // unwrap is fine because all referenced table is existed

                            if visited.contains(next_table_name) {
                                // oops, this table has been visited
                                let errs = vec![Rich::custom(
                                    next_table.span,
                                    format!("cyclic reference happens at {next_table_name}",),
                                )];
                                return Err(errs);
                            }

                            stack.push(next_table);
                        }
                        None => continue,
                    }
                }
            }

            // if reach this, all visited tables are fine
            checked.extend(visited);
        }

        Ok(())
    }

    /// Return a [`HashMap`] and also check for [`SemanticErr::TableRedeclaration`]
    /// Crucial: 's is the lifetime of the borrow of self, 'a is the lifetime of the Error
    fn collect_tables<'a, 's>(&'s self) -> CheckResult<'a, HashMap<&'s str, &'s TableDef>> {
        let mut map: HashMap<&str, &TableDef> = HashMap::new();

        for table in &self.tables {
            let table_name = table.id.name.as_str();

            if let Some(prev_table) = map.get(table_name) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid(src: &str) {
        let schema = &mut parse(src).unwrap();
        if let Err(errs) = schema.check() {
            print_report(src, errs);
            panic!("schema validation failed unexpectedly");
        }
        println!("{:?}", serde_json::to_string(schema).unwrap().as_str())
    }

    fn assert_invalid(src: &str) {
        let schema = &mut parse(src).unwrap();
        if let Err(errs) = schema.check() {
            print_report(src, errs);
        } else {
            panic!("schema validation succeeded but should have failed");
        }
    }

    // todo: migrate this to an error diagnosis mod
    fn print_report(src: &str, errs: Vec<Rich<Token, SimpleSpan>>) {
        for err in errs {
            Report::build(ReportKind::Error, ("main.mecha", err.span().into_range())) // todo: pass filename into this instead of hardcode
                .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                .with_message(err.to_string())
                .with_label(
                    Label::new(("main.mecha", err.span().into_range()))
                        .with_message(err.reason().to_string())
                        .with_color(Color::Red),
                )
                .finish()
                .print(("main.mecha", Source::from(src)))
                .unwrap();
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
        assert_invalid(src);
    }

    #[test]
    fn test_extend_non_existed_table() {
        let src = r"
            table foo extends bar {
                name: uuid4
            }
        ";
        assert_invalid(src);
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
        assert_invalid(src);
    }

    #[test]
    fn test_normal_ref_tables() {
        let src = r"
            abstract table bar {
                id: string
            }

            table foo extends bar {
                name: uuid4
            }
        ";
        assert_valid(src);
    }

    #[test]
    fn test_cyclic_ref_tables() {
        let src = r"
            abstract table bar extends foo {
                id: string
            }

            abstract table foo extends bar {
                name: uuid4
            }
        ";
        assert_invalid(src);
    }

    #[test]
    fn test_cyclic_ref_tables_2() {
        let src = r"
            abstract table bar extends foo {
                id: string
            }

            abstract table hey extends bar {
                time: timestampz
            }

            abstract table foo extends hey {
                name: uuid4
            }
        ";
        assert_invalid(src);
    }

    #[test]
    fn test_redeclared_column_ref_tables() {
        let src = r"
            abstract table bar {
                id: string
            }

            table foo extends bar {
                id: timestampz
            }
        ";
        assert_invalid(src);
    }
}
