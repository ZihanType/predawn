mod to_schema;
mod types;
mod util;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(ToSchema, attributes(schema))]
pub fn to_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    to_schema::generate(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
