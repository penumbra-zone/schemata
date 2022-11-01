use std::collections::HashMap;

use proc_macro2::Span;
use proc_macro_error::emit_error;
use quote::format_ident;
use syn::{
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    Ident, LitStr, Type,
};

use crate::syntax;

pub struct Ir {
    pub settings: Settings,
    pub root: Node,
}

pub struct Settings {
    pub names: Names,
    pub extensions: Extensions,
}

#[allow(non_snake_case)]
pub struct Names {
    pub Schema: Ident,
    pub Path: Ident,
    pub OwnedPath: Ident,
    pub Prefix: Ident,
    pub OwnedPrefix: Ident,
    pub Key: Ident,
    pub OwnedKey: Ident,
    pub Params: Ident,
    pub OwnedParams: Ident,
    pub SubPrefix: Ident,
    pub OwnedSubPrefix: Ident,
    pub SubKey: Ident,
    pub OwnedSubKey: Ident,
}

#[derive(Default, Clone)]
pub struct Extensions {
    pub clap: bool,
}

impl Default for Names {
    fn default() -> Self {
        Self {
            Schema: format_ident!("Schema"),
            Path: format_ident!("Path"),
            OwnedPath: format_ident!("OwnedPath"),
            Prefix: format_ident!("Prefix"),
            OwnedPrefix: format_ident!("OwnedPrefix"),
            Key: format_ident!("Key"),
            OwnedKey: format_ident!("OwnedKey"),
            Params: format_ident!("Params"),
            OwnedParams: format_ident!("OwnedParams"),
            SubPrefix: format_ident!("SubPrefix"),
            OwnedSubPrefix: format_ident!("OwnedSubPrefix"),
            SubKey: format_ident!("SubKey"),
            OwnedSubKey: format_ident!("OwnedSubKey"),
        }
    }
}

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
    Static {
        renamed: Option<LitStr>,
        param_count: usize,
    },
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

                if let Kind::Static {
                    renamed: Some(ref actual_name),
                    ..
                } = child.header.kind
                {
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

    pub fn is_leaf(&self) -> bool {
        match &self.children {
            Err(_) => true,
            Ok(children) => match children {
                Children::Leaf(_) => true,
                Children::Below(_) => false,
            },
        }
    }
}

impl From<syntax::Syntax> for Ir {
    fn from(syntax::Syntax { attrs, children }: syntax::Syntax) -> Self {
        // TODO: scrape settings from attrs
        let settings = Settings {
            names: Names::default(),
            extensions: Extensions::default(), // TODO: scrape extensions based on enabled features
        };
        let docs = vec![]; // TODO: scrape docs from attrs

        let children = Ok(Children::Below(
            children.into_iter().map(Node::from).collect(),
        ));
        let header = Header {
            docs,
            mod_name: None, // root node is only one not to have explicit mod name
            kind: Kind::Static {
                renamed: None,
                param_count: 0,
            }, // root node is always static and has no parameters
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
        let param_count = parameters.len();

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
            kind: Kind::Static {
                renamed,
                param_count,
            },
        };

        Node { header, children }
    }
}
