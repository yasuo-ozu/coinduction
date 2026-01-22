use proc_macro2::{Span, TokenStream};
use proc_macro_error::*;
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::*;
use template_quote::quote;
use type_leak::{Leaker, NotInternableError};

use crate::remove_path_args;
use crate::solver::Constraint;
use crate::NoArgPath;

/// Renames generic parameters to avoid collisions between different impl blocks.
/// Returns the rename map and modifies the inputs in place.
fn randomize_generic_params(
    generics: &mut Generics,
    self_ty: &mut Type,
    trait_path: &mut Path,
    ix0: usize,
    random_suffix: u64,
) -> HashMap<Ident, Ident> {
    // Build rename map and rename param declarations
    let rename_map: HashMap<Ident, Ident> = generics
        .params
        .iter_mut()
        .filter_map(|param| {
            match param {
                GenericParam::Type(tp) => {
                    let old = tp.ident.clone();
                    let new = Ident::new(
                        &format!("__{}_{}_{}", old, ix0, random_suffix),
                        old.span(),
                    );
                    tp.ident = new.clone();
                    Some((old, new))
                }
                GenericParam::Lifetime(lp) => {
                    let old = lp.lifetime.ident.clone();
                    let new = Ident::new(
                        &format!("__{}_{}_{}", old, ix0, random_suffix),
                        old.span(),
                    );
                    lp.lifetime.ident = new.clone();
                    Some((old, new))
                }
                GenericParam::Const(cp) => {
                    let old = cp.ident.clone();
                    let new = Ident::new(
                        &format!("__{}_{}_{}", old, ix0, random_suffix),
                        old.span(),
                    );
                    cp.ident = new.clone();
                    Some((old, new))
                }
            }
        })
        .collect();

    // Visitor for renaming generic parameter usages
    struct ParamRenamer<'a>(&'a HashMap<Ident, Ident>);

    impl syn::visit_mut::VisitMut for ParamRenamer<'_> {
        fn visit_type_mut(&mut self, ty: &mut Type) {
            syn::visit_mut::visit_type_mut(self, ty);
            if let Type::Path(TypePath { qself: None, path }) = ty {
                if path.leading_colon.is_none()
                    && path.segments.len() == 1
                    && matches!(path.segments[0].arguments, PathArguments::None)
                {
                    if let Some(new) = self.0.get(&path.segments[0].ident) {
                        path.segments[0].ident = new.clone();
                    }
                }
            }
        }

        fn visit_lifetime_mut(&mut self, lt: &mut Lifetime) {
            if let Some(new) = self.0.get(&lt.ident) {
                lt.ident = new.clone();
            }
            syn::visit_mut::visit_lifetime_mut(self, lt);
        }

        fn visit_expr_mut(&mut self, expr: &mut Expr) {
            syn::visit_mut::visit_expr_mut(self, expr);
            if let Expr::Path(ExprPath { qself: None, path, .. }) = expr {
                if path.leading_colon.is_none()
                    && path.segments.len() == 1
                    && matches!(path.segments[0].arguments, PathArguments::None)
                {
                    if let Some(new) = self.0.get(&path.segments[0].ident) {
                        path.segments[0].ident = new.clone();
                    }
                }
            }
        }
    }

    // Apply renaming
    use syn::visit_mut::VisitMut;
    let mut renamer = ParamRenamer(&rename_map);

    // Rename in generics bounds and where clause
    for param in generics.params.iter_mut() {
        match param {
            GenericParam::Type(tp) => {
                for bound in tp.bounds.iter_mut() {
                    renamer.visit_type_param_bound_mut(bound);
                }
            }
            GenericParam::Lifetime(lp) => {
                for bound in lp.bounds.iter_mut() {
                    renamer.visit_lifetime_mut(bound);
                }
            }
            GenericParam::Const(cp) => {
                renamer.visit_type_mut(&mut cp.ty);
            }
        }
    }
    if let Some(wc) = generics.where_clause.as_mut() {
        renamer.visit_where_clause_mut(wc);
    }

    // Rename in self_ty and trait_path
    renamer.visit_type_mut(self_ty);
    renamer.visit_path_mut(trait_path);

    rename_map
}

mod kw {
    syn::custom_keyword!(marker);
    syn::custom_keyword!(coinduction);
}

pub struct TypeDefArgs {
    pub paths: Punctuated<NoArgPath, Token![,]>,
    #[allow(dead_code)]
    pub coinduction: NoArgPath,
    #[allow(dead_code)]
    pub marker: Option<syn::Path>,
}

