use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::remove_path_args;
use crate::solver::Constraint;
use crate::NoArgPath;

pub struct TypeDefArgs {
    pub paths: Punctuated<NoArgPath, Token![,]>,
    #[allow(dead_code)]
    pub coinduction: NoArgPath,
}

impl Parse for TypeDefArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let coinduction = crate::try_parse_coinduction_args(input)?;
        let paths: Punctuated<NoArgPath, Token![,]> =
            input.parse_terminated(NoArgPath::parse, Token![,])?;
        Ok(TypeDefArgs { paths, coinduction })
    }
}

pub fn typedef(module: ItemMod, args: TypeDefArgs) -> TokenStream {
    let random_suffix = crate::get_random();
    let crate_version = env!("CARGO_PKG_VERSION");
    let content = module
        .content
        .as_ref()
        .map(|c| &c.1)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let trait_paths: HashSet<_> = if args.paths.len() > 0 {
        args.paths.into_iter().collect()
    } else {
        content
            .iter()
            .filter_map(|item| match item {
                Item::Impl(item_impl) => Some(item_impl),
                _ => None,
            })
            .filter_map(|ItemImpl { trait_, .. }| trait_.as_ref().map(|t| remove_path_args(&t.1)))
            .collect()
    };
    let type_idents = content
        .iter()
        .filter_map(|item| match item {
            Item::Enum(ItemEnum { vis, ident, .. })
            | Item::Struct(ItemStruct { vis, ident, .. })
            | Item::Union(ItemUnion { vis, ident, .. }) => Some((ident.clone(), vis.clone())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let type_impl_table: HashMap<Ident, Vec<(Generics, Path, PathArguments)>> = content
        .iter()
        .fold(
            HashMap::new(),
            |mut acc: HashMap<Ident, Vec<(Generics, Path, PathArguments)>>, item| {
                if let Item::Impl(ItemImpl {
                    trait_: Some((_, trait_path, _)),
                    generics,
                    self_ty,
                    ..
                }) = item
                {
                    if let Type::Path(TypePath {
                        qself: None,
                        path:
                            Path {
                                leading_colon: None,
                                segments,
                            },
                    }) = self_ty.as_ref()
                    {
                        if segments.len() == 1
                            && trait_paths.contains(&remove_path_args(trait_path))
                        {
                            acc.entry(segments[0].ident.clone()).or_default().push((
                                generics.clone(),
                                trait_path.clone(),
                                segments[0].arguments.clone(),
                            ));
                        }
                    }
                }
                acc
            },
        )
        .into_iter()
        .collect();
    let macros = type_impl_table
        .iter()
        .fold(TokenStream::new(), |acc, (ty_ident, impls)| {
            let temporal_mac_name = syn::Ident::new(
                &format!("__{}_temporal_{}", &ty_ident, random_suffix),
                ty_ident.span(),
            );
            let vis = type_idents
                .get(&ty_ident)
                .cloned()
                .unwrap_or(Visibility::Public(Default::default()));
            quote! {
                #acc

                #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
                #[doc(hidden)]
                #[macro_export]
                macro_rules! #temporal_mac_name {
                    (#crate_version, None, [$($wt:tt)*], $coinduction:path, $($t:tt)*) => {
                        $coinduction::__next_step {
                            #crate_version, Typedef {
                                predicates: [
                                    #(for (generics, trait_path, self_args) in impls) {
                                        (
                                            [ #(for p in &generics.params), {#p} ],
                                            #ty_ident #self_args: #trait_path ,
                                            [
                                                #(for c in {
                                                    let mut constraints = Vec::new();
                                                    Constraint::map_generics(&mut generics.clone(), |c| {
                                                        constraints.push(c.clone());
                                                        vec![c]
                                                    });
                                                    constraints
                                                }), {
                                                    #c
                                                }
                                            ]
                                        )
                                    }
                                ]
                            }, [$($wt)*], $coinduction, $($tt)*
                        }
                    }
                }

                #[doc(hidden)]
                #[allow(unused_imports, unused_macros, dead_code)]
                #vis use #temporal_mac_name as #ty_ident;
            }
        });
    quote! {
        #(for attr in &module.attrs) { #attr }
        #{&module.vis} #{&module.unsafety} #{&module.mod_token} #{&module.ident} {
            #(for item in &content) {
                #item
            }
            #macros
        }
    }
}
