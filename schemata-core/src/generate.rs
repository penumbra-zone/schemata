#![allow(non_snake_case)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Ident, Type};

use crate::ir::{Children, Ir, Kind, Names, Node, Settings};

impl ToTokens for Ir {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let context = Context {
            depth: 0,
            param_count: 0,
        };
        self.root.to_tokens(&self.settings, context, tokens);
    }
}

#[derive(Clone, Copy)]
pub struct Context {
    depth: usize,
    param_count: usize,
}

impl Context {
    pub fn is_root(&self) -> bool {
        self.depth == 0
    }

    pub fn has_params(&self) -> bool {
        self.param_count > 0
    }
}

impl Node {
    fn to_tokens(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
        // These selectively only generate themselves when we're at the root
        self.root_schema_struct(settings, context, tokens);
        self.root_schema_fns(settings, context, tokens);

        self.per_module_structs(settings, context, tokens);
    }

    fn root_schema_struct(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
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
                        params: #Params {
                            __: ::core::marker::PhantomData,
                        },
                    }
                }

                /// Get the root path of this schema, as an owned path.
                pub fn owned_root() -> #OwnedPath {
                    #OwnedPath {
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

    fn root_schema_fns(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
        // Only generate the schema functions for the root of the schema
        if !context.is_root() {
            return;
        }

        let Names { Schema, Path, .. } = &settings.names;

        if let Ok(children) = &self.children {
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

    fn per_module_structs(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
        self.path_structs(settings, context, tokens);
        self.key_structs(settings, context, tokens);
        self.params_structs(settings, context, tokens);

        // Only generated when not a terminal leaf
        self.prefix_structs(settings, context, tokens);
        self.sub_prefix_structs(settings, context, tokens);
        self.sub_key_structs(settings, context, tokens);
    }

    fn path_structs(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
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

    fn prefix_structs(&self, settings: &Settings, _context: Context, tokens: &mut TokenStream) {
        // Don't generate these for leaves of the schema
        if self.is_leaf() {
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

    fn key_structs(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
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

    fn params_structs(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {
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
            let field = self
                .header
                .mod_name
                .as_ref()
                .expect("mod name is specified when params are present");

            quote! {
                #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::cmp::PartialEq, ::core::cmp::Eq)]
                pub struct #Params<'a> {
                    pub #field: &'a #ty,
                }

                #[derive(::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
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

        tokens.extend(match &self.header.kind {
            Kind::Var(ty) => one_param_structs(ty),
            Kind::Static { .. } => zero_param_structs(),
        });
    }

    fn sub_prefix_structs(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {}

    fn sub_key_structs(&self, settings: &Settings, context: Context, tokens: &mut TokenStream) {}
}
