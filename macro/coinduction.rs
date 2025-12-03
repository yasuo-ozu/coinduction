use crate::common::{ConstraintGraph, TypeConstraint};
use proc_macro::TokenStream;
use syn::{
    parse_macro_input, GenericParam, Generics, ItemImpl, ItemMod, Path, Type, TypeParamBound,
    WherePredicate,
};
use template_quote::quote;

#[derive(Debug)]
struct CoinductionArgs {
    trait_paths: Vec<Path>,
}

impl syn::parse::Parse for CoinductionArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut trait_paths = Vec::new();

        while !input.is_empty() {
            trait_paths.push(input.parse()?);

            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(CoinductionArgs { trait_paths })
    }
}

pub fn coinduction_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let crate_version = env!("CARGO_PKG_VERSION");
    let args = parse_macro_input!(args as CoinductionArgs);
    let mut module = parse_macro_input!(input as ItemMod);

    // Ensure module has content
    // TODO: use unwrap_or_else
    let items = match &mut module.content {
        Some((_, items)) => items,
        None => {
            // TODO: use proc_macro_error crate (abort!)
            return syn::Error::new_spanned(module, "Module must have content")
                .to_compile_error()
                .into();
        }
    };

    // TODO: define new trait_paths collecting all traits implemented in the #[coinduction] module
    // then the args.trait_paths is empty

    // Only validate traits when explicitly provided as arguments
    // Auto-detected traits don't need validation since they're defined in the same module
    let trait_validations = if !args.trait_paths.is_empty() {
        args.trait_paths.iter().filter_map(|trait_path| {
            // Validate all trait paths, but extract the final segment for the macro call
            if let Some(last_segment) = trait_path.segments.last() {
                // Check that the final segment has no generics
                if last_segment.arguments.is_empty() {
                    let trait_name = &last_segment.ident;
                    Some(quote! {
                        const _: () = {
                            // Version check
                            macro_rules! coinduction_crate_version_mismatch {
                                () => {
                                    #trait_name! { @version_check #crate_version }
                                };
                            }
                            coinduction_crate_version_mismatch!();

                            macro_rules! this_trait_is_not_defined_with_coinduction_traitdef {
                                () => {
                                    #trait_name! { trait [#trait_path] is not defined with #[coinduction::traitdef] macro, so is not used as an argument for #[coinduction] macro }
                                };
                            }
                            this_trait_is_not_defined_with_coinduction_traitdef!();
                        };
                    })
                } else {
                    // TODO: we should make it as error
                    None
                }
            } else {
                // TODO: we should make it as error
                None
            }
        }).collect()
    } else {
        Vec::new()
    };

    let mut working_list = Vec::new();
    let mut graphs = Vec::new();

    // Process impl blocks
    for item in items.iter() {
        if let syn::Item::Impl(impl_item) = item {
            if let Some((_, trait_path, _)) = &impl_item.trait_ {
                // Check if this trait is in our arguments
                if args
                    .trait_paths
                    .iter()
                    .any(|tp| paths_equal(tp, trait_path))
                {
                    match process_impl_block(impl_item, &args.trait_paths) {
                        Ok((graph, constraints)) => {
                            graphs.push(graph);
                            working_list.extend(constraints);
                        }
                        Err(e) => return e.to_compile_error().into(),
                    }
                }
            }
        }
    }

    // Remove duplicates from working list
    working_list.sort_by(|a, b| format!("{}", quote! { #a }).cmp(&format!("{}", quote! { #b })));
    working_list.dedup_by(|a, b| format!("{}", quote! { #a }) == format!("{}", quote! { #b }));

    // Apply coinduction processing using the proper finalize system
    // TODO: remove this
    let mut modified_module = module.clone();

    // Apply coinduction processing to the module
    // TODO: the block is not needed
    if let Some((_, ref mut items)) = modified_module.content {
        for item in items.iter_mut() {
            if let syn::Item::Impl(impl_item) = item {
                // Check if this is implementing one of the traits we're interested in
                if let Some((_, trait_path, _)) = &impl_item.trait_ {
                    // Check if this trait is in our arguments
                    if args
                        .trait_paths
                        .iter()
                        .any(|tp| paths_equal(tp, trait_path))
                    {
                        // Apply selective constraint removal - remove only circular trait bounds
                        remove_circular_trait_bounds(impl_item, &args.trait_paths);
                    }
                }
            }
        }
    }

    // Generate the final result - trait validations and modified module
    let result = quote! {
        // Validate all traits are defined with #[traitdef]
        #(#trait_validations)*

        // TODO: do not emit the module. instead, find the first trait in working list if exists
        // and call that path regarding as a macro. The macro is defined in traitdef.rs using
        // macro_rules!
        #modified_module
    };

    result.into()
}

// TODO: add arg for all ItemImpls within the #[coinduction] module
fn process_impl_block(
    impl_item: &ItemImpl,
    trait_paths: &[Path],
) -> syn::Result<(ConstraintGraph, Vec<TypeConstraint>)> {
    let mut graph = ConstraintGraph::new();
    let mut constraints = Vec::new();

    // Validate Self type is just one path segment
    // TODO: use `if let ... = ... {} else {}`
    match &*impl_item.self_ty {
        Type::Path(type_path) => {
            if type_path.path.segments.len() != 1 {
                let path_str = quote! { #type_path }.to_string();
                // TODO use abort! from proc_macro_error
                return Err(syn::Error::new_spanned(
                    &impl_item.self_ty,
                    format!("{} is not defined in the same module\nhint: Self type must be a single path segment", path_str)
                ));
            }
        }
        _ => {
            // TODO use abort! from proc_macro_error
            return Err(syn::Error::new_spanned(
                &impl_item.self_ty,
                "Self type of impl blocks in #[coinduction] module must be a path",
            ));
        }
    }

    // Add root constraint from Self type
    let self_constraint = TypeConstraint {
        ty: (*impl_item.self_ty).clone(),
        trait_path: impl_item.trait_.as_ref().unwrap().1.clone(),
    };
    // TODO: do not clone
    let root_id = graph.set_root(self_constraint.clone());

    // Process constraints from generics and where clause
    let all_constraints = extract_constraints(&impl_item.generics)?;

    for constraint in all_constraints {
        let constraint_id = graph.add_constraint(constraint.clone());
        // Connect constraint to root (dependency relationship)
        graph.add_edge(root_id, constraint_id);

        // Add to working list if trait matches our arguments
        if trait_paths
            .iter()
            .any(|tp| paths_equal(tp, &constraint.trait_path))
        {
            constraints.push(constraint);
        }
    }

    Ok((graph, constraints))
}

// TODO:  change the reference to mutable
fn extract_constraints(generics: &Generics) -> syn::Result<Vec<TypeConstraint>> {
    let mut constraints = Vec::new();

    // Extract from generic parameters
    for param in &generics.params {
        if let GenericParam::Type(type_param) = param {
            // Separate multiple trait bounds (T: Trait1 + Trait2 becomes T: Trait1, T: Trait2)
            for bound in &type_param.bounds {
                if let TypeParamBound::Trait(trait_bound) = bound {
                    // TODO: remove the constraint from generics
                    constraints.push(TypeConstraint {
                        ty: Type::Path(syn::TypePath {
                            qself: None,
                            path: type_param.ident.clone().into(),
                        }),
                        trait_path: trait_bound.path.clone(),
                    });
                }
            }
        }
    }

    // Extract from where clause
    if let Some(where_clause) = &generics.where_clause {
        for predicate in &where_clause.predicates {
            if let WherePredicate::Type(type_predicate) = predicate {
                // Separate multiple trait bounds in where clause
                for bound in &type_predicate.bounds {
                    if let TypeParamBound::Trait(trait_bound) = bound {
                        // TODO: remove the constraint from generics
                        constraints.push(TypeConstraint {
                            ty: type_predicate.bounded_ty.clone(),
                            trait_path: trait_bound.path.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(constraints)
}

fn remove_circular_trait_bounds(impl_item: &mut ItemImpl, circular_traits: &[Path]) {
    // Remove only trait bounds for the specified circular traits, preserve everything else
    if let Some(where_clause) = &mut impl_item.generics.where_clause {
        let mut new_predicates = syn::punctuated::Punctuated::new();

        for predicate in &where_clause.predicates {
            match predicate {
                WherePredicate::Type(type_predicate) => {
                    let mut new_bounds = syn::punctuated::Punctuated::new();

                    for bound in &type_predicate.bounds {
                        let keep_bound = if let TypeParamBound::Trait(trait_bound) = bound {
                            // Check if this trait bound is for one of the circular traits
                            !circular_traits.iter().any(|circular_trait| {
                                paths_equal(circular_trait, &trait_bound.path)
                            })
                        } else {
                            true // Keep all non-trait bounds (lifetime, const, etc.)
                        };

                        if keep_bound {
                            new_bounds.push(bound.clone());
                        }
                    }

                    // Keep the predicate if it has any bounds left
                    if !new_bounds.is_empty() {
                        let mut new_predicate = type_predicate.clone();
                        new_predicate.bounds = new_bounds;
                        new_predicates.push(WherePredicate::Type(new_predicate));
                    }
                }
                _ => {
                    // Keep all non-type predicates (lifetime predicates, etc.)
                    new_predicates.push(predicate.clone());
                }
            }
        }

        where_clause.predicates = new_predicates;
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if a.segments.len() != b.segments.len() {
        return false;
    }

    a.segments
        .iter()
        .zip(b.segments.iter())
        .all(|(seg_a, seg_b)| seg_a.ident == seg_b.ident)
}
