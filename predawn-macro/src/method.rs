use std::fmt;

use from_attr::FromIdent;
use syn::{parse_quote, Ident};

#[derive(Copy, Clone, FromIdent)]
pub(crate) enum Method {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Patch,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ident = self.as_uppercase_ident();
        write!(f, "{}", ident)
    }
}

impl Method {
    pub(crate) fn as_uppercase_ident(&self) -> Ident {
        match self {
            Method::Options => parse_quote!(OPTIONS),
            Method::Get => parse_quote!(GET),
            Method::Post => parse_quote!(POST),
            Method::Put => parse_quote!(PUT),
            Method::Delete => parse_quote!(DELETE),
            Method::Head => parse_quote!(HEAD),
            Method::Trace => parse_quote!(TRACE),
            Method::Patch => parse_quote!(PATCH),
        }
    }
}

pub(crate) const ENUM_METHODS: [Method; 8] = [
    Method::Options,
    Method::Get,
    Method::Post,
    Method::Put,
    Method::Delete,
    Method::Head,
    Method::Trace,
    Method::Patch,
];