impl Parse for TypeDefArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let coinduction = crate::try_parse_coinduction_args(input)?;
        let mut marker = None;
        let mut paths = Punctuated::new();

        while !input.is_empty() {
            // Check for marker = ...
            if input.peek(kw::marker) && input.peek2(Token![=]) {
                input.parse::<kw::marker>()?;
                input.parse::<Token![=]>()?;
                marker = Some(input.parse()?);

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }

            // Parse path
            let path: NoArgPath = input.parse()?;
            paths.push(path);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(TypeDefArgs {
            paths,
            coinduction,
            marker,
        })
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
    let working_traits: HashSet<_> = if args.paths.len() > 0 {
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
    let (_typeref_impl, type_impl_table) = content.iter().enumerate().fold(
        Default::default(),
        |(mut typeref_impl, mut acc): (TokenStream, HashMap<Ident, Vec<(Generics, Constraint, Vec<Constraint>)>>),
         (ix0, item)| {
            if let Item::Impl(ItemImpl {
                trait_: Some((_, trait_path, _)),
                generics,
                self_ty,
                ..
            }) = item
            {
                match self_ty.as_ref() {
                    Type::Path(TypePath {
                        qself: None,
                        path:
                            Path {
                                leading_colon: None,
                                segments,
                            },
                    }) if segments.len() == 1
                        && working_traits.contains(&remove_path_args(trait_path)) =>
                    {
                        // Extract type identifier before modifications
                        let type_ident = segments[0].ident.clone();

                        // Clone and randomize generic parameter names to avoid collisions
                        let mut renamed_generics = generics.clone();
                        let mut renamed_self_ty = self_ty.as_ref().clone();
                        let mut renamed_trait_path = trait_path.clone();
                        let _rename_map = randomize_generic_params(
                            &mut renamed_generics,
                            &mut renamed_self_ty,
                            &mut renamed_trait_path,
                            ix0,
                            random_suffix,
                        );

                        // Extract renamed path arguments for the leaker
                        let renamed_path_args = if let Type::Path(TypePath { path, .. }) = &renamed_self_ty {
                            path.segments.last().unwrap().arguments.clone()
                        } else {
                            PathArguments::None
                        };

                        let mut leaker = Leaker::with_generics(renamed_generics.clone());
                        leaker.intern_with(|visitor| {
                            #[allow(unused_imports)]
                            use syn::visit::Visit;
                            visitor.visit_path_arguments(&renamed_path_args);
                            visitor.visit_path(&renamed_trait_path);
                        }).unwrap_or_else(|NotInternableError(span)| {
                            abort!(span, "use absolute path");
                        });
                        leaker.reduce_roots();
                        let referrer = leaker.finish();
                        let typeref_arg = type_leak::encode_generics_params_to_ty(&renamed_generics.params);

                        let mut constraint = Constraint {
                            typ: renamed_self_ty.clone(),
                            trait_path: renamed_trait_path.clone()
                        };
                        let mut children = Vec::new();
                        Constraint::map_generics(&mut renamed_generics.clone(), |c| {
                            children.push(c.clone());
                            vec![c]
                        });

                        if !referrer.is_empty() {
                            let marker = args.marker.as_ref().unwrap_or_else(|| {
                                let first = referrer.iter().next().unwrap();
                                abort!(
                                    Span::call_site(), "specify 'marker = ' argument";
                                    hint = first.span() => "or make this path absolute";
                                );
                            });
                            let mut visitor = referrer.clone().into_visitor(|_ty, ix| {
                                parse2(quote!(
                                        <#marker as #{&args.coinduction}::TypeRef<#random_suffix, #ix0, #ix, #typeref_arg>>::Type
                                )).unwrap()
                            });
                            use syn::visit_mut::VisitMut;
                            visitor.visit_type_mut(&mut constraint.typ);
                            visitor.visit_path_mut(&mut constraint.trait_path);
                            typeref_impl = quote!(
                                #typeref_impl
                                #(for (ix, ty) in referrer.iter().enumerate()) {
                                    impl #{renamed_generics.split_for_impl().0}
                                    #{&args.coinduction}::TypeRef<#random_suffix, #ix0, #ix, #typeref_arg> for #marker {
                                        type Type = #ty;
                                    }
                                }
                            );
                            for child in children.iter_mut() {
                                visitor.visit_type_mut(&mut child.typ);
                                visitor.visit_path_mut(&mut child.trait_path);
                            }
                        }

                        acc.entry(type_ident).or_default().push((
                            renamed_generics.clone(),
                            constraint,
                            children
                        ));
                    }
                    _ => (),
                }
            }
            (quote!(#typeref_impl), acc)
        },
    );
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
                    (#crate_version, None, [$($wt:tt)*], {$($coinduction:tt)+}, $($t:tt)*) => {
                        $($coinduction)+::__next_step! {
                            #crate_version, Typedef {
                                predicates: [
                                    #(for (generics, constraint, children) in impls), {
                                        (
                                            [ #(for p in &generics.params), {#p} ],
                                            #constraint,
                                            [ #(for c in children), { #c } ]
                                        )
                                    }
                                ]
                            }, [$($wt)*], {$($coinduction)+}, $($t)*
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
