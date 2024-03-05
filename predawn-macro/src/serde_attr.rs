use syn::{punctuated::Punctuated, Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue, Token};

pub(crate) struct SerdeAttr {
    pub(crate) rename: Option<String>,
}

impl SerdeAttr {
    pub(crate) fn new(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut rename = None;

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }

            let Ok(meta_list) = attr.meta.require_list() else {
                continue;
            };

            let nested =
                meta_list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            for meta in nested {
                match meta {
                    Meta::NameValue(MetaNameValue { path, value, .. }) => {
                        if path.is_ident("rename") {
                            match value {
                                Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit_str),
                                    ..
                                }) => {
                                    rename = Some(lit_str.value());
                                }
                                _ => continue,
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        Ok(Self { rename })
    }
}
