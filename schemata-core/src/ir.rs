use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Ident, LitStr, Type};

use crate::syntax;

pub struct Node {
    pub header: Header,
    pub children: Children,
}

pub struct Header {
    pub docs: Vec<LitStr>,
    pub mod_name: Option<Ident>,
    pub kind: Kind,
}

pub enum Kind {
    Var(Type),
    Static(Option<LitStr>),
}

pub enum Children {
    Below(Vec<Node>),
    Leaf(Type),
}

impl From<syntax::Input> for Node {
    fn from(syntax::Input { attrs, children }: syntax::Input) -> Self {
        todo!()
    }
}

impl ToTokens for Node {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        todo!()
    }
}
