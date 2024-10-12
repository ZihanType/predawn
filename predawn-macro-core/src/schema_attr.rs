use from_attr::{FlagOrValue, FromAttr};
use syn::Expr;

#[derive(FromAttr, Default)]
#[attribute(idents = [schema])]
pub struct SchemaAttr {
    pub rename: Option<String>,
    pub flatten: bool,
    pub default: FlagOrValue<Expr>,
}
