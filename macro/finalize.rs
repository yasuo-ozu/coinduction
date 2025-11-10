use proc_macro::TokenStream;
use template_quote::quote;
use syn::{parse_macro_input, ItemMod, ItemImpl};
use crate::common::{ConstraintGraph, TypeConstraint, constraints_match};

#[derive(Debug)]
struct FinalizeArgs {
    module: ItemMod,
    graphs: Vec<ConstraintGraph>,
}

impl syn::parse::Parse for FinalizeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let module: ItemMod = input.parse()?;
        let _: syn::Token![,] = input.parse()?;
        
        // Parse graphs
        let content;
        syn::bracketed!(content in input);
        let mut graphs = Vec::new();
        while !content.is_empty() {
            graphs.push(content.parse::<ConstraintGraph>()?);
            if !content.is_empty() {
                let _: syn::Token![,] = content.parse()?;
            }
        }

        Ok(FinalizeArgs { module, graphs })
    }
}

pub fn finalize_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as FinalizeArgs);
    
    let mut modified_module = args.module;
    
    // Process each item in the module
    if let Some(ref mut items) = modified_module.content {
        for item in &mut items.1 {
            if let syn::Item::Impl(impl_item) = item {
                if let Some(graph) = find_matching_graph(&args.graphs, impl_item) {
                    modify_impl_block(impl_item, graph);
                }
            }
        }
    }

    let result = quote! { #modified_module };
    result.into()
}

fn find_matching_graph<'a>(graphs: &'a [ConstraintGraph], impl_item: &ItemImpl) -> Option<&'a ConstraintGraph> {
    // Find a graph that has a constraint matching this impl block's self type and trait
    if let Some((_, trait_path, _)) = &impl_item.trait_ {
        let self_constraint = TypeConstraint {
            ty: (*impl_item.self_ty).clone(),
            trait_path: trait_path.clone(),
        };
        
        for graph in graphs {
            // Check if this graph has a constraint that matches
            if graph.constraints().any(|constraint| constraints_match(constraint, &self_constraint)) {
                return Some(graph);
            }
        }
    }
    
    None
}

fn modify_impl_block(impl_item: &mut ItemImpl, graph: &ConstraintGraph) {
    // Find strongly connected components (cycles) in the constraint graph
    let sccs = graph.find_strongly_connected_components();
    
    // Find cycles (SCCs with more than one node)
    let cycles: Vec<_> = sccs.into_iter().filter(|scc| scc.len() > 1).collect();
    
    if cycles.is_empty() {
        return; // No cycles, nothing to modify
    }
    
    // Get all constraints that are part of cycles
    let cyclic_constraints: std::collections::HashSet<usize> = cycles.iter().flatten().copied().collect();
    
    // Remove constraints that are part of cycles from where clause
    remove_cyclic_constraints_from_generics(&mut impl_item.generics, graph, &cyclic_constraints);
    
    // Add leaf constraints (constraints that are not part of cycles but are reachable from cycles)
    add_leaf_constraints_to_generics(&mut impl_item.generics, graph, &cyclic_constraints);
}

fn remove_cyclic_constraints_from_generics(
    generics: &mut syn::Generics,
    graph: &ConstraintGraph,
    cyclic_constraints: &std::collections::HashSet<usize>,
) {
    
    // Remove cyclic bounds from generic parameters
    for param in &mut generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let mut new_bounds = syn::punctuated::Punctuated::new();
            for bound in &type_param.bounds {
                let keep_bound = if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    let constraint = TypeConstraint {
                        ty: syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: type_param.ident.clone().into(),
                        }),
                        trait_path: trait_bound.path.clone(),
                    };
                    
                    // Keep the bound if it's not part of a cycle
                    !is_constraint_in_cycle(graph, &constraint, cyclic_constraints)
                } else {
                    true // Keep non-trait bounds
                };
                
                if keep_bound {
                    new_bounds.push(bound.clone());
                }
            }
            type_param.bounds = new_bounds;
        }
    }
    
    // Remove cyclic predicates from where clause
    if let Some(where_clause) = &mut generics.where_clause {
        let mut new_predicates = syn::punctuated::Punctuated::new();
        for predicate in &where_clause.predicates {
            let keep_predicate = if let syn::WherePredicate::Type(type_predicate) = predicate {
                let mut new_bounds = syn::punctuated::Punctuated::new();
                for bound in &type_predicate.bounds {
                    let keep_bound = if let syn::TypeParamBound::Trait(trait_bound) = bound {
                        let constraint = TypeConstraint {
                            ty: type_predicate.bounded_ty.clone(),
                            trait_path: trait_bound.path.clone(),
                        };
                        
                        // Keep the bound if it's not part of a cycle
                        !is_constraint_in_cycle(graph, &constraint, cyclic_constraints)
                    } else {
                        true // Keep non-trait bounds
                    };
                    
                    if keep_bound {
                        new_bounds.push(bound.clone());
                    }
                }
                
                // Keep the predicate if it has any bounds left
                if !new_bounds.is_empty() {
                    let mut new_predicate = type_predicate.clone();
                    new_predicate.bounds = new_bounds;
                    new_predicates.push(syn::WherePredicate::Type(new_predicate));
                }
                false // We handled it above
            } else {
                true // Keep non-type predicates
            };
            
            if keep_predicate {
                new_predicates.push(predicate.clone());
            }
        }
        where_clause.predicates = new_predicates;
    }
}

fn add_leaf_constraints_to_generics(
    generics: &mut syn::Generics,
    graph: &ConstraintGraph,
    cyclic_constraints: &std::collections::HashSet<usize>,
) {
    // Find leaf constraints (reachable from cycles but not part of cycles)
    let mut leaf_constraints = Vec::new();
    
    for &cyclic_node in cyclic_constraints {
        for neighbor in graph.neighbors(cyclic_node) {
            if !cyclic_constraints.contains(&neighbor) {
                if let Some(constraint) = graph.get_constraint(neighbor) {
                    leaf_constraints.push(constraint.clone());
                }
            }
        }
    }
    
    // Add leaf constraints to where clause
    if !leaf_constraints.is_empty() {
        // Ensure we have a where clause
        if generics.where_clause.is_none() {
            generics.where_clause = Some(syn::WhereClause {
                where_token: syn::Token![where](proc_macro2::Span::call_site()),
                predicates: syn::punctuated::Punctuated::new(),
            });
        }
        
        if let Some(where_clause) = &mut generics.where_clause {
            for constraint in leaf_constraints {
                let predicate = syn::WherePredicate::Type(syn::PredicateType {
                    lifetimes: None,
                    bounded_ty: constraint.ty,
                    colon_token: syn::Token![:](proc_macro2::Span::call_site()),
                    bounds: {
                        let mut bounds = syn::punctuated::Punctuated::new();
                        bounds.push(syn::TypeParamBound::Trait(syn::TraitBound {
                            paren_token: None,
                            modifier: syn::TraitBoundModifier::None,
                            lifetimes: None,
                            path: constraint.trait_path,
                        }));
                        bounds
                    },
                });
                
                where_clause.predicates.push(predicate);
            }
        }
    }
}

fn is_constraint_in_cycle(
    graph: &ConstraintGraph,
    constraint: &TypeConstraint,
    cyclic_constraints: &std::collections::HashSet<usize>,
) -> bool {
    if let Some(node_id) = graph.find_constraint(constraint) {
        cyclic_constraints.contains(&node_id)
    } else {
        false
    }
}


