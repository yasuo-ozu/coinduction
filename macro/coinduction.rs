use proc_macro2::TokenStream;
use proc_macro_error::abort;
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;
use template_quote::quote;

use crate::next_step::{next_step, NextStepArgs, NextStepKind};
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

/// Parse the arguments of an inner `#[coinduction]` / `#[coinduction(...)]` attribute.
fn parse_attr_args<T: Parse>(attr: &Attribute) -> T {
    match &attr.meta {
        Meta::Path(_) => parse2(TokenStream::new()),
        Meta::List(_) => attr.parse_args(),
        Meta::NameValue(_) => abort!(attr, "expected `#[coinduction]` or `#[coinduction(...)]`"),
    }
    .unwrap_or_else(|e| abort!(e.span(), "{}", e))
}

/// Arguments of `#[coinduction(...)]` on a struct, enum or union in a module: `macro_name = <ident>`.
struct TypeArgs {
    macro_name: Option<Ident>,
}

impl Parse for TypeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut macro_name = None;
        if !input.is_empty() {
            input.parse::<crate::kw::macro_name>()?;
            input.parse::<Token![=]>()?;
            macro_name = Some(input.parse()?);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        if !input.is_empty() {
            return Err(input.error("expected `macro_name = <ident>`"));
        }
        Ok(TypeArgs { macro_name })
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

/// Arguments of `#[coinduction(...)]` on a module: `marker = <path>` and `coinduction = <path>`.
pub struct ModuleArgs {
    coinduction: NoArgPath,
    marker: Option<Path>,
}

impl Parse for ModuleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut coinduction = None;
        let mut marker = None;
        while !input.is_empty() {
            if input.peek(kw::marker) && input.peek2(Token![=]) && marker.is_none() {
                input.parse::<kw::marker>()?;
                input.parse::<Token![=]>()?;
                marker = Some(input.parse()?);
            } else if input.peek(kw::coinduction) && input.peek2(Token![=]) && coinduction.is_none()
            {
                input.parse::<kw::coinduction>()?;
                input.parse::<Token![=]>()?;
                coinduction = Some(input.parse()?);
            } else {
                return Err(input.error(
                    "expected `marker = <path>` or `coinduction = <path>`; mark traits with `#[coinduction] use ...;` or `#[coinduction] trait ...` inside the module",
                ));
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(ModuleArgs {
            coinduction: coinduction.unwrap_or_else(|| NoArgPath(parse_quote!(::coinduction))),
            marker,
        })
    }
}

pub fn coinduction(module: ItemMod, args: ModuleArgs) -> TokenStream {
    // Traits marked with `#[coinduction] use ...;` or `#[coinduction] trait ...` in the module
    let mut marked_traits = HashSet::new();
    // Custom names for the macros of the types, given by `#[coinduction(macro_name = ...)]`
    let mut macro_names = HashMap::new();
    let (target_impls, other_contents): (Vec<ItemImpl>, Vec<TokenStream>) = module
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
                    others.push(quote!(#[allow(unused_imports)] #item_use));
                }
                (Item::Trait(item_trait), Some(attr)) => {
                    marked_traits.insert(NoArgPath(item_trait.ident.clone().into()));
                    others.push(crate::traitdef::traitdef(item_trait, parse_attr_args(&attr)));
                }
                (
                    item @ (Item::Struct(_) | Item::Enum(_) | Item::Union(_)),
                    Some(attr),
                ) => {
                    if let Some(macro_name) = parse_attr_args::<TypeArgs>(&attr).macro_name {
                        let (Item::Struct(ItemStruct { ident, .. })
                        | Item::Enum(ItemEnum { ident, .. })
                        | Item::Union(ItemUnion { ident, .. })) = &item
                        else {
                            unreachable!()
                        };
                        macro_names.insert(ident.clone(), macro_name);
                    }
                    others.push(quote!(#item));
                }
                (item, Some(_)) => abort!(
                    item,
                    "`#[coinduction]` in a `#[coinduction]` module can only be applied to `use`, `trait`, `struct`, `enum` or `union` items"
                ),
                (item, None) => others.push(quote!(#item)),
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
    let rewrite_rules = crate::solver::rules(&target_impls, |t| working_traits.contains(t));
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

            working_list.extend(
                solver.expand(&[constraint], &rewrite_rules, &ignore_tys, |t| {
                    working_traits.contains(t)
                }),
            );
            Some(solver)
        })
        .collect();
    let type_macros = crate::typedef::type_macros(
        module.content.as_ref().map_or(&[][..], |c| &c.1),
        &working_traits,
        &macro_names,
        args.marker.as_ref(),
        &args.coinduction,
    );
    let next_step_args = NextStepArgs {
        kind: NextStepKind::None,
        working_list: working_list.into_iter().collect(),
        coinduction: args.coinduction.clone(),
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
            #type_macros
        }
    }
}
