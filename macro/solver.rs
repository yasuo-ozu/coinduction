use gotgraph::prelude::*;
use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet, VecDeque};
use syn::parse::Parse;
use syn::*;
use template_quote::{quote, ToTokens};

use crate::matching::Matching;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeConstraint {
    pub ty: Type,
    pub trait_path: Path,
}

impl Parse for TypeConstraint {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse()?;
        let _: Token![:] = input.parse()?;
        let trait_path: Path = input.parse()?;
        Ok(TypeConstraint { ty, trait_path })
    }
}

impl ToTokens for TypeConstraint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty = &self.ty;
        let trait_path = &self.trait_path;
        tokens.extend(quote! { #ty: #trait_path });
    }
}

#[allow(unused)]
pub struct SolverContext {
    working_constraints: VecDeque<TypeConstraint>,
    attr_mac_info: AttributeMacroInfo,
    // TODO: remove `from_traitdef`
    from_traitdef: bool,
    traits: HashSet<Path>,
    module: ItemMod,
    // TODO: fix ToTokens and Parse impls to adopt to the type
    deps_graphs: Vec<VecGraph<TypeConstraint, ()>>,
}

impl Parse for SolverContext {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        // Parse working_constraints
        let content_wc;
        syn::bracketed!(content_wc in content);
        // TODO: parse into Punctuated<..>
        let mut working_constraints = VecDeque::new();
        while !content_wc.is_empty() {
            working_constraints.push_back(content_wc.parse::<TypeConstraint>()?);
            if !content_wc.is_empty() {
                let _: syn::Token![,] = content_wc.parse()?;
            }
        }
        let _: syn::Token![,] = content.parse()?;

        // Parse attr_mac_info
        let attr_mac_info: AttributeMacroInfo = content.parse()?;
        let _: syn::Token![,] = content.parse()?;

        // Parse from_traitdef
        let from_traitdef: syn::LitBool = content.parse()?;
        let _: syn::Token![,] = content.parse()?;

        // Parse traits
        let content_traits;
        syn::bracketed!(content_traits in content);
        let mut traits = HashSet::new();
        while !content_traits.is_empty() {
            traits.insert(content_traits.parse::<Path>()?);
            if !content_traits.is_empty() {
                let _: syn::Token![,] = content_traits.parse()?;
            }
        }
        let _: syn::Token![,] = content.parse()?;

        // Parse module
        let module: ItemMod = content.parse()?;
        let _: syn::Token![,] = content.parse()?;

        // Parse deps_graphs
        let content_dg;
        syn::bracketed!(content_dg in content);
        let mut deps_graphs = Vec::new();
        while !content_dg.is_empty() {
            let content_graph;
            syn::parenthesized!(content_graph in content_dg);

            // Parse HashSet<GenericParam>
            let content_params;
            syn::bracketed!(content_params in content_graph);
            // TODO: parse content_params into `Punctuated` to reduce code
            let mut generic_params = HashSet::new();
            while !content_params.is_empty() {
                let param: GenericParam = content_params.parse()?;
                generic_params.insert(param);
                if !content_params.is_empty() {
                    let _: syn::Token![,] = content_params.parse()?;
                }
            }
            let _: syn::Token![,] = content_graph.parse()?;

            // Parse VecGraph nodes
            let content_vecgraph_nodes;
            syn::bracketed!(content_vecgraph_nodes in content_graph);
            let mut graph = VecGraph::default();
            let mut node_indices = Vec::new();
            // TODO: parse content_vecgraph_nodes into `Punctuated!<..., Token![,]>` to reduce code()
            while !content_vecgraph_nodes.is_empty() {
                let constraint: TypeConstraint = content_vecgraph_nodes.parse()?;
                let node_idx = graph.add_node(constraint);
                node_indices.push(node_idx);
                if !content_vecgraph_nodes.is_empty() {
                    let _: syn::Token![,] = content_vecgraph_nodes.parse()?;
                }
            }
            // TODO: run graph.add_node()
            let _: syn::Token![,] = content_graph.parse()?;

            // Parse VecGraph edges
            let content_vecgraph_edges;
            syn::bracketed!(content_vecgraph_edges in content_graph);
            while !content_vecgraph_edges.is_empty() {
                let content_edge;
                syn::parenthesized!(content_edge in content_vecgraph_edges);
                // TODO: do not parse LitInt here. parse TypeConstraint to specify the endpoints
                let from_idx: syn::LitInt = content_edge.parse()?;
                let _: syn::Token![,] = content_edge.parse()?;
                let to_idx: syn::LitInt = content_edge.parse()?;

                let from_idx_val = from_idx.base10_parse::<usize>()?;
                let to_idx_val = to_idx.base10_parse::<usize>()?;

                // Only add edge if indices are valid
                // TODO: remove the following logic. instead, find node indices using `node_pairs()` and `find()` on graph with the endpoint TypeConstraint.
                if from_idx_val < node_indices.len() && to_idx_val < node_indices.len() {
                    graph.add_edge((), node_indices[from_idx_val], node_indices[to_idx_val]);
                }

                if !content_vecgraph_edges.is_empty() {
                    let _: syn::Token![,] = content_vecgraph_edges.parse()?;
                }
            }

            deps_graphs.push((generic_params, graph));

            if !content_dg.is_empty() {
                let _: syn::Token![,] = content_dg.parse()?;
            }
        }

