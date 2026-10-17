use proc_macro2::TokenStream;
use proc_macro_error::abort;
use safegraph::graph::Graph;
use std::collections::{HashSet, VecDeque};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::matching::Matching;
use crate::next_step::{next_step, NextStepArgs};
use crate::solver::{Constraint, Solver};
use crate::{remove_path_args, NoArgPath};

/// Remove a `#[coinduction]` attribute from `attrs` and return it.
fn take_coinduction_attr(attrs: &mut Vec<Attribute>) -> Option<Attribute> {
    let ix = attrs.iter().position(|attr| {
        attr.path()
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "coinduction")
    })?;
    let attr = attrs.remove(ix);
    if let Some(dup) = attrs.iter().find(|attr| {
        attr.path()
            .segments
            .last()
            .map_or(false, |seg| seg.ident == "coinduction")
    }) {
        abort!(dup, "duplicate `#[coinduction]` attribute");
    }
    Some(attr)
}

/// Whether `attr` is `#[announce]` / `#[announce(...)]` (also with a path, as
/// `#[announce::announce]`): the attribute, or the helper attribute of `#[derive(Announce)]`.
fn is_announce_attr(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .map_or(false, |seg| seg.ident == "announce")
}

/// Whether the item is announced: it has `#[announce]`, or derives `Announce`.
fn is_announced(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        is_announce_attr(attr)
            || (attr.path().is_ident("derive")
                && attr
                    .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                    .map_or(false, |paths| {
                        paths.iter().any(|path| {
                            path.segments
                                .last()
                                .map_or(false, |seg| seg.ident == "Announce")
                        })
                    }))
    })
}

