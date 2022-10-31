#![doc = include_str!("../README.md")]

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::parse2;

mod ir;
mod syntax;
mod tests;

#[doc(hidden)]
pub fn schema_internal(input: TokenStream) -> TokenStream {
    match parse2::<syntax::Input>(input) {
        Err(err) => abort!(err),
        Ok(input) => {
            let output = ir::Ir::from(input);
            quote!( #output )
        }
    }
}