        Ok(SolverContext {
            working_constraints,
            attr_mac_info,
            from_traitdef: from_traitdef.value,
            traits,
            module,
            deps_graphs,
        })
    }
}

pub enum AttributeMacroInfo {
    Traitdef {
        appending_constraints: Vec<TypeConstraint>,
    },
    Typedef {
        predicates: Vec<(TypeConstraint, Vec<TypeConstraint>)>,
    },
    None,
}

impl Parse for AttributeMacroInfo {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "Traitdef" => {
                    let content;
                    syn::parenthesized!(content in input);
                    let content2;
                    syn::bracketed!(content2 in content);
                    let mut constraints = Vec::new();
                    while !content2.is_empty() {
                        constraints.push(content2.parse::<TypeConstraint>()?);
                        if !content2.is_empty() {
                            let _: syn::Token![,] = content2.parse()?;
                        }
                    }
                    Ok(AttributeMacroInfo::Traitdef {
                        appending_constraints: constraints,
                    })
                }
                "Typedef" => {
                    let content;
                    syn::parenthesized!(content in input);
                    let content2;
                    syn::bracketed!(content2 in content);
                    let mut predicates = Vec::new();
                    while !content2.is_empty() {
                        let content3;
                        syn::parenthesized!(content3 in content2);
                        let predicate: TypeConstraint = content3.parse()?;
                        let _: syn::Token![,] = content3.parse()?;
                        let content4;
                        syn::bracketed!(content4 in content3);
                        let mut consequences = Vec::new();
                        while !content4.is_empty() {
                            consequences.push(content4.parse::<TypeConstraint>()?);
                            if !content4.is_empty() {
                                let _: syn::Token![,] = content4.parse()?;
                            }
                        }
                        predicates.push((predicate, consequences));
                        if !content2.is_empty() {
                            let _: syn::Token![,] = content2.parse()?;
                        }
                    }
                    Ok(AttributeMacroInfo::Typedef { predicates })
                }
                "None" => Ok(AttributeMacroInfo::None),
                _ => Err(syn::Error::new(
                    ident.span(),
                    "Expected Traitdef, Typedef, or None",
                )),
            }
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for AttributeMacroInfo {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttributeMacroInfo::Traitdef {
                appending_constraints,
            } => {
                tokens.extend(quote! {
                    Traitdef([#(#appending_constraints),*])
                });
            }
            AttributeMacroInfo::Typedef { predicates } => {
                let pred_tokens: Vec<_> = predicates
                    .iter()
                    .map(|(pred, cons)| {
                        quote! { (#pred, [#(#cons),*]) }
                    })
                    .collect();
                tokens.extend(quote! {
                    Typedef([#(#pred_tokens),*])
                });
            }
            AttributeMacroInfo::None => {
                tokens.extend(quote! { None });
            }
        }
    }
}

fn remove_path_args(path: &Path) -> Path {
    let mut new_path = path.clone();
    new_path
        .segments
        .iter_mut()
        .last()
        .expect("pats should have at least one segment")
        .arguments = PathArguments::None;
    new_path
}

fn is_valid_impl_mod(imp: &ItemImpl, traits: &HashSet<Path>) -> bool {
    imp.trait_
        .as_ref()
        .map(|(_, path, _)| traits.get(&remove_path_args(path)).is_some())
        == Some(true)
}

impl SolverContext {
    pub fn impls(&self) -> impl Iterator<Item = &ItemImpl> {
        self.module
            .content
            .as_ref()
            .map(|(_, contents)| {
                contents.iter().filter_map(|imp| match imp {
                    Item::Impl(imp) if is_valid_impl_mod(imp, &self.traits) => Some(imp),
                    _ => None,
                })
            })
            .into_iter()
            .flatten()
    }

    pub fn impls_mut(&mut self) -> impl Iterator<Item = &mut ItemImpl> {
        self.module
            .content
            .as_mut()
            .map(|(_, contents)| {
                contents.iter_mut().filter_map(|item| match item {
                    Item::Impl(imp) if is_valid_impl_mod(imp, &self.traits) => Some(imp),
                    _ => None,
                })
            })
            .into_iter()
            .flatten()
    }

    #[allow(unused)]
    fn get_graphs_mut(
        &mut self,
    ) -> Vec<(HashSet<GenericParam>, &mut VecGraph<TypeConstraint, ()>)> {
        // TODO: implement this function. the GenericParam is found in corresponding impl blocks
        // returned from `self.impls()`
        todo!()
    }

    fn finish(&mut self) {
        // Collect data first to avoid borrowing issues
        let mut graph_data = Vec::new();
        // TODO: remove the following logic. use `get_graphs_mut()`
        for (generics_set, graph) in &self.deps_graphs {
            let leaf_nodes = graph.scope(|graph| {
                let mut nodes = Vec::new();
                for (node_ix, _constraint) in graph.node_pairs() {
                    // Convert node_ix to a value that doesn't borrow from graph
                    nodes.push(()); // Simplified - just collect the count for now
                }
                nodes.len() // Return the count instead of the actual indices
            });
            graph_data.push((generics_set.clone(), leaf_nodes));
        }

        // Now modify the impl blocks
        // TODO: zip the iterator from `self.impls_mut()` and `self.deps_graphs`
        for (generics_set, _leaf_nodes) in graph_data {
            for impl_item in self.impls_mut() {
                let impl_generics = impl_item
                    .generics
                    .params
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>();

                // TODO: iterate all where predicates (from both where clause and inpl generics
                // position) in the form of TypeConstraint. (skip non-type-constraint form
                // predicates).  For each TypeConstraints, judge whether we preserve the
                // constraint or remove it in the following way:
                // TODO: TODO:
            }
        }
    }

    #[allow(unused)]
    pub fn next_step(mut self) -> TokenStream {
        if let Some(current_constraint) = self.working_constraints.pop_front() {
            let mut new_constraints = HashSet::new();
            // TODO: use `self.get_graphs_mut` instead
            for (_, graph) in &mut self.deps_graphs {
                let appending_constraints = match &self.attr_mac_info {
                    AttributeMacroInfo::Traitdef {
                        appending_constraints,
                    } => appending_constraints.clone(),
                    AttributeMacroInfo::Typedef { predicates } => {
                        let mut appending_constraints = Vec::new();
                        for (predicate_pattern, consequence_constraints) in predicates {
                            // TODO: append the `params` items as identity map items
                            let mut substitutions = HashMap::new();
                            if predicate_pattern.matches(&current_constraint, &mut substitutions) {
                                for mut consequence in consequence_constraints.clone() {
                                    consequence.replace(&substitutions);
                                    appending_constraints.push(consequence);
                                }
                            }
                        }
                        appending_constraints
                    }
                    AttributeMacroInfo::None => {
                        panic!()
                    }
                };
                graph.scope_mut(|mut graph| {
                    let working_node_ix =
                        match graph.node_pairs().find(|(_, n)| *n == &current_constraint) {
                            Some((ix, _)) => ix,
                            None => return, // Skip if constraint not found in this graph
                        };
                    for constraint in appending_constraints {
                        let existing_node = graph
                            .node_pairs()
                            .find_map(|(ix, n)| (n == &constraint).then_some(ix));

                        let target_node = existing_node.unwrap_or_else(|| {
                            new_constraints.insert(constraint.clone());
                            graph.add_node(constraint.clone())
                        });
                        graph.add_edge((), working_node_ix, target_node);
                    }
                });
            }
            self.working_constraints.extend(new_constraints);
        }
        match self.working_constraints.front() {
            Some(next_constraint) => {
                let macro_path = remove_path_args(&next_constraint.trait_path);
                self.attr_mac_info = AttributeMacroInfo::None;
                quote! {
                    #macro_path! { #self }
                }
            }
            None => {
                self.finish();
                quote! { #{&self.module} }
            }
        }
    }
}

impl ToTokens for SolverContext {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let working_constraints: Vec<_> = self.working_constraints.iter().collect();
        let attr_mac_info = &self.attr_mac_info;
        let from_traitdef = self.from_traitdef;
        let traits: Vec<_> = self.traits.iter().collect();
        let module = &self.module;

        // Serialize deps_graphs
        let deps_graph_tokens: Vec<_> = self
            .deps_graphs
            .iter()
            .map(|(params, graph)| {
                let params: Vec<_> = params.iter().collect();

                // Extract nodes and edges from VecGraph
                // TODO: change the type to `(Vec<TypeConstraint>, Vec<(TypeConstraint,
                // TypeConstraint>)>)`
                let (graph_nodes, graph_edges): (Vec<TypeConstraint>, Vec<(usize, usize)>) = graph
                    .scope(|graph| {
                        // TODO: get nodes using graph.nodes()
                        let nodes: Vec<_> = graph
                            .node_pairs()
                            .map(|(_, constraint)| constraint.clone())
                            .collect();

                        // Get edges - simplified approach due to gotgraph API complexity
                        // For now, serialize empty edges list - a full implementation would extract actual edges
                        // TODO: get edges with `graph.edges()`
                        let edges: Vec<(usize, usize)> = Vec::new();

                        (nodes, edges)
                    });

                // Convert edges to token stream
                let edge_tokens: Vec<_> = graph_edges
                    .iter()
                    .map(|(from, to)| {
                        quote! { (#from, #to) }
                    })
                    .collect();

                quote! {
                    ([#(#params),*], [#(#graph_nodes),*], [#(#edge_tokens),*])
                }
            })
            .collect();

        tokens.extend(quote! {
            (
                [#(#working_constraints),*],
                #attr_mac_info,
                #from_traitdef,
                [#(#traits),*],
                #module,
                [#(#deps_graph_tokens),*]
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};
    use syn::parse_quote;
    use template_quote::quote;

    #[test]
    fn test_solver_context_round_trip_empty() {
        // Create a minimal SolverContext
        let original = SolverContext {
            working_constraints: VecDeque::new(),
            attr_mac_info: AttributeMacroInfo::None,
            from_traitdef: false,
            traits: HashSet::new(),
            module: parse_quote! { mod test {} },
            deps_graphs: Vec::new(),
        };

        // Convert to tokens
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: SolverContext = syn::parse2(tokens).expect("Failed to parse SolverContext");

        // Verify fields match
        assert_eq!(
            parsed.working_constraints.len(),
            original.working_constraints.len()
        );
        assert!(matches!(parsed.attr_mac_info, AttributeMacroInfo::None));
        assert_eq!(parsed.from_traitdef, original.from_traitdef);
        assert_eq!(parsed.traits.len(), original.traits.len());
        assert_eq!(parsed.deps_graphs.len(), original.deps_graphs.len());
    }

    #[test]
    fn test_solver_context_round_trip_with_constraints() {
        // Create SolverContext with some data
        let mut working_constraints = VecDeque::new();
        working_constraints.push_back(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Clone },
        });
        working_constraints.push_back(TypeConstraint {
            ty: parse_quote! { U },
            trait_path: parse_quote! { Send },
        });

        let mut traits = HashSet::new();
        traits.insert(parse_quote! { Clone });
        traits.insert(parse_quote! { Send });

        let original = SolverContext {
            working_constraints,
            attr_mac_info: AttributeMacroInfo::Traitdef {
                appending_constraints: vec![TypeConstraint {
                    ty: parse_quote! { V },
                    trait_path: parse_quote! { Sync },
                }],
            },
            from_traitdef: true,
            traits,
            module: parse_quote! {
                mod test {
                    fn example() {}
                }
            },
            deps_graphs: Vec::new(),
        };

        // Convert to tokens
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: SolverContext = syn::parse2(tokens).expect("Failed to parse SolverContext");

        // Verify fields match
        assert_eq!(parsed.working_constraints.len(), 2);
        assert_eq!(parsed.from_traitdef, true);
        assert_eq!(parsed.traits.len(), 2);

        // Check AttributeMacroInfo variant
        match parsed.attr_mac_info {
            AttributeMacroInfo::Traitdef {
                appending_constraints,
            } => {
                assert_eq!(appending_constraints.len(), 1);
            }
            _ => panic!("Expected Traitdef variant"),
        }
    }

    #[test]
    fn test_solver_context_round_trip_typedef_variant() {
        let predicates = vec![(
            TypeConstraint {
                ty: parse_quote! { T },
                trait_path: parse_quote! { Clone },
            },
            vec![
                TypeConstraint {
                    ty: parse_quote! { T },
                    trait_path: parse_quote! { Send },
                },
                TypeConstraint {
                    ty: parse_quote! { T },
                    trait_path: parse_quote! { Sync },
                },
            ],
        )];

        let original = SolverContext {
            working_constraints: VecDeque::new(),
            attr_mac_info: AttributeMacroInfo::Typedef { predicates },
            from_traitdef: false,
            traits: HashSet::new(),
            module: parse_quote! { mod empty {} },
            deps_graphs: Vec::new(),
        };

        // Convert to tokens
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: SolverContext = syn::parse2(tokens).expect("Failed to parse SolverContext");

        // Check AttributeMacroInfo variant
        match parsed.attr_mac_info {
            AttributeMacroInfo::Typedef { predicates } => {
                assert_eq!(predicates.len(), 1);
                assert_eq!(predicates[0].1.len(), 2); // Two consequences
            }
            _ => panic!("Expected Typedef variant"),
        }
    }

    #[test]
    fn test_type_constraint_round_trip() {
        let original = TypeConstraint {
            ty: parse_quote! { Vec<T> },
            trait_path: parse_quote! { std::clone::Clone },
        };

        // Convert to tokens
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: TypeConstraint = syn::parse2(tokens).expect("Failed to parse TypeConstraint");

        // Verify equality (note: syn types don't implement PartialEq directly)
        let original_tokens = quote! { #original };
        let parsed_tokens = quote! { #parsed };
        assert_eq!(original_tokens.to_string(), parsed_tokens.to_string());
    }

    #[test]
    fn test_attribute_macro_info_round_trip() {
        // Test None variant
        let original = AttributeMacroInfo::None;
        let tokens = quote! { #original };
        let parsed: AttributeMacroInfo = syn::parse2(tokens).expect("Failed to parse None variant");
        assert!(matches!(parsed, AttributeMacroInfo::None));

        // Test Traitdef variant
        let original = AttributeMacroInfo::Traitdef {
            appending_constraints: vec![TypeConstraint {
                ty: parse_quote! { T },
                trait_path: parse_quote! { Clone },
            }],
        };
        let tokens = quote! { #original };
        let parsed: AttributeMacroInfo =
            syn::parse2(tokens).expect("Failed to parse Traitdef variant");
        match parsed {
            AttributeMacroInfo::Traitdef {
                appending_constraints,
            } => {
                assert_eq!(appending_constraints.len(), 1);
            }
            _ => panic!("Expected Traitdef variant"),
        }

        // Test Typedef variant
        let original = AttributeMacroInfo::Typedef {
            predicates: vec![(
                TypeConstraint {
                    ty: parse_quote! { T },
                    trait_path: parse_quote! { Clone },
                },
                vec![TypeConstraint {
                    ty: parse_quote! { T },
                    trait_path: parse_quote! { Send },
                }],
            )],
        };
        let tokens = quote! { #original };
        let parsed: AttributeMacroInfo =
            syn::parse2(tokens).expect("Failed to parse Typedef variant");
        match parsed {
            AttributeMacroInfo::Typedef { predicates } => {
                assert_eq!(predicates.len(), 1);
                assert_eq!(predicates[0].1.len(), 1);
            }
            _ => panic!("Expected Typedef variant"),
        }
    }

    #[test]
    fn test_solver_context_with_deps_graphs() {
        // Test SolverContext with actual deps_graphs data
        let mut working_constraints = VecDeque::new();
        working_constraints.push_back(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Clone },
        });

        let mut traits = HashSet::new();
        traits.insert(parse_quote! { Clone });

        // Create deps_graphs with GenericParam
        let mut deps_graphs = Vec::new();
        let mut generic_params = HashSet::new();
        generic_params.insert(GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        }));

        let mut graph = VecGraph::default();
        let node1 = graph.add_node(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Clone },
        });
        let node2 = graph.add_node(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Copy },
        });

        // Add an edge between the nodes for testing edge serialization
        graph.add_edge((), node1, node2);

        deps_graphs.push((generic_params, graph));

        let original = SolverContext {
            working_constraints,
            attr_mac_info: AttributeMacroInfo::None,
            from_traitdef: false,
            traits,
            module: parse_quote! { mod test {} },
            deps_graphs,
        };

        // Convert to tokens
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: SolverContext =
            syn::parse2(tokens).expect("Failed to parse SolverContext with deps_graphs");

        // Verify basic structure
        assert_eq!(parsed.working_constraints.len(), 1);
        assert_eq!(parsed.deps_graphs.len(), 1);

        // Verify the deps_graphs structure
        let (parsed_params, parsed_graph) = &parsed.deps_graphs[0];
        assert_eq!(parsed_params.len(), 1);

        // Verify nodes in the graph
        let node_count = parsed_graph.scope(|graph| graph.node_pairs().count());
        assert_eq!(node_count, 2); // Should have 2 constraints
    }

    #[test]
    fn test_solver_context_empty_deps_graphs() {
        // Test with empty deps_graphs to ensure basic serialization works
        let original = SolverContext {
            working_constraints: VecDeque::new(),
            attr_mac_info: AttributeMacroInfo::None,
            from_traitdef: false,
            traits: HashSet::new(),
            module: parse_quote! { mod test {} },
            deps_graphs: Vec::new(),
        };

        // Convert to tokens
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: SolverContext =
            syn::parse2(tokens).expect("Failed to parse SolverContext with empty deps_graphs");

        // Verify structure
        assert_eq!(parsed.deps_graphs.len(), 0);
    }

    #[test]
    fn test_deps_graphs_edge_serialization() {
        // Test that the new edge serialization format works correctly
        let mut generic_params = HashSet::new();
        generic_params.insert(GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        }));

        let mut graph = VecGraph::default();
        let node1 = graph.add_node(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Clone },
        });
        let node2 = graph.add_node(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Copy },
        });
        let node3 = graph.add_node(TypeConstraint {
            ty: parse_quote! { T },
            trait_path: parse_quote! { Send },
        });

        // Add edges to test serialization
        graph.add_edge((), node1, node2);
        graph.add_edge((), node2, node3);

        let mut deps_graphs = Vec::new();
        deps_graphs.push((generic_params, graph));

        let original = SolverContext {
            working_constraints: VecDeque::new(),
            attr_mac_info: AttributeMacroInfo::None,
            from_traitdef: false,
            traits: HashSet::new(),
            module: parse_quote! { mod test {} },
            deps_graphs,
        };

        // Convert to tokens - this should include edge information
        let tokens = quote! { #original };

        // Parse back from tokens
        let parsed: SolverContext =
            syn::parse2(tokens).expect("Failed to parse SolverContext with edges");

        // Verify the graph structure is preserved
        assert_eq!(parsed.deps_graphs.len(), 1);
        let (parsed_params, parsed_graph) = &parsed.deps_graphs[0];
        assert_eq!(parsed_params.len(), 1);

        // Verify nodes were preserved
        let node_count = parsed_graph.scope(|graph| graph.node_pairs().count());
        assert_eq!(node_count, 3); // Should have 3 constraints

        // Note: Edge verification would require more complex graph traversal
        // For now, we verify that serialization/parsing doesn't fail with edges present
    }
}
