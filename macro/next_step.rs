use gotgraph::prelude::*;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::{quote, ToTokens};

use crate::{
    matching::Matching,
    solver::{Constraint, Solver},
    NoArgPath,
};

const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, PartialEq)]
pub enum NextStepKind {
    Traitdef {
        appending_constraints: Vec<PredicateType>,
    },
    Typedef {
        predicates: Vec<(HashSet<GenericParam>, Constraint, Vec<Constraint>)>,
    },
    None,
}

impl Parse for NextStepKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        mod kw {
            syn::custom_keyword!(appending_constraints);
            syn::custom_keyword!(predicates);
        }

        let ident: syn::Ident = input.parse()?;

        match ident.to_string().as_str() {
            "Traitdef" => {
                let content;
                syn::braced!(content in input);

                content.parse::<kw::appending_constraints>()?;
                content.parse::<Token![:]>()?;
                let constraints_content;
                syn::bracketed!(constraints_content in content);
                let mut constraints = Vec::new();
                while !constraints_content.is_empty() {
                    let pred = constraints_content.parse::<WherePredicate>()?;
                    if let WherePredicate::Type(pred_type) = pred {
                        constraints.push(pred_type);
                    } else {
                        abort!(pred, "expected type predicate");
                    }
                    if constraints_content.parse::<Token![,]>().is_err() {
                        break;
                    }
                }

                Ok(NextStepKind::Traitdef {
                    appending_constraints: constraints,
                })
            }
            "Typedef" => {
                let content;
                syn::braced!(content in input);
                content.parse::<kw::predicates>()?;
                content.parse::<Token![:]>()?;
                let predicates_content;
                syn::bracketed!(predicates_content in content);
                let mut predicates = Vec::new();
                while !predicates_content.is_empty() {
                    let tuple_content;
                    syn::parenthesized!(tuple_content in predicates_content);

                    // Parse HashSet<GenericParam>
                    let params_content;
                    syn::bracketed!(params_content in tuple_content);
                    let params: Punctuated<GenericParam, Token![,]> =
                        params_content.parse_terminated(GenericParam::parse, Token![,])?;
                    let param_set: HashSet<GenericParam> = params.into_iter().collect();

                    tuple_content.parse::<Token![,]>()?;

                    // Parse Constraint
                    let constraint = tuple_content.parse::<Constraint>()?;
                    tuple_content.parse::<Token![,]>()?;

                    // Parse Vec<Constraint>
                    let vec_content;
                    syn::bracketed!(vec_content in tuple_content);
                    let constraints: Punctuated<Constraint, Token![,]> =
                        vec_content.parse_terminated(Constraint::parse, Token![,])?;

                    predicates.push((param_set, constraint, constraints.into_iter().collect()));
                    if predicates_content.parse::<Token![,]>().is_err() {
                        break;
                    }
                }
                Ok(NextStepKind::Typedef { predicates })
            }
            "None" => Ok(NextStepKind::None),
            _ => Err(syn::Error::new_spanned(ident, "Invalid NextStepKind")),
        }
    }
}

