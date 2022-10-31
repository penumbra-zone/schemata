use std::{
    collections::{HashMap, HashSet},
    mem,
};

use proc_macro2::TokenStream;
use proc_macro_error::emit_error;
use quote::ToTokens;
use syn::{
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    Ident, LitStr, Type,
};

use crate::syntax;

pub struct Ir {
    settings: Settings,
    root: Node,
}

pub struct Settings {}

pub struct Node {
    pub header: Header,
    pub children: Result<Children, Duplicate>,
}

pub struct Duplicate;

pub struct Header {
    pub docs: Vec<LitStr>,
    pub mod_name: Option<Ident>,
    pub kind: Kind,
}

pub enum Kind {
    Var(Box<Type>),
    Static(Option<LitStr>),
}

pub enum Children {
    Below(Vec<Node>),
    Leaf(Box<Type>),
}

impl Node {
    fn prune_duplicates(&mut self) {
        let mut seen_mod_names = HashMap::new();
        let mut seen_actual_names = HashMap::new();

        if let Ok(Children::Below(ref mut children)) = self.children {
            for child in children {
                child.prune_duplicates();

                if let Some(ref mod_name) = child.header.mod_name {
                    if let Some(previous_mod_name) =
                        seen_mod_names.insert(mod_name.to_string(), mod_name.span())
                    {
                        child.children = Err(Duplicate);

                        // Put the old name back so we get a consistent hint span
                        seen_mod_names.insert(mod_name.to_string(), previous_mod_name);

                        // We don't emit an error about this issue; the Rust compiler will complain
                        // because the generated code will contain a duplicate module
                    }
                }

                if let Kind::Static(Some(ref actual_name)) = child.header.kind {
                    if let Some(previous_actual_name) =
                        seen_actual_names.insert(actual_name.value(), actual_name.span())
                    {
                        child.children = Err(Duplicate);

                        // Put the old name back so we get a consistent hint span
                        seen_actual_names.insert(actual_name.value(), previous_actual_name.span());

                        // The Rust compiler isn't going to check strings for equality, so we need
                        // to complain about this issue by making our own error
                        emit_error!(
                            actual_name,
                            "duplicate path segment: \"{}\"",
                            actual_name.value();
                            help = "use a different name for this path segment, or merge the two namespaces";
                        );
                    }
                } else if let Some(ref mod_name) = child.header.mod_name {
                    if let Some(previous_actual_name) =
                        seen_actual_names.insert(mod_name.to_string(), mod_name.span())
                    {
                        child.children = Err(Duplicate);

                        // Put the old name back so we get a consistent hint span
                        seen_actual_names.insert(mod_name.to_string(), previous_actual_name);

                        // The Rust compiler isn't going to check strings for equality, so we need
                        // to complain about this issue by making our own error
                        emit_error!(
                            mod_name,
                            "duplicate path segment: \"{}\"",
                            mod_name;
                            help = "use a different name for this path segment, or merge the two namespaces";
                        );
                    }
                }
            }
        }
    }
}

impl From<syntax::Input> for Ir {
    fn from(syntax::Input { attrs, children }: syntax::Input) -> Self {
        // TODO: scrape settings from attrs
        let settings = Settings {};
        let docs = vec![]; // TODO: scrape docs from attrs

        let children = Ok(Children::Below(
            children.into_iter().map(Node::from).collect(),
        ));
        let header = Header {
            docs,
            mod_name: None, // root node is only one not to have explicit mod name
            kind: Kind::Static(None), // root node is always static
        };
        let mut root = Node { header, children };

        // We don't generate code beneath duplicated modules, so detect and prune it now
        root.prune_duplicates();

        Ir { settings, root }
    }
}

impl From<syntax::Child> for Node {
    fn from(child: syntax::Child) -> Self {
        // Extract the segment (unprocessed) and the converted children (processed)
        let (segment, mut children) = match child {
            syntax::Child::Leaf { segment, ty, .. } => (segment, Ok(Children::Leaf(ty))),
            syntax::Child::Internal {
                segment, children, ..
            } => (
                segment,
                Ok(Children::Below(
                    children.into_iter().map(Node::from).collect(),
                )),
            ),
        };

        // Iterate through the parameters, if any, layering them as nodes on top of the children
        let mut parameters = segment
            .params
            .map(|p| p.params)
            .unwrap_or_else(Punctuated::new);

        while let Some(syntax::Parameter {
            attrs, name, ty, ..
        }) = parameters.pop().map(Pair::into_value)
        {
            let docs = vec![]; // TODO: scrape docs from attrs
            let header = Header {
                docs,
                mod_name: Some(*name),
                kind: Kind::Var(ty),
            };
            children = Ok(Children::Below(vec![Node { header, children }]));
        }

        // Top off the result with a named static node
        let docs = vec![]; // TODO: scrape docs from attrs
        let renamed = None; // TODO: scrape rename from attrs
        let header = Header {
            docs,
            mod_name: Some(segment.name),
            kind: Kind::Static(renamed),
        };

        Node { header, children }
    }
}

impl ToTokens for Ir {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.root.to_tokens(&self.settings, tokens);
    }
}

impl Node {
    fn to_tokens(&self, settings: &Settings, tokens: &mut TokenStream) {
        // TODO: generate all the code
    }
}
