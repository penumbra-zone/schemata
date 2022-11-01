#![doc = include_str!("../README.md")]

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::parse2;

mod generate;
mod ir;
mod syntax;
mod tests;

#[doc(hidden)]
pub fn schema_internal(input: TokenStream) -> TokenStream {
    match parse2::<syntax::Syntax>(input) {
        Err(err) => abort!(err),
        Ok(syntax) => {
            let ir = ir::Ir::from(syntax);
            quote!( #ir )
        }
    }
}
