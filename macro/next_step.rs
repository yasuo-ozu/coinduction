use proc_macro2::TokenStream;
use proc_macro_error::abort;
use safegraph::graph::Graph;
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

/// What was learned about the constraint at the front of the working list.
pub enum Step {
    /// The predicates of the matching rule of the trait, and the name of the trait.
    Rules(Vec<PredicateType>, Ident),
    /// The predicates of the impls of the type: their generic parameters, the implemented
    /// constraint and the predicates of the impl.
    Impls(Vec<(HashSet<GenericParam>, Constraint, Vec<Constraint>)>),
}

/// Parse the predicates of a rule: `T: A + B, U: C`.
fn parse_rule_predicates(input: ParseStream) -> syn::Result<Vec<PredicateType>> {
    let mut constraints = Vec::new();
    while !input.is_empty() {
        let pred = input.parse::<WherePredicate>()?;
        if let WherePredicate::Type(pred_type) = pred {
            constraints.push(pred_type);
        } else {
            abort!(pred, "expected type predicate");
        }
        if input.parse::<Token![,]>().is_err() {
            break;
        }
    }
    Ok(constraints)
}

/// The arguments of the `coinduction::impls(...)` META element that a `#[coinduction]` module
/// adds to the announced types: `"<version>", [([<params>], <constraint>, [<constraints>]), ...]`.
pub struct Impls(pub Vec<(HashSet<GenericParam>, Constraint, Vec<Constraint>)>);

impl Parse for Impls {
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
        let predicates_content;
        syn::bracketed!(predicates_content in input);
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
        Ok(Impls(predicates))
    }
}

/// Whether a `META` element of an announced type holds its impls: `coinduction::impls(...)`.
pub fn is_impls_meta(path: &Path) -> bool {
    let n = path.segments.len();
    n >= 2 && path.segments[n - 2].ident == "coinduction" && path.segments[n - 1].ident == "impls"
}

/// The state of the resolution, passed along the macro chain.
pub struct NextStepArgs {
    pub working_list: VecDeque<Constraint>,
    pub(crate) coinduction: NoArgPath,
    pub(crate) working_traits: Vec<NoArgPath>,
    /// Names imported under another name: `alias -> original`.
    pub trait_aliases: Vec<(Ident, Ident)>,
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

        // Parse trait_aliases: `[alias = original, ...]`
        let trait_aliases_content;
        syn::bracketed!(trait_aliases_content in input);
        let mut trait_aliases = Vec::new();
        while !trait_aliases_content.is_empty() {
            let alias: Ident = trait_aliases_content.parse()?;
            trait_aliases_content.parse::<Token![=]>()?;
            let original: Ident = trait_aliases_content.parse()?;
            trait_aliases.push((alias, original));
            if trait_aliases_content.is_empty() {
                break;
            }
            trait_aliases_content.parse::<Token![,]>()?;
        }

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
            working_list,
            coinduction,
            working_traits,
            trait_aliases,
            ignore_tys,
            solvers,
            target_impls,
        })
    }
}

