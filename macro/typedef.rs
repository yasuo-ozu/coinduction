use proc_macro2::TokenStream;
use proc_macro_error::*;
use std::collections::{HashMap, HashSet};
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

/// The `META` that a `#[coinduction]` module adds to the announcement of each type in
/// `announced`: `::coinduction::impls(<version>, [<predicates>])`, the predicates of the impls of
/// `working_traits` on the type, so that `#[coinduction]` modules elsewhere can see through bounds
/// on the type. Also returns the `TypeRef` impls of `marker` that the predicates use.
///
/// Impls mentioning relative paths need `marker`: without it, announcing a type with such impls is
/// an error.
pub(crate) fn type_impls(
    content: &[Item],
    working_traits: &HashSet<NoArgPath>,
    announced: &[Ident],
    marker: Option<&Path>,
    coinduction: &NoArgPath,
) -> (HashMap<Ident, TokenStream>, TokenStream) {
    let random_suffix = crate::get_random();
    let crate_version = env!("CARGO_PKG_VERSION");
    let mut unexported_types = HashSet::new();
    let (typeref_impl, type_impl_table) = content.iter().enumerate().fold(
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
                        && announced.contains(&segments[0].ident)
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

                        let mut leaker = Leaker::new();
                        leaker.intern_with(&renamed_generics, |visitor| {
                            #[allow(unused_imports)]
                            use syn::visit::Visit;
                            visitor.visit_path_arguments(&renamed_path_args);
                            visitor.visit_path(&renamed_trait_path);
                        }).unwrap_or_else(|e: NotInternableError| {
                            abort!(e.span(), "use absolute path");
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
                            let Some(marker) = marker else {
                                unexported_types.insert(type_ident);
                                return (typeref_impl, acc);
                            };
                            let mut visitor = referrer.clone().into_visitor(|_ty, ix| {
                                parse2(quote!(
                                        <#marker as #coinduction::TypeRef<#random_suffix, #ix0, #ix, #typeref_arg>>::Type
                                )).unwrap()
                            });
                            use syn::visit_mut::VisitMut;
                            visitor.visit_type_mut(&mut constraint.typ);
                            visitor.visit_path_mut(&mut constraint.trait_path);
                            typeref_impl = quote!(
                                #typeref_impl
                                #(for (ix, ty) in referrer.iter().enumerate()) {
                                    #[allow(non_camel_case_types)]
                                    impl #{renamed_generics.split_for_impl().0}
                                    #coinduction::TypeRef<#random_suffix, #ix0, #ix, #typeref_arg> for #marker {
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
    let metas = announced
        .iter()
        .map(|ty_ident| {
            if unexported_types.contains(ty_ident) {
                abort!(
                    ty_ident,
                    "the impls of `{}` use relative paths, so they cannot be announced", ty_ident;
                    help = "specify `marker = <path>` on the module, or use absolute paths"
                );
            }
            let impls = type_impl_table.get(ty_ident).map_or(&[][..], |v| &v[..]);
            let meta = quote! {
                ::coinduction::impls(
                    #crate_version,
                    [
                        #(for (generics, constraint, children) in impls), {
                            (
                                [ #(for p in &generics.params), {#p} ],
                                #constraint,
                                [ #(for c in children), { #c } ]
                            )
                        }
                    ]
                )
            };
            (ty_ident.clone(), meta)
        })
        .collect();
    (metas, typeref_impl)
}