impl ToTokens for NextStepKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            NextStepKind::Traitdef {
                appending_constraints,
            } => {
                tokens.extend(quote! {
                    Traitdef {
                        appending_constraints: [#(#appending_constraints),*]
                    }
                });
            }
            NextStepKind::Typedef { predicates } => {
                let predicate_tokens: Vec<_> = predicates
                    .iter()
                    .map(|(params, c, cs)| {
                        let param_tokens: Vec<_> = params.iter().collect();
                        quote! { ([#(#param_tokens),*], #c, [#(#cs),*]) }
                    })
                    .collect();
                tokens.extend(quote! {
                    Typedef {
                        predicates: [#(#predicate_tokens),*]
                    }
                });
            }
            NextStepKind::None => {
                tokens.extend(quote! { None });
            }
        }
    }
}

pub struct NextStepArgs {
    pub kind: NextStepKind,
    pub working_list: VecDeque<Constraint>,
    pub coinduction: NoArgPath,
    pub working_traits: Vec<NoArgPath>,
    pub ignore_tys: HashSet<Ident>,
    pub solvers: Vec<Option<Solver>>,
    pub target_impls: Vec<ItemImpl>,
}

impl Parse for NextStepArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let version: LitStr = input.parse()?;

        if version.value() != PACKAGE_VERSION {
            abort!(
                version,
                "version mismatch: expected '{}', found '{}'",
                PACKAGE_VERSION,
                version.value()
            );
        }

        input.parse::<Token![,]>()?;

        // Parse kind
        let kind: NextStepKind = input.parse()?;

        input.parse::<Token![,]>()?;

        // Parse working_list
        let working_list_content;
        syn::bracketed!(working_list_content in input);
        let working_list_vec: Punctuated<Constraint, Token![,]> =
            working_list_content.parse_terminated(Constraint::parse, Token![,])?;
        let working_list: VecDeque<Constraint> = working_list_vec.into_iter().collect();

        input.parse::<Token![,]>()?;

        // Parse coinduction surrounded by braces
        let coinduction_content;
        syn::braced!(coinduction_content in input);
        let coinduction: NoArgPath = coinduction_content.parse()?;

        input.parse::<Token![,]>()?;

        // Parse working_traits
        let working_traits_content;
        syn::bracketed!(working_traits_content in input);
        let working_traits_vec: Punctuated<NoArgPath, Token![,]> =
            working_traits_content.parse_terminated(NoArgPath::parse, Token![,])?;
        let working_traits: Vec<NoArgPath> = working_traits_vec.into_iter().collect();

        input.parse::<Token![,]>()?;

        // Parse ignore_tys
        let ignore_tys_content;
        syn::bracketed!(ignore_tys_content in input);
        let ignore_tys_vec: Punctuated<Ident, Token![,]> =
            ignore_tys_content.parse_terminated(Ident::parse, Token![,])?;
        let ignore_tys: HashSet<Ident> = ignore_tys_vec.into_iter().collect();

        input.parse::<Token![,]>()?;

        // Parse solvers
        let solvers_content;
        syn::bracketed!(solvers_content in input);
        let mut solvers = Vec::new();
        while !solvers_content.is_empty() {
            if solvers_content.peek(syn::token::Brace) {
                // Parse Some(Solver)
                let solver = solvers_content.parse::<Solver>()?;
                solvers.push(Some(solver));
            } else if solvers_content.peek(syn::Ident) {
                // Check for None
                let ident: syn::Ident = solvers_content.parse()?;
                if ident == "None" {
                    solvers.push(None);
                } else {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "Expected 'None' or a Solver",
                    ));
                }
            } else {
                return Err(solvers_content.error("Expected Solver or None"));
            }

            if solvers_content.peek(Token![,]) {
                solvers_content.parse::<Token![,]>()?;
            }
        }

        input.parse::<Token![,]>()?;

        // Parse target_impls
        let target_impls_bracket;
        syn::bracketed!(target_impls_bracket in input);
        let target_impls: Punctuated<ItemImpl, Token![,]> =
            target_impls_bracket.parse_terminated(ItemImpl::parse, Token![,])?;
        let target_impls: Vec<ItemImpl> = target_impls.into_iter().collect();

        Ok(NextStepArgs {
            kind,
            working_list,
            coinduction,
            working_traits,
            ignore_tys,
            solvers,
            target_impls,
        })
    }
}

