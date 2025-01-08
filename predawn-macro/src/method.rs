use std::fmt;

use from_attr::FromIdent;
use syn::{parse_quote, Ident};

#[derive(Copy, Clone, FromIdent)]
pub(crate) enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Patch,
    Trace,
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
            Method::Get => parse_quote!(GET),
            Method::Post => parse_quote!(POST),
            Method::Put => parse_quote!(PUT),
            Method::Delete => parse_quote!(DELETE),
            Method::Head => parse_quote!(HEAD),
            Method::Options => parse_quote!(OPTIONS),
            Method::Connect => parse_quote!(CONNECT),
            Method::Patch => parse_quote!(PATCH),
            Method::Trace => parse_quote!(TRACE),
        }
    }
}

pub(crate) const ENUM_METHODS: [Method; 9] = [
    Method::Get,
    Method::Post,
    Method::Put,
    Method::Delete,
    Method::Head,
    Method::Options,
    Method::Connect,
    Method::Patch,
    Method::Trace,
];