impl ToTokens for NextStepArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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
        let aliases = self
            .trait_aliases
            .iter()
            .map(|(alias, original)| quote!(#alias = #original));

        tokens.extend(quote! {
            #PACKAGE_VERSION,
            [#(#working_list),*],
            {#coinduction},
            [#(#working_traits),*],
            [#(#aliases),*],
            [#(#ignore_tys),*],
            [#(#solver_tokens),*],
            [#(#target_impls),*]
        });
    }
}

/// The name of the trait at `path`: its last segment, or the original name of an alias.
fn trait_name(path: &Path, aliases: &[(Ident, Ident)]) -> Ident {
    let last = &path.segments.last().unwrap().ident;
    if path.leading_colon.is_none() && path.segments.len() == 1 {
        if let Some((_, original)) = aliases.iter().find(|(alias, _)| alias == last) {
            return original.clone();
        }
    }
    last.clone()
}

/// Name the trait at `path`, written by another module (in the rules of a trait or the impls of an
/// announced type), as the module does: replace it by a working trait of the same name, keeping
/// the arguments of `path`. A trait imported with `#[coinduction] use` is a working trait both by
/// its path and by its name; the one in the constraints of the module (`used`) is preferred, then
/// the shorter one. A path from another crate's root (`::core::clone::Clone`) names that trait,
/// not one of the module, so it is renamed only when it is a working trait itself.
fn rename_working_trait(
    path: &mut Path,
    working_traits: &[NoArgPath],
    aliases: &[(Ident, Ident)],
    used: &HashSet<String>,
) {
    let idents = |path: &Path| {
        path.segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
    };
    let segments = idents(path);
    let relative = path.leading_colon.is_none()
        && (segments.len() == 1
            || ["crate", "$crate", "self", "super"].contains(&segments[0].as_str()));
    if !relative && !working_traits.iter().any(|wt| idents(&wt.0) == segments) {
        return;
    }
    let name = &path.segments.last().unwrap().ident;
    let working_trait = working_traits
        .iter()
        .filter(|wt| {
            &wt.0.segments.last().unwrap().ident == name || &trait_name(&wt.0, aliases) == name
        })
        .min_by_key(|wt| {
            let key = crate::comparison_key(*wt);
            (!used.contains(&key), wt.0.segments.len(), key)
        });
    if let Some(working_trait) = working_trait {
        let arguments = path.segments.last().unwrap().arguments.clone();
        *path = working_trait.0.clone();
        path.segments.last_mut().unwrap().arguments = arguments;
    }
}

/// Replace `path` by `to` (keeping the arguments of `path`) when it names `from`.
fn rename_trait(path: &mut Path, from: &NoArgPath, to: &NoArgPath) {
    if crate::comparison_key(&crate::remove_path_args(path)) == crate::comparison_key(from) {
        let arguments = path.segments.last().unwrap().arguments.clone();
        *path = to.0.clone();
        path.segments.last_mut().unwrap().arguments = arguments;
    }
}

/// Record `step`, what was learned about the constraint at the front of the working list, in the
/// solvers, and remove the constraint from the list.
fn apply(args: &mut NextStepArgs, step: Step) {
    if let Some(target) = args.working_list.pop_front() {
        // The type may arrive as a `ty` fragment of a rule matcher, in an invisible group
        let unwrapped_target = Constraint {
            typ: crate::unwrap_type_group(target.typ.clone()),
            trait_path: target.trait_path.clone(),
        };
        let (working_traits, trait_aliases) = (&args.working_traits, &args.trait_aliases);
        for solver in args.solvers.iter_mut().filter_map(Option::as_mut) {
            solver.graph.scope_mut(|mut graph| {
                let root_ix_opt = graph.node_indices().find(|ix| graph.node(*ix) == &target);

                if let Some(root_ix) = root_ix_opt {
                    // The traits as the constraints of the module name them
                    let used_traits = graph
                        .node_indices()
                        .map(|ix| {
                            crate::comparison_key(&crate::remove_path_args(
                                &graph.node(ix).trait_path,
                            ))
                        })
                        .collect::<HashSet<_>>();
                    let used_traits = &used_traits;
                    let dep_constraints = match &step {
                        Step::Rules(appending_constraints, trait_ident) => appending_constraints
                            .iter()
                            .flat_map(|pred| {
                                pred.bounds.iter().map(|bound| {
                                    if let TypeParamBound::Trait(trait_bound) = bound {
                                        // The rules name the traits as the definition of the
                                        // trait does; name them as the target and the module do
                                        let mut trait_path = trait_bound.path.clone();
                                        if &trait_path.segments.last().unwrap().ident == trait_ident {
                                            let arguments =
                                                trait_path.segments.last().unwrap().arguments.clone();
                                            trait_path = crate::remove_path_args(&target.trait_path).0;
                                            trait_path.segments.last_mut().unwrap().arguments = arguments;
                                        } else {
                                            rename_working_trait(
                                                &mut trait_path,
                                                working_traits,
                                                trait_aliases,
                                                used_traits,
                                            );
                                        }
                                        (
                                            Constraint {
                                                typ: pred.bounded_ty.clone(),
                                                trait_path,
                                            },
                                            HashSet::new(),
                                        )
                                    } else {
                                        abort!(bound, "non-trait bounds are not supported")
                                    }
                                })
                            })
                            .collect::<Vec<_>>(),
                        Step::Impls(predicates) => predicates
                            .iter()
                            .map(|(params, replacing, new_constraints)| {
                                let mut replacing = replacing.clone();
                                // The impls name the type relative to its module; name it as the
                                // target does
                                match (&mut replacing.typ, &unwrapped_target.typ) {
                                    (
                                        Type::Path(TypePath { qself: None, path }),
                                        Type::Path(TypePath { qself: None, path: target_path }),
                                    ) if path.leading_colon.is_none() && path.segments.len() == 1 => {
                                        let mut new_path = target_path.clone();
                                        new_path.segments.last_mut().unwrap().arguments =
                                            path.segments.last().unwrap().arguments.clone();
                                        *path = new_path;
                                    }
                                    (replacing_typ, target_typ) => abort!(
                                        target_typ,
                                        "coinduction: cannot match `{}` with the impl for `{}`",
                                        crate::display(target_typ),
                                        crate::display(replacing_typ)
                                    ),
                                }
                                // The impls also name the trait as the module of the type does;
                                // name it as the target does, also in the predicates
                                let impl_trait = crate::remove_path_args(&replacing.trait_path);
                                let target_trait = crate::remove_path_args(&target.trait_path);
                                let impl_trait_ident = &impl_trait.0.segments.last().unwrap().ident;
                                let rename = impl_trait_ident
                                    == &target_trait.0.segments.last().unwrap().ident
                                    || impl_trait_ident
                                        == &trait_name(&target_trait.0, &args.trait_aliases);
                                if rename {
                                    rename_trait(&mut replacing.trait_path, &impl_trait, &target_trait);
                                }
                                replacing.matches(&unwrapped_target, &params).map(|substitute| {
                                    new_constraints.iter().map(move |new_constraint0| {
                                        let mut new_constraint = new_constraint0.clone();
                                        if rename {
                                            rename_trait(&mut new_constraint.trait_path, &impl_trait, &target_trait);
                                        }
                                        // Other traits in the predicates
                                        rename_working_trait(
                                            &mut new_constraint.trait_path,
                                            working_traits,
                                            trait_aliases,
                                            used_traits,
                                        );
                                        new_constraint.replace(&substitute);
                                        (new_constraint, params.clone())
                                    })
                                })
                            })
                            .flatten()
                            .flatten()
                            .collect::<Vec<_>>(),
                    };

                    for (new_constraint, additional_params) in dep_constraints {
                        let existing_ix_opt = graph
                            .node_indices()
                            .find(|ix| graph.node(*ix) == &new_constraint);
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
                                crate::comparison_key(wt)
                                    == crate::comparison_key(&crate::remove_path_args(
                                        &new_constraint.trait_path,
                                    ))
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
                        let target_ix = existing_ix_opt
                            .unwrap_or_else(|| graph.insert_node(new_constraint).unwrap());
                        let edge_exists = graph
                            .edge_indices_from(root_ix)
                            .any(|edge_ix| graph.edge_head_index(edge_ix) == target_ix);
                        if !edge_exists {
                            graph.insert_edge((), [root_ix, target_ix]).unwrap();
                        }
                    }
                }
            });
        }
    }
}

/// Continue the resolution: ask the trait of the constraint at the front of the working list for
/// its definition, or, when the list is empty, emit the rewritten impls.
///
/// The trait is an announced item, so its trampoline (the macro named after it) is called with
/// `__next_step` as the callback and the state as `REST`.
pub fn next_step(args: NextStepArgs) -> TokenStream {
    if let Some(target) = args.working_list.front() {
        let macro_path = crate::remove_path_args(&target.trait_path);
        let coinduction = &args.coinduction;
        quote! {
            #macro_path ! { [#macro_path !, #coinduction::__next_step!] #args }
        }
    } else {
        let mut target_impls = args.target_impls.clone();
        for (impl_item, solver) in target_impls
            .iter_mut()
            .zip(&args.solvers)
            .filter_map(|(item_impl, solver)| solver.as_ref().map(|solver| (item_impl, solver)))
        {
            solver.graph.scope(|graph| {
                let loops = safegraph::algo::connectivity::tarjan_scc(graph)
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
                                    .edge_indices_from(*ix)
                                    .map(|eix| graph.edge_head_index(eix))
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

/// Call the trampoline of the type of `target` with `__next_step` as the callback, or report that
/// nothing describes the type. In a rule matcher, `state` is `$($__coinduction_rest)*`.
fn fetch_type(
    target: &Constraint,
    coinduction: &NoArgPath,
    state: TokenStream,
) -> TokenStream {
    match crate::unwrap_type_group(target.typ.clone()) {
        Type::Path(TypePath { qself: None, path }) => {
            let macro_path = crate::remove_path_args(&path);
            quote! {
                #macro_path ! { [#macro_path !, #coinduction::__next_step!] #state }
            }
        }
        typ => {
            let msg = format!(
                "coinduction: no rule of the trait `{}` matches `{}`, and the type has no announced definition",
                crate::display(&target.trait_path),
                crate::display(&typ),
            );
            syn::Error::new_spanned(&target.typ, msg).to_compile_error()
        }
    }
}

/// `__next_step`: the callback of the macro chain that a `#[coinduction]` module starts.
///
/// The input is either
/// - what an announce trampoline sends: `[LIST] KEY => [META] { ITEM } STATE`, the definition of
///   the trait or the type of the constraint at the front of the working list, or
/// - `@rules TRAIT [PREDICATES] STATE`, from the rule matcher of the trait named `TRAIT`, when a
///   rule matched.
pub fn next_step_macro(input: TokenStream) -> TokenStream {
    struct RulesInput {
        trait_ident: Ident,
        predicates: Vec<PredicateType>,
        state: NextStepArgs,
    }
    impl Parse for RulesInput {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            input.parse::<Token![@]>()?;
            let kw: Ident = input.parse()?;
            if kw != "rules" {
                return Err(syn::Error::new(kw.span(), "expected `@rules`"));
            }
            let trait_ident = input.parse()?;
            let content;
            syn::bracketed!(content in input);
            let predicates = parse_rule_predicates(&content)?;
            Ok(RulesInput {
                trait_ident,
                predicates,
                state: input.parse()?,
            })
        }
    }

    if matches!(input.clone().into_iter().next(), Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == '@')
    {
        let RulesInput {
            trait_ident,
            predicates,
            mut state,
        } = parse2(input).unwrap_or_else(|e| abort!(e.span(), "{}", e));
        apply(&mut state, Step::Rules(predicates, trait_ident));
        return next_step(state);
    }

    let input: announce::helper::Input =
        parse2(input).unwrap_or_else(|e| abort!(e.span(), "{}", e));
    let Some(entry) = input.entries.first() else {
        abort!(
            proc_macro2::Span::call_site(),
            "coinduction: `__next_step` was called without a definition"
        )
    };
    let mut state: NextStepArgs = input
        .rest_as()
        .unwrap_or_else(|e| abort!(e.span(), "{}", e));
    let Some(target) = state.working_list.front().cloned() else {
        abort!(
            proc_macro2::Span::call_site(),
            "coinduction: `__next_step` was called with an empty working list"
        )
    };
    match entry.item() {
        Ok(Item::Trait(item_trait)) => {
            let rules = entry
                .meta
                .iter()
                .filter(|meta| crate::rules::is_rules_meta(&meta.path))
                .map(|meta| {
                    meta.parse_args_with(crate::rules::Rules::parse)
                        .unwrap_or_else(|e| abort!(e.span(), "invalid `coinduction::rules`: {}", e))
                })
                .flat_map(|rules| rules.rules)
                .collect::<Vec<_>>();
            if rules.is_empty() {
                fetch_type(&target, &state.coinduction, quote!(#state))
            } else {
                let fallback = fetch_type(
                    &target,
                    &state.coinduction,
                    quote!($($__coinduction_rest)*),
                );
                crate::rules::matcher(
                    &item_trait.ident,
                    &crate::rules::Rules { rules },
                    &crate::unwrap_type_group(target.typ.clone()),
                    fallback,
                    &state.coinduction,
                    quote!(#state),
                )
            }
        }
        Ok(Item::Struct(_) | Item::Enum(_) | Item::Union(_)) => {
            let Some(meta) = entry
                .meta
                .iter()
                .find(|meta| is_impls_meta(&meta.path))
            else {
                abort!(
                    target.typ,
                    "coinduction: `{}` is announced, but not in a `#[coinduction]` module, so its impls are unknown",
                    crate::display(&target.typ);
                    help = "move the type into a `#[coinduction]` module, or give the trait a rule for it with `#[announce(meta = coinduction::rules(...))]`"
                )
            };
            let Impls(predicates) = meta
                .parse_args_with(Impls::parse)
                .unwrap_or_else(|e| abort!(e.span(), "{}", e));
            apply(&mut state, Step::Impls(predicates));
            next_step(state)
        }
        Ok(item) => abort!(
            item,
            "coinduction: expected the definition of a trait, struct, enum or union"
        ),
        Err(e) => abort!(e.span(), "{}", e),
    }
}
