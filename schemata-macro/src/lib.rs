#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[doc(hidden)]
#[proc_macro_error]
#[proc_macro]
pub fn schema_internal(input: TokenStream) -> TokenStream {
    schemata_core::schema_internal(input.into()).into()
}
