#![allow(non_snake_case)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Ident, Type};

use crate::ir::{Children, Ir, Kind, Names, Node, Settings};

impl ToTokens for Ir {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let context = Context {
            depth: 0,
            remaining_param_count: 0,
        };
        NodeInContextWithSettings {
            node: &self.root,
            context,
            settings: &self.settings,
        }
        .to_tokens(tokens);
    }
}

#[derive(Clone, Copy)]
pub struct Context {
    depth: usize,
    remaining_param_count: usize,
}

impl Context {
    pub fn is_root(&self) -> bool {
        self.depth == 0
    }

    pub fn has_remaining_params(&self) -> bool {
        self.remaining_param_count > 0
    }
}

pub struct NodeInContextWithSettings<'a> {
    pub node: &'a Node,
    pub context: Context,
    pub settings: &'a Settings,
}

impl ToTokens for NodeInContextWithSettings<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // These selectively only generate themselves when we're at the root
        self.root_schema_struct(tokens);
        self.root_schema_fns(tokens);

        // Generate all the structs for this module
        self.per_module_structs(tokens);

        // Generate all the child modules
        self.child_modules(tokens);
    }
}

impl NodeInContextWithSettings<'_> {
    fn root_schema_struct(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        // Only generate the schema struct for the root of the schema
        if !context.is_root() {
            return;
        }

        let Names {
            Schema,
            Path,
            OwnedPath,
            Params,
            OwnedParams,
            ..
        } = &settings.names;

        tokens.extend(quote! {
            #[derive(
                ::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq,
            )]
            pub struct #Schema;

            impl #Schema {
                /// Get the root path of this schema.
                pub fn root<'a>() -> #Path<'a> {
                    #Path {
                        parent: #Schema,
                        params: #Params {
                            __: ::core::marker::PhantomData,
                        },
                    }
                }

                /// Get the root path of this schema, as an owned path.
                pub fn owned_root() -> #OwnedPath {
                    #OwnedPath {
                        parent: #Schema,
                        params: #OwnedParams {},
                    }
                }
            }

            impl From<&#Schema> for #Schema {
                fn from(_: &#Schema) -> Self {
                    #Schema
                }
            }
        });
    }

    fn root_schema_fns(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        // Only generate the schema functions for the root of the schema
        if !context.is_root() {
            return;
        }

        let Names { Schema, Path, .. } = &settings.names;

        if let Ok(children) = &node.children {
            match children {
                Children::Below(children) => {
                    for child in children {
                        let name = child
                            .header
                            .mod_name
                            .as_ref()
                            .expect("child module always has a name");

                        tokens.extend(quote! {
                            pub fn #name<'a>() -> #name::#Path<'a> {
                                #Schema::root().#name()
                            }
                        })
                    }
                }
                Children::Leaf(_) => unreachable!("root of schema can't be a bare type"),
            }
        } else {
            unreachable!("root of schema can't be a duplicate");
        }
    }

    fn per_module_structs(&self, tokens: &mut TokenStream) {
        self.path_structs(tokens);
        self.key_structs(tokens);
        self.params_structs(tokens);

        // Only generated when not a terminal leaf
        self.prefix_structs(tokens);
        self.sub_prefix_structs(tokens);
        self.sub_key_structs(tokens);
    }

    fn path_structs(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        let Names {
            Schema,
            Path,
            OwnedPath,
            Params,
            OwnedParams,
            ..
        } = &settings.names;

        let (parent, owned_parent) = if context.is_root() {
            (quote!(#Schema), quote!(#Schema))
        } else {
            (quote!(super::#Path), quote!(super::#OwnedPath))
        };

        tokens.extend(quote! {
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            pub struct #Path<'a> {
                params: #Params<'a>,
                parent: #parent,
            }

            #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            pub struct #OwnedPath {
                params: #OwnedParams,
                parent: #owned_parent,
            }
        });
    }

    fn prefix_structs(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        // Don't generate these for leaves of the schema
        if node.is_leaf() {
            return;
        }

        let Names {
            Params,
            OwnedParams,
            Prefix,
            OwnedPrefix,
            SubPrefix,
            OwnedSubPrefix,
            ..
        } = &settings.names;

        tokens.extend(quote! {
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            pub struct #Prefix<'a> {
                params: #Params<'a>,
                child: ::core::option::Option<#SubPrefix<'a>>,
            }

            #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            pub struct #OwnedPrefix {
                params: #OwnedParams,
                child: ::core::option::Option<#OwnedSubPrefix>,
            }
        });
    }

    fn key_structs(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        let Names {
            Params,
            OwnedParams,
            Key,
            OwnedKey,
            SubKey,
            OwnedSubKey,
            ..
        } = &settings.names;

        let (derive_clap_args, group_skip, clap_flatten, clap_subcommand) =
            if settings.extensions.clap {
                (
                    quote!(derive(::clap::Args)),
                    quote!(#[group(skip)]),
                    quote!(#[clap(flatten)]),
                    quote!(#[clap(subcommand)]),
                )
            } else {
                (quote!(), quote!(), quote!(), quote!())
            };

        tokens.extend(quote! {
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            pub struct #Key<'a> {
                params: #Params<'a>,
                child: #SubKey<'a>,
            }

            #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            #derive_clap_args
            #group_skip
            pub struct #OwnedKey {
                #clap_flatten
                params: #OwnedParams,
                #clap_subcommand
                child: #OwnedSubKey,
            }
        });
    }

    fn params_structs(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        let Names {
            Params,
            OwnedParams,
            ..
        } = &settings.names;

        let (derive_clap_args, group_skip, clap_long) = if settings.extensions.clap {
            (
                quote!(derive(::clap::Args)),
                quote!(#[group(skip)]),
                quote!(#[clap(long)]),
            )
        } else {
            (quote!(), quote!(), quote!())
        };

        // If there is a parameter at this level, put it in `Params`
        let one_param_structs = |ty| {
            let field = node
                .header
                .mod_name
                .as_ref()
                .expect("mod name is specified when params are present");

            quote! {
                #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
                #[allow(non_snake_case)]
                pub struct #Params<'a> {
                    pub #field: &'a #ty,
                }

                #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
                #[allow(non_snake_case)]
                #derive_clap_args
                #group_skip
                pub struct #OwnedParams {
                    #clap_long
                    pub #field: #ty,
                }
            }
        };

        // If there are no parameters at this level, make `Params` empty (except for the lifetime)
        let zero_param_structs = || {
            quote! {
                #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
                pub struct #Params<'a> {
                    __: ::core::marker::PhantomData<&'a ()>,
                }

                #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
                pub struct #OwnedParams {}
            }
        };

        tokens.extend(match &node.header.kind {
            Kind::Var(ty) => one_param_structs(ty),
            Kind::Static { .. } => zero_param_structs(),
        });
    }

    fn sub_prefix_structs(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        let Names {
            Prefix,
            OwnedPrefix,
            SubPrefix,
            OwnedSubPrefix,
            ..
        } = &settings.names;

        let derive_clap_subcommand = if settings.extensions.clap {
            quote!(derive(::clap::Subcommand))
        } else {
            quote!()
        };

        let no_children = &vec![];
        let subkey: Vec<&Ident> = match &node.children {
            // If we're a leaf, we shouldn't generate subprefix structs at all
            Ok(Children::Leaf(_)) => return,
            Ok(Children::Below(children)) => children,
            Err(_) => no_children,
        }
        .iter()
        .map(|child| {
            child
                .header
                .mod_name
                .as_ref()
                .expect("child module has a module name")
        })
        .collect();

        tokens.extend(quote! {
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            enum #SubPrefix<'a> {
                #(#subkey(#subkey::#Prefix<'a>)),*
            }
        });

        tokens.extend(quote! {
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #derive_clap_subcommand
            #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            enum #OwnedSubPrefix {
                #(#subkey(#subkey::#OwnedPrefix)),*
            }
        });
    }

    fn sub_key_structs(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        let Names {
            Key,
            OwnedKey,
            SubKey,
            OwnedSubKey,
            ..
        } = &settings.names;

        let derive_clap_subcommand = if settings.extensions.clap {
            quote!(derive(::clap::Subcommand))
        } else {
            quote!()
        };

        let no_children = &vec![];
        let subkey: Vec<&Ident> = match &node.children {
            // If we're a leaf, we shouldn't generate subkey structs at all
            Ok(Children::Leaf(_)) => return,
            Ok(Children::Below(children)) => children,
            Err(_) => no_children,
        }
        .iter()
        .map(|child| {
            child
                .header
                .mod_name
                .as_ref()
                .expect("child module has a module name")
        })
        .collect();

        tokens.extend(quote! {
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            enum #SubKey<'a> {
                #(#subkey(#subkey::#Key<'a>)),*
            }
        });

        tokens.extend(quote! {
            #[allow(non_camel_case_types)]
            #[non_exhaustive]
            #derive_clap_subcommand
            #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
            enum #OwnedSubKey {
                #(#subkey(#subkey::#OwnedKey)),*
            }
        });
    }

    fn child_modules(&self, tokens: &mut TokenStream) {
        let Self {
            node,
            context,
            settings,
        } = self;

        if let Ok(Children::Below(children)) = &node.children {
            // Figure out the context for the children
            let context = Context {
                depth: context.depth + 1,
                remaining_param_count: match node.header.kind {
                    Kind::Var(_) => context.remaining_param_count - 1,
                    Kind::Static { param_count, .. } => param_count,
                },
            };

            // Generate code for all the children
            for child in children {
                let mod_name = child
                    .header
                    .mod_name
                    .as_ref()
                    .expect("child has module name");

                let child = NodeInContextWithSettings {
                    node: child,
                    context,
                    settings,
                };

                tokens.extend(quote! {
                    mod #mod_name {
                        #child
                    }
                })
            }
        }
    }
}
