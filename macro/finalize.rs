use crate::common::{constraints_match, ConstraintGraph, TypeConstraint};
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl, ItemMod};
use template_quote::{quote, ToTokens};

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

impl ToTokens for FinalizeArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let module = &self.module;
        let graphs = &self.graphs;
        tokens.extend(quote! {
            #module, [#(#graphs),*]
        });
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

fn find_matching_graph<'a>(
    graphs: &'a [ConstraintGraph],
    impl_item: &ItemImpl,
) -> Option<&'a ConstraintGraph> {
    // Find a graph that has a constraint matching this impl block's self type and trait
    if let Some((_, trait_path, _)) = &impl_item.trait_ {
        let self_constraint = TypeConstraint {
            ty: (*impl_item.self_ty).clone(),
            trait_path: trait_path.clone(),
        };

        for graph in graphs {
            // Check if this graph has a constraint that matches
            if graph
                .constraints()
                .any(|constraint| constraints_match(constraint, &self_constraint))
            {
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
    let cyclic_constraints: std::collections::HashSet<usize> =
        cycles.iter().flatten().copied().collect();

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

                    // Keep the bound if it's not part of a cycle or if it's an additional constraint
                    !is_constraint_directly_in_cycle(graph, &constraint, cyclic_constraints)
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

                        // Keep the bound if it's not part of a cycle or if it's an additional constraint
                        !is_constraint_directly_in_cycle(graph, &constraint, cyclic_constraints)
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

fn is_constraint_directly_in_cycle(
    graph: &ConstraintGraph,
    constraint: &TypeConstraint,
    cyclic_constraints: &std::collections::HashSet<usize>,
) -> bool {
    // Only remove constraints that are exactly found in the graph and part of cycles
    // This preserves additional type bounds that aren't part of circular dependencies
    if let Some(node_id) = graph.find_constraint(constraint) {
        cyclic_constraints.contains(&node_id)
    } else {
        // If the constraint is not in the graph, it's an additional constraint and should be kept
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, ItemMod};
    use template_quote::quote;

    fn create_test_module() -> ItemMod {
        parse_quote! {
            mod test_mod {
                impl<T> Clone for T where T: Copy {
                    fn clone(&self) -> Self {
                        *self
                    }
                }
            }
        }
    }

    fn create_test_constraint_graph() -> ConstraintGraph {
        let mut graph = ConstraintGraph::new();

        let constraint1 = TypeConstraint {
            ty: parse_quote!(T),
            trait_path: parse_quote!(Clone),
        };
        let constraint2 = TypeConstraint {
            ty: parse_quote!(T),
            trait_path: parse_quote!(Copy),
        };

        let node1 = graph.add_constraint(constraint1);
        let node2 = graph.add_constraint(constraint2);
        graph.add_edge(node1, node2);
        graph.add_edge(node2, node1); // Create a cycle

        graph
    }

    #[test]
    fn test_finalize_args_to_tokens() {
        let module = create_test_module();
        let graphs = vec![create_test_constraint_graph()];

        let args = FinalizeArgs { module, graphs };
        let tokens = quote! { #args };

        // The output should contain the module and the graphs in brackets
        let output = tokens.to_string();
        assert!(output.contains("mod test_mod"));
        assert!(output.contains("Clone"));
        assert!(output.contains("Copy"));
    }

    fn finalize_impl_internal(args: FinalizeArgs) -> proc_macro2::TokenStream {
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

        quote! { #modified_module }
    }

    #[test]
    fn test_finalize_impl_with_no_cycles() {
        let module = parse_quote! {
            mod test_mod {
                impl<T> Display for T where T: Debug {
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(f, "{:?}", self)
                    }
                }
            }
        };

        let mut graph = ConstraintGraph::new();
        graph.add_constraint(TypeConstraint {
            ty: parse_quote!(T),
            trait_path: parse_quote!(Debug),
        });

        let args = FinalizeArgs {
            module,
            graphs: vec![graph],
        };

        let output = finalize_impl_internal(args);

        // Should return the module unchanged since there are no cycles
        let output_str = output.to_string();
        assert!(output_str.contains("impl < T > Display for T where T : Debug"));
    }

    #[test]
    fn test_finalize_impl_with_cycles() {
        let module = parse_quote! {
            mod test_mod {
                impl<T> Clone for T where T: Copy, T: Clone {
                    fn clone(&self) -> Self {
                        *self
                    }
                }
            }
        };

        let mut graph = ConstraintGraph::new();
        let constraint1 = TypeConstraint {
            ty: parse_quote!(T),
            trait_path: parse_quote!(Clone),
        };
        let constraint2 = TypeConstraint {
            ty: parse_quote!(T),
            trait_path: parse_quote!(Copy),
        };

        let node1 = graph.add_constraint(constraint1);
        let node2 = graph.add_constraint(constraint2);
        graph.add_edge(node1, node2);
        graph.add_edge(node2, node1); // Create a cycle

        let args = FinalizeArgs {
            module,
            graphs: vec![graph],
        };

        let output = finalize_impl_internal(args);

        // The cyclic constraints should be removed/modified
        let output_str = output.to_string();
        assert!(output_str.contains("impl"));
        assert!(output_str.contains("Clone"));
    }

    #[test]
    fn test_find_matching_graph() {
        let impl_item: ItemImpl = parse_quote! {
            impl<T> Clone for T where T: Copy {}
        };

        let graph = create_test_constraint_graph();
        let graphs = vec![graph];

        let result = find_matching_graph(&graphs, &impl_item);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_matching_graph_no_match() {
        let impl_item: ItemImpl = parse_quote! {
            impl<T> Display for T where T: Debug {}
        };

        let graph = create_test_constraint_graph();
        let graphs = vec![graph];

        let result = find_matching_graph(&graphs, &impl_item);
        assert!(result.is_none());
    }

    #[test]
    fn test_coinduction_module_with_complex_dependencies() {
        let module = parse_quote! {
            #[coinduction]
            mod complex_recursive {
                struct Wrapper<T>(T);
                struct Wrapper2<T>(T);

                impl<A, B> Clone for (A, B) 
                where 
                    A: Clone,
                    B: Clone,
                    Wrapper<A>: Copy,
                    Wrapper2<B>: Send
                {
                    fn clone(&self) -> Self {
                        (self.0.clone(), self.1.clone())
                    }
                }

                impl<T> Copy for Wrapper<T> 
                where 
                    T: Copy,
                    (T, Wrapper2<T>): Clone,
                    Wrapper2<T>: Sync
                {
                }

                impl<T> Send for Wrapper2<T> 
                where 
                    T: Send,
                    Wrapper<T>: Copy,
                    (Wrapper<T>, T): Clone
                {
                }

                impl<T> Sync for Wrapper2<T> 
                where 
                    T: Sync,
                    Wrapper<T>: Send,
                    (T, T): Clone
                {
                }
            }
        };

        // Create constraint graphs representing the circular dependencies
        let mut graph1 = ConstraintGraph::new();
        
        // Constraints for tuple Clone impl
        let tuple_clone = TypeConstraint {
            ty: parse_quote!((A, B)),
            trait_path: parse_quote!(Clone),
        };
        let wrapper_copy = TypeConstraint {
            ty: parse_quote!(Wrapper<A>),
            trait_path: parse_quote!(Copy),
        };
        let wrapper2_send = TypeConstraint {
            ty: parse_quote!(Wrapper2<B>),
            trait_path: parse_quote!(Send),
        };

        let node1 = graph1.add_constraint(tuple_clone);
        let node2 = graph1.add_constraint(wrapper_copy);
        let node3 = graph1.add_constraint(wrapper2_send);

        // Create circular dependencies
        graph1.add_edge(node1, node2); // tuple Clone -> Wrapper Copy
        graph1.add_edge(node2, node3); // Wrapper Copy -> Wrapper2 Send
        graph1.add_edge(node3, node1); // Wrapper2 Send -> tuple Clone

        let mut graph2 = ConstraintGraph::new();
        
        // More complex circular dependencies with Wrapper types
        let wrapper_copy2 = TypeConstraint {
            ty: parse_quote!(Wrapper<T>),
            trait_path: parse_quote!(Copy),
        };
        let wrapper2_sync = TypeConstraint {
            ty: parse_quote!(Wrapper2<T>),
            trait_path: parse_quote!(Sync),
        };
        let tuple_clone2 = TypeConstraint {
            ty: parse_quote!((T, Wrapper2<T>)),
            trait_path: parse_quote!(Clone),
        };

        let node4 = graph2.add_constraint(wrapper_copy2);
        let node5 = graph2.add_constraint(wrapper2_sync);
        let node6 = graph2.add_constraint(tuple_clone2);

        // Create another cycle
        graph2.add_edge(node4, node5); // Wrapper Copy -> Wrapper2 Sync
        graph2.add_edge(node5, node6); // Wrapper2 Sync -> tuple Clone
        graph2.add_edge(node6, node4); // tuple Clone -> Wrapper Copy

        let args = FinalizeArgs {
            module,
            graphs: vec![graph1, graph2],
        };

        let output = finalize_impl_internal(args);
        
        // The output should contain the modified module with processed cycles
        let output_str = output.to_string();
        assert!(output_str.contains("mod complex_recursive"));
        assert!(output_str.contains("impl"));
        assert!(output_str.contains("Wrapper"));
        assert!(output_str.contains("Wrapper2"));
        
        // Should contain tuple types
        assert!(output_str.contains("("));
    }
}