impl ToTokens for NextStepArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let kind = &self.kind;
        let working_list: Vec<_> = self.working_list.iter().collect();
        let working_traits: Vec<_> = self.working_traits.iter().collect();
        let ignore_tys: Vec<_> = self.ignore_tys.iter().collect();
        let solver_tokens: Vec<_> = self
            .solvers
            .iter()
            .map(|solver_opt| match solver_opt {
                Some(solver) => quote! { #solver },
                None => quote! { None },
            })
            .collect();
        let coinduction = &self.coinduction;
        let target_impls = &self.target_impls;

        tokens.extend(quote! {
            #PACKAGE_VERSION,
            #kind,
            [#(#working_list),*],
            {#coinduction},
            [#(#working_traits),*],
            [#(#ignore_tys),*],
            [#(#solver_tokens),*],
            [#(#target_impls),*]
        });
    }
}

pub fn next_step(mut args: NextStepArgs) -> TokenStream {
    if let Some(Some(target)) =
        (&args.kind != &NextStepKind::None).then(|| args.working_list.pop_front())
    {
        for solver in args.solvers.iter_mut().filter_map(Option::as_mut) {
            solver.graph.scope_mut(|mut graph| {
                let root_ix_opt = graph
                    .node_pairs()
                    .find(|(_, node)| {
                        template_quote::quote!(#node).to_string()
                            == template_quote::quote!(#target).to_string()
                    })
                    .map(|(ix, _)| ix);

                if let Some(root_ix) = root_ix_opt {
                    let dep_constraints = match &args.kind {
                        NextStepKind::Traitdef {
                            appending_constraints,
                        } => appending_constraints
                            .iter()
                            .flat_map(|pred| {
                                pred.bounds.iter().map(|bound| {
                                    if let TypeParamBound::Trait(trait_bound) = bound {
                                        (
                                            Constraint {
                                                typ: pred.bounded_ty.clone(),
                                                trait_path: trait_bound.path.clone(),
                                            },
                                            HashSet::new(),
                                        )
                                    } else {
                                        abort!(bound, "non-trait bounds are not supported")
                                    }
                                })
                            })
                            .collect::<Vec<_>>(),
                        NextStepKind::Typedef { predicates } => predicates
                            .iter()
                            .map(|(params, replacing, new_constraints)| {
                                let mut replacing = replacing.clone();
                                match (&mut replacing.typ, &target.typ) {
                                    (
                                        Type::Path(TypePath { qself: None, path: Path { leading_colon: None, segments } }),
                                        Type::Path(TypePath{ path: Path { segments: target_segments, ..}, ..})
                                    ) if segments.len() == 1 => {
                                        let mut new_segments = target_segments.clone();
                                        new_segments.last_mut().unwrap().arguments = segments.last().unwrap().arguments.clone();
                                        *segments = new_segments
                                    },
                                    _ => unreachable!(),
                                }
                                if &replacing.trait_path.segments.last().unwrap().ident == &target.trait_path.segments.last().unwrap().ident {
                                    let mut new_path = target.trait_path.clone();
                                    new_path.segments.last_mut().unwrap().arguments = replacing.trait_path.segments.last().unwrap().arguments.clone();
                                }
                                replacing.matches(&target, &params).map(|substitute| {
                                    new_constraints.iter().map(move |new_constraint0| {
                                        let mut new_constraint = new_constraint0.clone();
                                        new_constraint.replace(&substitute);
                                        (new_constraint, params.clone())
                                    })
                                })
                            })
                            .flatten()
                            .flatten()
                            .collect::<Vec<_>>(),
                        NextStepKind::None => unreachable!(),
                    };

                    for (new_constraint, additional_params) in dep_constraints {
                        let existing_ix_opt = graph
                            .node_pairs()
                            .find(|(_, c)| *c == &new_constraint)
                            .map(|(ix, _)| ix);
                        if existing_ix_opt.is_none() {
                            let not_in_working_list = !args.working_list.contains(&new_constraint);

                            // Check if the type contains any generic parameters
                            let typ_str =
                                template_quote::quote!(#{&new_constraint.typ}).to_string();
                            let is_generic =
                                solver.generic_params.iter().chain(&additional_params).any(
                                    |param| {
                                        if let GenericParam::Type(tp) = param {
                                            let param_str = template_quote::quote!(#tp).to_string();
                                            &typ_str == &param_str
                                        } else {
                                            false
                                        }
                                    },
                                );

                            let trait_in_working_traits = args.working_traits.iter().any(|wt| {
                                wt == &crate::remove_path_args(&new_constraint.trait_path)
                            });

                            let is_ignored = matches!(
                                crate::unwrap_type_group(new_constraint.typ.clone()),
                                Type::Path(TypePath { qself: None, path })
                                if path.segments.len() == 1 && args.ignore_tys.contains(&path.segments[0].ident)
                            );

                            if not_in_working_list
                                && !is_generic
                                && trait_in_working_traits
                                && !is_ignored
                            {
                                args.working_list.push_back(new_constraint.clone());
                            }
                        }
                        let target_ix =
                            existing_ix_opt.unwrap_or_else(|| graph.add_node(new_constraint));
                        let edge_exists = graph
                            .outgoing_edge_indices(root_ix)
                            .any(|edge_ix| graph.endpoints(edge_ix)[1] == target_ix);
                        if !edge_exists {
                            graph.add_edge((), root_ix, target_ix);
                        }
                    }
                }
            });
        }
    }
    if let Some(target) = args.working_list.front() {
        args.kind = NextStepKind::None;
        let macro_path = crate::remove_path_args(&target.trait_path);
        quote! {
            #macro_path ! { #args }
        }
    } else {
        let mut target_impls = args.target_impls.clone();
        for (impl_item, solver) in target_impls
            .iter_mut()
            .zip(&args.solvers)
            .filter_map(|(item_impl, solver)| solver.as_ref().map(|solver| (item_impl, solver)))
        {
            solver.graph.scope(|graph| {
                let loops = gotgraph::algo::tarjan(graph)
                    .filter_map(|lp| {
                        (lp.len() > 1).then_some(
                            lp.iter()
                                .map(|ix| (graph.node(*ix), *ix))
                                .collect::<HashMap<_, _>>(),
                        )
                    })
                    .collect::<Vec<_>>();
                Constraint::map_generics(&mut impl_item.generics, |constraint| {
                    if let Some(the_loop) = loops.iter().find(|lp| lp.contains_key(&constraint)) {
                        let dependencies = the_loop
                            .values()
                            .map(|ix| {
                                graph
                                    .outgoing_edge_indices(*ix)
                                    .map(|eix| graph.endpoints(eix)[1])
                            })
                            .flatten()
                            .collect::<HashSet<_>>();
                        dependencies
                            .difference(&the_loop.values().cloned().collect())
                            .map(|ix| graph.node(*ix).clone())
                            .collect()
                    } else {
                        vec![constraint]
                    }
                });
            });
        }
        quote! {
            #(for content in target_impls) {
                #content
            }
        }
    }
}
