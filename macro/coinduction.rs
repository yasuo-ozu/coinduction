use gotgraph::prelude::*;
use proc_macro2::TokenStream;
use std::collections::{HashSet, VecDeque};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::matching::Matching;
use crate::next_step::{next_step, NextStepArgs, NextStepKind};
use crate::solver::{Constraint, Solver};
use crate::{remove_path_args, NoArgPath};

pub struct CoinductionArgs {
    pub paths: Punctuated<NoArgPath, Token![,]>,
    pub coinduction: NoArgPath,
}

impl Parse for CoinductionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let coinduction = crate::try_parse_coinduction_args(input)?;
        let paths: Punctuated<NoArgPath, Token![,]> =
            input.parse_terminated(NoArgPath::parse, Token![,])?;
        Ok(CoinductionArgs { paths, coinduction })
    }
}

pub fn coinduction(module: ItemMod, args: CoinductionArgs) -> TokenStream {
    let (target_impls, other_contents): (Vec<ItemImpl>, Vec<Item>) = module
        .content
        .as_ref()
        .map(|c| &c.1)
        .into_iter()
        .flatten()
        .fold(Default::default(), |(mut impls, mut others), item| {
            match item {
                Item::Impl(item_impl) if item_impl.trait_.is_some() => {
                    impls.push(item_impl.clone())
                }
                other => others.push(other.clone()),
            }
            (impls, others)
        });
    let working_traits: HashSet<_> = if args.paths.len() > 0 {
        args.paths.into_iter().collect()
    } else {
        target_impls
            .iter()
            .filter_map(|ItemImpl { trait_, .. }| trait_.as_ref().map(|t| remove_path_args(&t.1)))
            .collect()
    };
    let rewrite_rules = target_impls
        .iter()
        .filter_map(|item_impl| {
            working_traits
                .contains(&remove_path_args(&item_impl.trait_.as_ref().unwrap().1))
                .then(|| {
                    let mut rules = Vec::new();
                    Constraint::map_generics(&mut item_impl.generics.clone(), |c| {
                        rules.push(c.clone());
                        vec![c]
                    });
                    (
                        item_impl.generics.clone(),
                        Constraint {
                            typ: item_impl.self_ty.as_ref().clone(),
                            trait_path: item_impl.trait_.as_ref().unwrap().1.clone(),
                        },
                        rules,
                    )
                })
        })
        .collect::<Vec<_>>();
    let mut working_list = HashSet::new();
    // Iterate items in the module, and generate Ident list of the struct/enum/unions
    let ignore_tys: HashSet<Ident> = module
        .content
        .as_ref()
        .map(|c| &c.1)
        .into_iter()
        .flatten()
        .filter_map(|item| match item {
            Item::Struct(item_struct) => Some(item_struct.ident.clone()),
            Item::Enum(item_enum) => Some(item_enum.ident.clone()),
            Item::Union(item_union) => Some(item_union.ident.clone()),
            _ => None,
        })
        .collect();
    let solvers = target_impls
        .iter()
        .map(|item_impl| {
            let constraint = Constraint {
                typ: item_impl.self_ty.as_ref().clone(),
                trait_path: item_impl.trait_.as_ref().unwrap().1.clone(),
            };
            if !working_traits.contains(&remove_path_args(&constraint.trait_path)) {
                return None;
            }
            let mut solver = Solver {
                graph: Default::default(),
                generic_params: item_impl.generics.params.iter().cloned().collect(),
            };

            solver.graph.scope_mut(|mut graph| {
                let root_node = graph.add_node(constraint.clone());
                let mut local_working_list = VecDeque::new();
                local_working_list.push_back(root_node);
                let mut iteration_count = 0;
                const MAX_ITERATIONS: usize = 1000;
                while let Some(node_id) = local_working_list.pop_front() {
                    let constraint = graph.node(node_id).clone();
                    iteration_count += 1;
                    if iteration_count > MAX_ITERATIONS {
                        proc_macro_error::abort!(
                            &constraint.trait_path,
                            "Maximum iteration limit reached ({}). Possible infinite loop in coinduction resolution.",
                            MAX_ITERATIONS
                        );
                    }
                    if !working_traits.contains(&remove_path_args(&constraint.trait_path)) {
                        continue;
                    }
                    let unwrapped_typ = crate::unwrap_type_group(constraint.typ.clone());
                    let is_module_type = matches!(&unwrapped_typ, Type::Path(p) if p.path.segments.len() == 1 && ignore_tys.contains(&p.path.segments[0].ident));
                    let is_generic = matches!(&unwrapped_typ, Type::Path(p) if p.path.segments.len() == 1 &&
                        item_impl.generics.params.iter().any(|param|
                            matches!(param, GenericParam::Type(tp) if tp.ident == p.path.segments[0].ident)
                        )
                    );

                    if !is_module_type && !is_generic {
                        working_list.insert(constraint.clone());
                        continue;
                    }

                    for (generics, rule_constraint, rule_constraints) in &rewrite_rules {
                        let params: HashSet<_> = generics.params.iter().cloned().collect();
                        if let Some(substitution) = rule_constraint.matches(&constraint, &params) {
                            for mut new_constraint in rule_constraints.clone() {
                                new_constraint.replace(&substitution);
                                let existing_node = graph
                                    .node_pairs()
                                    .find(|(_, c)| **c == new_constraint)
                                    .map(|(id, _)| id);
                                let new_node_id = if let Some(id) = existing_node {
                                    id
                                } else {
                                    let n = graph.add_node(new_constraint.clone());
                                    local_working_list.push_back(n);
                                    n
                                };
                                graph.add_edge((), node_id, new_node_id);
                            }
                            break;
                        }
                    }
                }
            });
            Some(solver)
        })
        .collect();
    let next_step_args = NextStepArgs {
        kind: NextStepKind::None,
        working_list: working_list.into_iter().collect(),
        coinduction: args.coinduction,
        working_traits: working_traits.into_iter().collect(),
        ignore_tys,
        solvers,
        target_impls,
    };
    let next = next_step(next_step_args);
    quote! {
        #(for attr in &module.attrs) {#attr}
        #{ &module.vis }
        #{ &module.unsafety }
        #{ &module.mod_token }
        #{ &module.ident } {
            #(for content in other_contents) { #content }
            #next
        }
    }
}