/// Add `meta = <meta>` to the options of the announcement of an item: to its `#[announce(...)]`
/// attribute, or, for `#[derive(Announce)]` without one, in a new helper attribute.
fn add_announce_meta(attrs: &mut Vec<Attribute>, meta: &TokenStream) {
    let Some(attr) = attrs.iter_mut().find(|attr| is_announce_attr(attr)) else {
        attrs.push(parse_quote!(#[announce(meta = #meta)]));
        return;
    };
    let path = attr.path().clone();
    let options = match &attr.meta {
        Meta::Path(_) => TokenStream::new(),
        Meta::List(list) => {
            let mut options = list.tokens.clone();
            if !options.is_empty()
                && !matches!(options.clone().into_iter().last(), Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == ',')
            {
                options.extend(quote!(,));
            }
            options
        }
        Meta::NameValue(_) => abort!(attr, "expected `#[announce]` or `#[announce(...)]`"),
    };
    attr.meta = parse_quote!(#path(#options meta = #meta));
}

/// Collect the names that a `use` item imports under another name (`alias -> original`), and the
/// names it imports as they are.
fn collect_use_aliases(tree: &UseTree, aliases: &mut Vec<(Ident, Ident)>, names: &mut Vec<Ident>) {
    match tree {
        UseTree::Path(UsePath { tree, .. }) => collect_use_aliases(tree, aliases, names),
        UseTree::Name(UseName { ident }) => names.push(ident.clone()),
        UseTree::Rename(UseRename { ident, rename, .. }) if rename != "_" && ident != "self" => {
            aliases.push((rename.clone(), ident.clone()));
        }
        UseTree::Group(UseGroup { items, .. }) => {
            for tree in items {
                collect_use_aliases(tree, aliases, names);
            }
        }
        _ => {}
    }
}

/// Collect the traits imported by a `#[coinduction] use ...;` item, both by the name they are
/// imported as and by their full path.
fn collect_use_traits(tree: &UseTree, prefix: &mut Path, out: &mut HashSet<NoArgPath>) {
    match tree {
        UseTree::Path(UsePath { ident, tree, .. }) => {
            prefix.segments.push(ident.clone().into());
            collect_use_traits(tree, prefix, out);
            prefix.segments.pop();
        }
        UseTree::Name(UseName { ident }) | UseTree::Rename(UseRename { ident, .. })
            if ident == "self" =>
        {
            abort!(
                ident,
                "`self` imports cannot be marked with `#[coinduction]`"
            )
        }
        UseTree::Name(UseName { ident }) => {
            let mut full = prefix.clone();
            full.segments.push(ident.clone().into());
            out.insert(NoArgPath(full));
            out.insert(NoArgPath(ident.clone().into()));
        }
        UseTree::Rename(UseRename { ident, rename, .. }) => {
            let mut full = prefix.clone();
            full.segments.push(ident.clone().into());
            out.insert(NoArgPath(full));
            out.insert(NoArgPath(rename.clone().into()));
        }
        UseTree::Glob(glob) => {
            abort!(glob, "glob imports cannot be marked with `#[coinduction]`")
        }
        UseTree::Group(UseGroup { items, .. }) => {
            for tree in items {
                collect_use_traits(tree, prefix, out);
            }
        }
    }
}

mod kw {
    syn::custom_keyword!(marker);
    syn::custom_keyword!(coinduction);
}

/// Arguments of `#[coinduction(...)]` on a module: `coinduction = <path>`.
pub struct ModuleArgs {
    coinduction: NoArgPath,
}

impl Parse for ModuleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut coinduction = None;
        while !input.is_empty() {
            if input.peek(kw::coinduction) && input.peek2(Token![=]) && coinduction.is_none() {
                input.parse::<kw::coinduction>()?;
                input.parse::<Token![=]>()?;
                coinduction = Some(input.parse()?);
            } else if input.peek(kw::marker) {
                return Err(input.error(
                    "`marker` is no longer needed: the impls of announced types are announced as written",
                ));
            } else {
                return Err(input.error(
                    "expected `coinduction = <path>`; mark traits with `#[coinduction] use ...;` inside the module, or announce them there with `#[announce] trait ...`",
                ));
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(ModuleArgs {
            coinduction: coinduction.unwrap_or_else(|| NoArgPath(parse_quote!(::coinduction))),
        })
    }
}

/// Whether `attrs` has a `#[cfg(...)]` attribute.
fn has_cfg(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("cfg"))
}

/// Evaluate the `#[cfg(...)]` attributes of the impls in `module`, and apply `#[coinduction]`
/// again to the module with the impls that exist. An attribute macro sees the impls that a `cfg`
/// removes, and they would take part in the resolution and the announced impls.
///
/// The `cfg`s are evaluated by a chain of `macro_rules!`, one for each conditional impl: the
/// definition under `#[cfg(P)]` adds the impl to the list it passes on, and the one under
/// `#[cfg(not(P))]` drops it. The last one emits the module. The items are passed as input, so
/// that `$` in them is not read by the chain.
fn eval_cfg_impls(module: ItemMod, args: &ModuleArgs) -> TokenStream {
    let random = crate::get_random();
    let name = |ix: usize| {
        Ident::new(
            &format!("__coinduction_cfg_{}_{}_{}", module.ident, ix, random),
            proc_macro2::Span::call_site(),
        )
    };
    let (inner_attrs, outer_attrs): (Vec<_>, Vec<_>) = module
        .attrs
        .iter()
        .partition(|attr| matches!(attr.style, AttrStyle::Inner(_)));
    let (conditional, items): (Vec<_>, Vec<_>) = module
        .content
        .as_ref()
        .map_or(&[][..], |(_, items)| items)
        .iter()
        .cloned()
        .partition(|item| matches!(item, Item::Impl(item_impl) if has_cfg(&item_impl.attrs)));
    let conditional = conditional
        .into_iter()
        .map(|item| {
            let Item::Impl(mut item_impl) = item else {
                unreachable!()
            };
            let (cfgs, attrs) = item_impl
                .attrs
                .into_iter()
                .partition::<Vec<_>, _>(|attr| attr.path().is_ident("cfg"));
            item_impl.attrs = attrs;
            let cfgs = cfgs
                .iter()
                .map(|attr| {
                    attr.meta
                        .require_list()
                        .map(|list| list.tokens.clone())
                        .unwrap_or_else(|e| abort!(e.span(), "{}", e))
                })
                .collect::<Vec<_>>();
            (quote!(all(#(#cfgs),*)), item_impl)
        })
        .collect::<Vec<_>>();
    let coinduction = &args.coinduction;
    let last = name(conditional.len());
    quote! {
        #(for (ix, (cfg, _)) in conditional.iter().enumerate()) {
            #[cfg(#cfg)]
            macro_rules! #{name(ix)} {
                (
                    [$($__coinduction_impls:tt)*] {$($__coinduction_items:tt)*}
                    {$($__coinduction_impl:tt)*} $($__coinduction_rest:tt)*
                ) => {
                    #{name(ix + 1)}! {
                        [$($__coinduction_impls)* $($__coinduction_impl)*]
                        {$($__coinduction_items)*} $($__coinduction_rest)*
                    }
                };
            }
            #[cfg(not(#cfg))]
            macro_rules! #{name(ix)} {
                (
                    [$($__coinduction_impls:tt)*] {$($__coinduction_items:tt)*}
                    {$($__coinduction_impl:tt)*} $($__coinduction_rest:tt)*
                ) => {
                    #{name(ix + 1)}! {
                        [$($__coinduction_impls)*] {$($__coinduction_items)*} $($__coinduction_rest)*
                    }
                };
            }
        }
        macro_rules! #last {
            ([$($__coinduction_impls:tt)*] {$($__coinduction_items:tt)*}) => {
                #[#coinduction::coinduction(coinduction = #coinduction)]
                #(#outer_attrs)*
                #{&module.vis} #{&module.unsafety} #{&module.mod_token} #{&module.ident} {
                    $($__coinduction_items)*
                    $($__coinduction_impls)*
                }
            };
        }
        #{name(0)}! {
            [] { #(#inner_attrs)* #(#items)* }
            #(for (_, item_impl) in &conditional) { {#item_impl} }
        }
    }
}

