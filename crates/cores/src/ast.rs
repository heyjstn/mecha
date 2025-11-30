use chumsky::span::SimpleSpan;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum RefOperator {
    OneToMany,
    OneToOne,
    ManyToMany,
}

#[derive(Debug, Clone, Serialize)]
pub enum Index {
    Single(Ident, #[serde(skip)] SimpleSpan),
    Composite(Vec<Ident>, #[serde(skip)] SimpleSpan),
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
    pub indexes: Option<Vec<Index>>,

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
