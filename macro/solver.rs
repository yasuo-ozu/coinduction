use proc_macro_error::abort;
use safegraph::graph::Graph;
use safegraph::VecGraph;
use std::collections::{HashSet, VecDeque};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::{quote, ToTokens};

use crate::matching::Matching;
use crate::NoArgPath;

#[derive(Clone, Debug)]
pub struct Constraint {
    pub typ: Type,
    pub trait_path: Path,
}

impl PartialEq for Constraint {
    fn eq(&self, other: &Self) -> bool {
        crate::comparison_key(self) == crate::comparison_key(other)
    }
}

impl Eq for Constraint {}

impl std::hash::Hash for Constraint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        crate::comparison_key(self).hash(state);
    }
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
                    let additional_predicates =
                        Self::map_bounds(bounds, bounded_ty, lifetimes.as_ref(), &mut f);
                    let has_bounds = !bounds.is_empty();
                    if has_bounds {
                        wc.predicates.extend(core::iter::once(pair));
                    }
                    wc.predicates.extend(additional_predicates);
                }
                _ => wc.predicates.extend(core::iter::once(pair)),
            }
        }
    }

    fn map_bounds(
        bounds: &mut Punctuated<TypeParamBound, Token![+]>,
        bounded_ty: &Type,
        lifetimes: Option<&BoundLifetimes>,
        mut f: impl FnMut(Self) -> Vec<Self>,
    ) -> Vec<WherePredicate> {
        let mut additional_predicates = Vec::new();
        for bound in core::mem::take(bounds).into_pairs() {
            let punct = bound.punct().cloned();
            match bound.into_value() {
                TypeParamBound::Trait(TraitBound {
                    modifier,
                    lifetimes: bound_lifetimes,
                    path,
                    ..
                }) => {
                    if &modifier != &TraitBoundModifier::None {
                        abort!(&modifier, "trait bound modifier is not supported");
                    }
                    for replacing in f(Constraint {
                        typ: bounded_ty.clone(),
                        trait_path: path,
                    }) {
                        let new_bound = TypeParamBound::Trait(TraitBound {
                            paren_token: None,
                            modifier,
                            lifetimes: bound_lifetimes.clone(),
                            path: replacing.trait_path,
                        });
                        if crate::unwrap_type_group(replacing.typ.clone())
                            == crate::unwrap_type_group(bounded_ty.clone())
                        {
                            bounds.push(new_bound);
                            if let Some(punct) = punct.clone() {
                                bounds.push_punct(punct);
                            }
                        } else {
                            additional_predicates.push(WherePredicate::Type(PredicateType {
                                lifetimes: lifetimes.cloned(),
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
                Self::map_bounds(bounds, &bounded_ty, None, f)
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
        // Skip optional `for<...>` syntax if present
        if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            input.parse::<Token![<]>()?;
            let mut _parsed = Punctuated::new();
            while !input.peek(Token![>]) {
                _parsed.push_value(input.parse::<GenericParam>()?);
                if input.peek(Token![,]) {
                    _parsed.push_punct(input.parse::<Token![,]>()?);
                } else {
                    break;
                }
            }
            input.parse::<Token![>]>()?;
        }

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

/// The constructor of `typ` and its type arguments: `Wrap < _ >` and `[T]` for `Wrap<T>`.
fn split_type(typ: &Type) -> (String, Vec<Type>) {
    let mut typ = crate::unwrap_type_group(typ.clone());
    let mut args = Vec::new();
    let mut take = |t: &mut Type| args.push(core::mem::replace(t, parse_quote!(_)));
    match &mut typ {
        Type::Path(TypePath { qself: None, path }) => {
            for segment in &mut path.segments {
                if let PathArguments::AngleBracketed(arguments) = &mut segment.arguments {
                    for argument in &mut arguments.args {
                        if let GenericArgument::Type(t) = argument {
                            take(t);
                        }
                    }
                }
            }
        }
        Type::Tuple(TypeTuple { elems, .. }) => elems.iter_mut().for_each(&mut take),
        Type::Reference(TypeReference { elem, .. })
        | Type::Array(TypeArray { elem, .. })
        | Type::Slice(TypeSlice { elem, .. })
        | Type::Ptr(TypePtr { elem, .. }) => take(elem),
        _ => (),
    }
    (quote!(#typ).to_string(), args)
}

/// Whether `big` is the type `small` grown: the same constructor, with each type argument of
/// `small` embedded in the one of `big`. `G<Wrap<T>>` grows from `G<T>`.
fn grows(small: &Type, big: &Type) -> bool {
    let ((small_head, small_args), (big_head, big_args)) = (split_type(small), split_type(big));
    small_head == big_head
        && small_args.len() == big_args.len()
        && small_args
            .iter()
            .zip(&big_args)
            .all(|(s, b)| embedded(s, b))
}

/// Whether `big` is `small`, possibly grown, with types around it: `T` in `Wrap<T>`.
fn embedded(small: &Type, big: &Type) -> bool {
    grows(small, big) || split_type(big).1.iter().any(|arg| embedded(small, arg))
}

/// An impl of the module: its generics, the constraint it implements and its predicates.
pub type Rule = (Generics, Constraint, Vec<Constraint>);

/// The impls of the module for the traits it works on.
pub(crate) fn rules(impls: &[ItemImpl], is_working: impl Fn(&NoArgPath) -> bool) -> Vec<Rule> {
    impls
        .iter()
        .filter_map(|item_impl| {
            let trait_path = &item_impl.trait_.as_ref().unwrap().1;
            is_working(&crate::remove_path_args(trait_path)).then(|| {
                let mut predicates = Vec::new();
                Constraint::map_generics(&mut item_impl.generics.clone(), |c| {
                    predicates.push(c.clone());
                    vec![c]
                });
                (
                    item_impl.generics.clone(),
                    Constraint {
                        typ: item_impl.self_ty.as_ref().clone(),
                        trait_path: trait_path.clone(),
                    },
                    predicates,
                )
            })
        })
        .collect()
}

impl Solver {
    /// Follow the impls of the module from the constraints `from`, as long as their types are
    /// types of the module (`module_tys`) or generic parameters of the impl. Return the constraints
    /// on other types reached on the way, which the macros of their traits answer.
    pub(crate) fn expand(
        &mut self,
        from: &[Constraint],
        rules: &[Rule],
        module_tys: &HashSet<Ident>,
        is_working: impl Fn(&NoArgPath) -> bool,
    ) -> Vec<Constraint> {
        let generic_params = &self.generic_params;
        let mut external = Vec::new();
        self.graph.scope_mut(|mut graph| {
            let mut local_working_list = VecDeque::new();
            for constraint in from {
                let existing_node = graph.node_indices().find(|ix| graph.node(*ix) == constraint);
                local_working_list
                    .push_back(existing_node.unwrap_or_else(|| graph.insert_node(constraint.clone()).unwrap()));
            }
            let mut iteration_count = 0;
            const MAX_ITERATIONS: usize = 1000;
            const MAX_GROWTH: usize = 8;
            while let Some(node_id) = local_working_list.pop_front() {
                let constraint = graph.node(node_id).clone();
                iteration_count += 1;
                if iteration_count > MAX_ITERATIONS {
                    abort!(
                        &constraint.trait_path,
                        "Maximum iteration limit reached ({}). Possible infinite loop in coinduction resolution.",
                        MAX_ITERATIONS
                    );
                }
                if !is_working(&crate::remove_path_args(&constraint.trait_path)) {
                    continue;
                }
                let unwrapped_typ = crate::unwrap_type_group(constraint.typ.clone());
                let is_module_type = matches!(&unwrapped_typ, Type::Path(p) if p.path.segments.len() == 1 && module_tys.contains(&p.path.segments[0].ident));
                let is_generic = matches!(&unwrapped_typ, Type::Path(p) if p.path.segments.len() == 1 &&
                    generic_params.iter().any(|param|
                        matches!(param, GenericParam::Type(tp) if tp.ident == p.path.segments[0].ident)
                    )
                );

                if !is_module_type && !is_generic {
                    if !external.contains(&constraint) {
                        external.push(constraint);
                    }
                    continue;
                }
                // A type of the module that comes back grown each time never closes a loop
                if is_module_type {
                    let trait_key = quote!(#{&constraint.trait_path}).to_string();
                    let grown = graph
                        .node_indices()
                        .map(|ix| graph.node(ix))
                        .filter(|node| {
                            *node != &constraint
                                && quote!(#{&node.trait_path}).to_string() == trait_key
                                && grows(&node.typ, &unwrapped_typ)
                        })
                        .count();
                    if grown >= MAX_GROWTH {
                        abort!(
                            &constraint.typ,
                            "coinduction: `{}` grows without end through the impls",
                            quote!(#constraint)
                        );
                    }
                }

                // The type of an answer may arrive in an invisible group
                let unwrapped = Constraint {
                    typ: unwrapped_typ,
                    trait_path: constraint.trait_path.clone(),
                };
                for (generics, rule_constraint, rule_constraints) in rules {
                    let params: HashSet<_> = generics.params.iter().cloned().collect();
                    if let Some(substitution) = rule_constraint.matches(&unwrapped, &params) {
                        for mut new_constraint in rule_constraints.clone() {
                            new_constraint.replace(&substitution);
                            let existing_node = graph
                                .node_indices()
                                .find(|id| *graph.node(*id) == new_constraint);
                            let new_node_id = if let Some(id) = existing_node {
                                id
                            } else {
                                let n = graph.insert_node(new_constraint.clone()).unwrap();
                                local_working_list.push_back(n);
                                n
                            };
                            graph.insert_edge((), [node_id, new_node_id]).unwrap();
                        }
                        break;
                    }
                }
            }
        });
        external
    }
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
        graph.scope_mut(|mut graph| {
            for vertex in &vertices {
                graph.insert_node(vertex.clone()).unwrap();
            }
            // Add edges
            for edge_tuple in &edge_tuples {
                let from_id = graph
                    .node_indices()
                    .find(|id| *graph.node(*id) == edge_tuple.from);
                let to_id = graph
                    .node_indices()
                    .find(|id| *graph.node(*id) == edge_tuple.to);
                let from_id =
                    from_id.unwrap_or_else(|| graph.insert_node(edge_tuple.from.clone()).unwrap());
                let to_id =
                    to_id.unwrap_or_else(|| graph.insert_node(edge_tuple.to.clone()).unwrap());
                graph.insert_edge((), [from_id, to_id]).unwrap();
            }
        });
        Ok(Solver {
            graph,
            generic_params: generic_param_list.into_iter().collect(),
        })
    }
}

impl ToTokens for Solver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Only emit orphan vertices (vertices with no incoming or outgoing edges)
        let (orphan_vertices, edges) = self.graph.scope(|graph| {
            let orphan_vertices: Vec<_> = graph
                .node_indices()
                .filter(|node_id| {
                    graph.edge_indices_to(*node_id).count() == 0
                        && graph.edge_indices_from(*node_id).count() == 0
                })
                .map(|node_id| graph.node(node_id).clone())
                .collect();

            // Serialize edges as tuples of constraints
            let edges: Vec<_> = graph
                .edge_indices()
                .map(|edge_idx| {
                    (
                        graph.edge_tail(edge_idx).clone(),
                        graph.edge_head(edge_idx).clone(),
                    )
                })
                .collect();
            (orphan_vertices, edges)
        });

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
