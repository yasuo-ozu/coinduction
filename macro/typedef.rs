use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet};
use syn::*;
use template_quote::quote;

use crate::remove_path_args;
use crate::solver::Constraint;
use crate::NoArgPath;

/// Renames generic parameters to avoid collisions between different impl blocks.
/// Modifies the inputs in place.
fn randomize_generic_params(
    generics: &mut Generics,
    self_ty: &mut Type,
    trait_path: &mut Path,
    ix0: usize,
    random_suffix: u64,
) {
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

}

/// The `META` that a `#[coinduction]` module adds to the announcement of each type in
/// `announced`: `::coinduction::impls(<version>, [<predicates>])`, the predicates of the impls of
/// `working_traits` on the type, so that `#[coinduction]` modules elsewhere can see through bounds
/// on the type.
///
/// The impls are announced as written: announce sends `crate::` paths as `$crate::`, which
/// resolve everywhere, and relative paths are compared as written. Only a trait imported under
/// another name (in `aliases`) is named by its own name, as the modules using the type compare
/// traits by name.
pub(crate) fn type_impls(
    content: &[Item],
    working_traits: &HashSet<NoArgPath>,
    announced: &[Ident],
    aliases: &[(Ident, Ident)],
) -> HashMap<Ident, TokenStream> {
    let random_suffix = crate::get_random();
    let crate_version = env!("CARGO_PKG_VERSION");
    let mut type_impl_table: HashMap<Ident, Vec<(Generics, Constraint, Vec<Constraint>)>> =
        HashMap::new();
    for (ix0, item) in content.iter().enumerate() {
        let Item::Impl(ItemImpl {
            trait_: Some((_, trait_path, _)),
            generics,
            self_ty,
            ..
        }) = item
        else {
            continue;
        };
        let Type::Path(TypePath {
            qself: None,
            path:
                Path {
                    leading_colon: None,
                    segments,
                },
        }) = self_ty.as_ref()
        else {
            continue;
        };
        if segments.len() != 1
            || !announced.contains(&segments[0].ident)
            || !working_traits.contains(&remove_path_args(trait_path))
        {
            continue;
        }

        // Rename the generic parameters, so that those of different impls do not collide
        let mut renamed_generics = generics.clone();
        let mut renamed_self_ty = self_ty.as_ref().clone();
        let mut renamed_trait_path = trait_path.clone();
        randomize_generic_params(
            &mut renamed_generics,
            &mut renamed_self_ty,
            &mut renamed_trait_path,
            ix0,
            random_suffix,
        );

        let unalias = |path: &mut Path| {
            if path.leading_colon.is_none() && path.segments.len() == 1 {
                if let Some((_, original)) =
                    aliases.iter().find(|(alias, _)| alias == &path.segments[0].ident)
                {
                    path.segments[0].ident = original.clone();
                }
            }
        };
        let mut constraint = Constraint {
            typ: renamed_self_ty,
            trait_path: renamed_trait_path,
        };
        unalias(&mut constraint.trait_path);
        let mut children = Vec::new();
        Constraint::map_generics(&mut renamed_generics.clone(), |c| {
            children.push(c.clone());
            vec![c]
        });
        for child in children.iter_mut() {
            unalias(&mut child.trait_path);
        }
        type_impl_table
            .entry(segments[0].ident.clone())
            .or_default()
            .push((renamed_generics, constraint, children));
    }
    announced
        .iter()
        .map(|ty_ident| {
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
        .collect()
}
