use from_attr::FlagOrValue;
use syn::{
    Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue, Token, punctuated::Punctuated,
    spanned::Spanned,
};

pub struct SerdeAttr {
    pub rename: Option<String>,
    pub flatten: bool,
    pub default: FlagOrValue<String>,
}

impl SerdeAttr {
    pub fn new(attrs: &[Attribute]) -> Self {
        let mut rename = None;
        let mut flatten = false;
        let mut default = FlagOrValue::None;

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }

            let Ok(meta_list) = attr.meta.require_list() else {
                continue;
            };

            let Ok(nested) =
                meta_list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            else {
                continue;
            };

            for meta in nested {
                let (path, ident) = {
                    let path = meta.path();

                    let Some(ident) = path.get_ident() else {
                        continue;
                    };

                    (path.span(), ident.to_string())
                };

                match ident.as_str() {
                    "rename" => match &meta {
                        Meta::NameValue(MetaNameValue {
                            value:
                                Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit_str),
                                    ..
                                }),
                            ..
                        }) => {
                            rename = Some(lit_str.value());
                        }
                        _ => continue,
                    },
                    "flatten" => match &meta {
                        Meta::Path(_) => {
                            flatten = true;
                        }
                        _ => continue,
                    },
                    "default" => match &meta {
                        Meta::Path(_) => {
                            default = FlagOrValue::Flag { path };
                        }
                        Meta::NameValue(MetaNameValue {
                            value:
                                Expr::Lit(ExprLit {
                                    lit: Lit::Str(lit_str),
                                    ..
                                }),
                            ..
                        }) => {
                            default = FlagOrValue::Value {
                                path,
                                value: lit_str.value(),
                            };
                        }
                        _ => {
                            continue;
                        }
                    },
                    _ => continue,
                }
            }
        }

        Self {
            rename,
            flatten,
            default,
        }
    }
}
