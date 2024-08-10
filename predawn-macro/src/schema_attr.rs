use from_attr::{FlagOrValue, FromAttr};
use syn::Expr;

#[derive(FromAttr, Default)]
#[attribute(idents = [schema])]
pub(crate) struct SchemaAttr {
    pub(crate) rename: Option<String>,
    pub(crate) flatten: bool,
    pub(crate) default: FlagOrValue<Expr>,
}