pub fn coinduction(module: ItemMod, args: ModuleArgs) -> TokenStream {
    if module.content.as_ref().is_some_and(|(_, items)| {
        items
            .iter()
            .any(|item| matches!(item, Item::Impl(item_impl) if has_cfg(&item_impl.attrs)))
    }) {
        return eval_cfg_impls(module, &args);
    }
    // Traits marked with `#[coinduction] use ...;` or announced in the module
    let mut marked_traits = HashSet::new();
    // Types announced in the module: their announcements carry the predicates of their impls
    let mut announced_types = Vec::new();
    // Names imported under another name, to compare traits by their own names. A name that is
    // also imported as it is, as in `use path::{Count, CountMacro as Count};` for a trait whose
    // trampoline is renamed, names that item.
    let mut trait_aliases = Vec::new();
    let mut imported_names = Vec::new();
    for item in module.content.as_ref().map_or(&[][..], |c| &c.1) {
        if let Item::Use(item_use) = item {
            collect_use_aliases(&item_use.tree, &mut trait_aliases, &mut imported_names);
        }
    }
    trait_aliases.retain(|(alias, _)| !imported_names.contains(alias));
    let (target_impls, mut other_contents): (Vec<ItemImpl>, Vec<Item>) = module
        .content
        .as_ref()
        .map(|c| &c.1)
        .into_iter()
        .flatten()
        .fold(Default::default(), |(mut impls, mut others), item| {
            let mut item = item.clone();
            let marked = match &mut item {
                Item::Const(ItemConst { attrs, .. })
                | Item::Enum(ItemEnum { attrs, .. })
                | Item::ExternCrate(ItemExternCrate { attrs, .. })
                | Item::Fn(ItemFn { attrs, .. })
                | Item::ForeignMod(ItemForeignMod { attrs, .. })
                | Item::Impl(ItemImpl { attrs, .. })
                | Item::Macro(ItemMacro { attrs, .. })
                | Item::Mod(ItemMod { attrs, .. })
                | Item::Static(ItemStatic { attrs, .. })
                | Item::Struct(ItemStruct { attrs, .. })
                | Item::Trait(ItemTrait { attrs, .. })
                | Item::TraitAlias(ItemTraitAlias { attrs, .. })
                | Item::Type(ItemType { attrs, .. })
                | Item::Union(ItemUnion { attrs, .. })
                | Item::Use(ItemUse { attrs, .. }) => take_coinduction_attr(attrs),
                _ => None,
            };
            match (item, marked) {
                (Item::Impl(item_impl), None) if item_impl.trait_.is_some() => {
                    impls.push(item_impl);
                }
                (Item::Use(item_use), Some(attr)) => {
                    if !matches!(attr.meta, Meta::Path(_)) {
                        abort!(attr, "arguments are not supported on `#[coinduction] use`");
                    }
                    let mut prefix = Path {
                        leading_colon: item_use.leading_colon,
                        segments: Punctuated::new(),
                    };
                    collect_use_traits(&item_use.tree, &mut prefix, &mut marked_traits);
                    let mut item_use = item_use;
                    item_use.attrs.push(parse_quote!(#[allow(unused_imports)]));
                    others.push(Item::Use(item_use));
                }
                (item, Some(_)) => abort!(
                    item,
                    "`#[coinduction]` in a `#[coinduction]` module can only be applied to `use` items"
                ),
                (Item::Trait(item_trait), None) => {
                    if is_announced(&item_trait.attrs) {
                        marked_traits.insert(NoArgPath(item_trait.ident.clone().into()));
                    }
                    others.push(Item::Trait(item_trait));
                }
                (
                    item @ (Item::Struct(_) | Item::Enum(_) | Item::Union(_)),
                    None,
                ) => {
                    let (Item::Struct(ItemStruct { attrs, ident, .. })
                    | Item::Enum(ItemEnum { attrs, ident, .. })
                    | Item::Union(ItemUnion { attrs, ident, .. })) = &item
                    else {
                        unreachable!()
                    };
                    if is_announced(attrs) {
                        announced_types.push(ident.clone());
                    }
                    others.push(item);
                }
                (item, None) => others.push(item),
            }
            (impls, others)
        });
    let working_traits: HashSet<_> = if !marked_traits.is_empty() {
        marked_traits
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
                let root_node = graph.insert_node(constraint.clone()).unwrap();
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
            Some(solver)
        })
        .collect();
    let impls_metas = crate::typedef::type_impls(
        module.content.as_ref().map_or(&[][..], |c| &c.1),
        &working_traits,
        &announced_types,
        &trait_aliases,
    );
    for item in other_contents.iter_mut() {
        if let Item::Struct(ItemStruct { attrs, ident, .. })
        | Item::Enum(ItemEnum { attrs, ident, .. })
        | Item::Union(ItemUnion { attrs, ident, .. }) = item
        {
            if let Some(meta) = impls_metas.get(ident) {
                add_announce_meta(attrs, meta);
            }
        }
    }
    let next_step_args = NextStepArgs {
        working_list: working_list.into_iter().collect(),
        coinduction: args.coinduction.clone(),
        working_traits: working_traits.into_iter().collect(),
        trait_aliases,
        ignore_tys,
        solvers,
        target_impls,
    };
    let next = next_step(next_step_args);
    quote! {
        #(for attr in &module.attrs) {
            #(if matches!(attr.style, AttrStyle::Outer)) { #attr }
        }
        #{ &module.vis }
        #{ &module.unsafety }
        #{ &module.mod_token }
        #{ &module.ident } {
            #(for attr in &module.attrs) {
                #(if matches!(attr.style, AttrStyle::Inner(_))) { #attr }
            }
            #(for content in other_contents) { #content }
            #next
        }
    }
}
