use gotgraph::graph::{Graph, GraphUpdate};
use gotgraph::prelude::VecGraph;
use proc_macro_error::abort;
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::{quote, ToTokens};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Constraint {
    pub typ: Type,
    pub trait_path: Path,
}

impl Constraint {
    pub fn map_where_clause(wc: &mut WhereClause, mut f: impl FnMut(Self) -> Vec<Self>) {
        for mut pair in core::mem::take(&mut wc.predicates).into_pairs() {
            match pair.value_mut() {
                WherePredicate::Type(PredicateType {
                    lifetimes,
                    bounded_ty,
                    bounds,
                    ..
                }) => {
                    if lifetimes.is_some() {
                        todo!("bounded lifetimes is not supported");
                    }
                    let additional_predicates = Self::map_bounds(bounds, bounded_ty, &mut f);
                    wc.predicates.extend(additional_predicates);
                }
                _ => wc.predicates.extend(core::iter::once(pair)),
            }
        }
    }

    fn map_bounds(
        bounds: &mut Punctuated<TypeParamBound, Token![+]>,
        bounded_ty: &Type,
        mut f: impl FnMut(Self) -> Vec<Self>,
    ) -> Vec<WherePredicate> {
        let mut additional_predicates = Vec::new();
        for bound in core::mem::take(bounds).into_pairs() {
            let punct = bound.punct().cloned();
            match bound.into_value() {
                TypeParamBound::Trait(TraitBound {
                    modifier,
                    lifetimes,
                    path,
                    ..
                }) => {
                    if &modifier != &TraitBoundModifier::None {
                        abort!(&modifier, "trait bound modifier is not supported");
                    }
                    if lifetimes.is_some() {
                        todo!("bounded lifetimes is not supported");
                    }
                    for replacing in f(Constraint {
                        typ: bounded_ty.clone(),
                        trait_path: path,
                    }) {
                        let new_bound = TypeParamBound::Trait(TraitBound {
                            paren_token: None,
                            modifier,
                            lifetimes: lifetimes.clone(),
                            path: replacing.trait_path,
                        });
                        if &replacing.typ == bounded_ty {
                            bounds.push(new_bound);
                            if let Some(punct) = punct.clone() {
                                bounds.push_punct(punct);
                            }
                        } else {
                            additional_predicates.push(WherePredicate::Type(PredicateType {
                                lifetimes: None,
                                bounded_ty: replacing.typ,
                                colon_token: Default::default(),
                                bounds: core::iter::once(new_bound).collect(),
                            }))
                        }
                    }
                }
                bound => bounds.extend(core::iter::once(bound)),
            }
        }
        additional_predicates
    }

    pub fn map_generics(generics: &mut Generics, mut f: impl FnMut(Self) -> Vec<Self>) {
        let mut additional_predicates = Vec::new();
        for param in generics.params.iter_mut() {
            additional_predicates.extend(Self::map_generic_param(param, &mut f));
        }
        if let Some(wc) = &mut generics.where_clause {
            Self::map_where_clause(wc, &mut f);
            wc.predicates.extend(additional_predicates);
        } else if additional_predicates.len() > 0 {
            generics.where_clause = Some(WhereClause {
                where_token: Default::default(),
                predicates: additional_predicates.into_iter().collect(),
            });
        }
    }

    pub fn map_generic_param(
        param: &mut GenericParam,
        f: impl FnMut(Self) -> Vec<Self>,
    ) -> Vec<WherePredicate> {
        match param {
            GenericParam::Type(TypeParam { ident, bounds, .. }) => {
                let bounded_ty = Type::Path(TypePath {
                    qself: None,
                    path: ident.clone().into(),
                });
                Self::map_bounds(bounds, &bounded_ty, f)
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct ConstraintTuple {
    from: Constraint,
    to: Constraint,
}

impl Parse for Constraint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let typ = input.parse::<Type>()?;
        input.parse::<Token![:]>()?;
        let trait_path = input.parse::<Path>()?;
        Ok(Constraint { typ, trait_path })
    }
}

impl ToTokens for Constraint {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let typ = &self.typ;
        let trait_path = &self.trait_path;
        tokens.extend(quote! { #typ : #trait_path });
    }
}

impl Parse for ConstraintTuple {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let from = content.parse::<Constraint>()?;
        content.parse::<Token![,]>()?;
        let to = content.parse::<Constraint>()?;
        Ok(ConstraintTuple { from, to })
    }
}

pub struct Solver {
    pub graph: VecGraph<Constraint, ()>,
    pub generic_params: HashSet<GenericParam>,
}

impl Parse for Solver {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse { [...], [...], [...] }
        let content;
        syn::braced!(content in input);
        // Parse vertices
        let vertices_content;
        syn::bracketed!(vertices_content in content);
        let vertices: Punctuated<Constraint, Token![,]> =
            vertices_content.parse_terminated(Constraint::parse, Token![,])?;
        content.parse::<Token![,]>()?;
        // Parse edges
        let edges_content;
        syn::bracketed!(edges_content in content);
        let edge_tuples: Punctuated<ConstraintTuple, Token![,]> =
            edges_content.parse_terminated(ConstraintTuple::parse, Token![,])?;
        content.parse::<Token![,]>()?;
        // Parse generic_params
        let params_content;
        syn::bracketed!(params_content in content);
        let generic_param_list: Punctuated<GenericParam, Token![,]> =
            params_content.parse_terminated(GenericParam::parse, Token![,])?;
        // Add vertices
        let mut graph = VecGraph::default();
        for vertex in &vertices {
            graph.add_node(vertex.clone());
        }
        // Add edges
        for edge_tuple in &edge_tuples {
            let from_id = graph
                .node_pairs()
                .find(|(_, v)| **v == edge_tuple.from)
                .map(|(id, _)| id);
            let to_id = graph
                .node_pairs()
                .find(|(_, v)| **v == edge_tuple.to)
                .map(|(id, _)| id);
            let from_id = from_id.unwrap_or_else(|| graph.add_node(edge_tuple.from.clone()));
            let to_id = to_id.unwrap_or_else(|| graph.add_node(edge_tuple.to.clone()));
            graph.add_edge((), from_id, to_id);
        }
        Ok(Solver {
            graph,
            generic_params: generic_param_list.into_iter().collect(),
        })
    }
}

impl ToTokens for Solver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Only emit orphan vertices (vertices with no incoming or outgoing edges)
        let orphan_vertices: Vec<_> = self
            .graph
            .node_pairs()
            .filter(|(node_id, _)| {
                self.graph.incoming_edge_indices(*node_id).count() == 0
                    && self.graph.outgoing_edge_indices(*node_id).count() == 0
            })
            .map(|(_, vertex)| vertex)
            .cloned()
            .collect();

        // Serialize edges as tuples of constraints
        let mut edges = Vec::new();
        for edge_idx in self.graph.edge_indices() {
            let endpoints = self.graph.endpoints(edge_idx);
            let [from_idx, to_idx] = endpoints;
            let from_constraint = self.graph.node(from_idx);
            let to_constraint = self.graph.node(to_idx);
            edges.push((from_constraint.clone(), to_constraint.clone()));
        }

        let generic_params: Vec<_> = self.generic_params.iter().collect();

        let edge_tokens: Vec<_> = edges
            .iter()
            .map(|(from, to)| {
                quote! { (#from, #to) }
            })
            .collect();

        tokens.extend(quote! {
            {
                [#(#orphan_vertices),*],
                [#(#edge_tokens),*],
                [#(#generic_params),*]
            }
        });
    }
}
