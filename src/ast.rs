#[derive(Debug)]
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

#[derive(Debug)]
pub enum ColumnAttribute {
    Primary,
    Unique,
}

#[derive(Debug)]
pub struct Schema {
    pub tables: Vec<TableDef>,
}

#[derive(Debug)]
pub struct TableDef {
    pub name: String,
    pub is_abstract: bool,
    pub extended_by: Option<String>,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug)]
pub struct ColumnDef {
    pub name: String,
    pub typ: String,
    pub attribute: Option<ColumnAttribute>,
    pub reference: Option<ReferenceDef>,
}

#[derive(Debug)]
pub struct IndexDef {}

#[derive(Debug)]
pub struct ReferenceDef {
    pub operator: RefOperator,
    pub table: String,
    pub column: String,
}
