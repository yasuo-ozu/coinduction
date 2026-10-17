use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::*;
use template_quote::quote;

use crate::NoArgPath;

/// Whether a `META` element of an announced trait holds the rules of the trait:
/// `#[announce(meta = coinduction::rules(...))]`. The path is never resolved, so it is compared
/// by its last two segments.
pub fn is_rules_meta(path: &Path) -> bool {
    let n = path.segments.len();
    n >= 2 && path.segments[n - 2].ident == "coinduction" && path.segments[n - 1].ident == "rules"
}

/// The arguments of `coinduction::rules(...)`: `(<pattern>) => { <predicates> }, ...`.
pub struct Rules {
    pub rules: Vec<(TokenStream, TokenStream)>,
}

impl Parse for Rules {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut rules = Vec::new();
        while !input.is_empty() {
            // Parse rule pattern (token stream)
            let pattern_content;
            syn::parenthesized!(pattern_content in input);
            let pattern: TokenStream = pattern_content.parse()?;

            input.parse::<Token![=>]>()?;

            // Parse constraints
            let constraints_content;
            syn::braced!(constraints_content in input);
            let constraints = constraints_content.parse()?;

            rules.push((pattern, constraints));

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Rules { rules })
    }
}

/// Match `ty` against the rules of a trait. The rules are `macro_rules!` patterns, so this emits
/// a local `macro_rules!` with one arm per rule and calls it on `ty`.
///
/// The arm of a matching rule calls `__next_step` with the name of the trait and the predicates of
/// the rule; when no rule matches, the last arm expands to `fallback`, with
/// `$($__coinduction_rest)*` standing for `state`.
pub(crate) fn matcher(
    trait_ident: &Ident,
    rules: &Rules,
    ty: &Type,
    fallback: TokenStream,
    coinduction: &NoArgPath,
    state: TokenStream,
) -> TokenStream {
    let name = Ident::new(
        &format!("__coinduction_rules_{}", crate::get_random()),
        proc_macro2::Span::call_site(),
    );
    quote! {
        macro_rules! #name {
            #(for (pattern, constraints) in &rules.rules) {
                ([#pattern] $($__coinduction_rest:tt)*) => {
                    #coinduction::__next_step! {
                        @rules #trait_ident [#constraints] $($__coinduction_rest)*
                    }
                };
            }
            ([$($__coinduction_ty:tt)*] $($__coinduction_rest:tt)*) => {
                #fallback
            };
        }
        #name! { [#ty] #state }
    }
}
