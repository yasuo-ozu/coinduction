use proc_macro2::TokenStream;
use syn::*;
use syn::{parse::Parse, parse::ParseStream, ItemTrait};
use template_quote::quote;

use crate::NoArgPath;

pub struct TraitDefArgs {
    #[allow(dead_code)]
    pub coinduction: NoArgPath,
    pub rules: Vec<(TokenStream, TokenStream)>,
}

impl Parse for TraitDefArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // let coinduction = crate::try_parse_coinduction_args(input)?;
        let coinduction = parse2(quote! {::coinduction}).unwrap();
        let mut rules = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
            }

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
        }

        Ok(TraitDefArgs { coinduction, rules })
    }
}

fn remove_matcher_kinds(input: TokenStream) -> TokenStream {
    // Remove the `XXX` from `$yyy:XXX` in input
    use proc_macro2::TokenTree;

    let mut result = TokenStream::new();
    let mut tokens = input.into_iter().peekable();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(ref p) if p.as_char() == '$' => {
                // This is a macro variable marker ($)
                result.extend(Some(token));

                // Get the variable name following $
                if let Some(name) = tokens.next() {
                    result.extend(Some(name));

                    // Check if next is a colon (indicating matcher kind specifier)
                    if let Some(TokenTree::Punct(ref colon)) = tokens.peek() {
                        if colon.as_char() == ':' {
                            // Skip the colon
                            tokens.next();
                            // Skip the matcher kind (e.g., ident, ty, expr, etc.)
                            tokens.next();
                        }
                    }
                }
            }
            TokenTree::Group(group) => {
                // Recursively process groups (parentheses, braces, brackets)
                let delim = group.delimiter();
                let stream = remove_matcher_kinds(group.stream());
                let mut new_group = proc_macro2::Group::new(delim, stream);
                new_group.set_span(group.span());
                result.extend(Some(TokenTree::Group(new_group)));
            }
            _ => {
                // Pass through other tokens unchanged
                result.extend(Some(token));
            }
        }
    }

    result
}

pub fn traitdef(item: ItemTrait, args: TraitDefArgs) -> TokenStream {
    let random_suffix = crate::get_random();
    let temporal_mac_name = syn::Ident::new(
        &format!("__{}_temporal_{}", &item.ident, random_suffix),
        item.ident.span(),
    );
    let crate_version = env!("CARGO_PKG_VERSION");
    quote! {
        #item

        #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #temporal_mac_name {
            #(for (pattern, pattern_converted, constraints) in args.rules.iter().map(|(pattern, constraints)| (pattern.clone(), remove_matcher_kinds(pattern.clone()), constraints))) {
                (#crate_version, None, [#pattern  :$($wt:tt)*], {$($coinduction:tt)+}, $($t:tt)*) => {
                    $($coinduction)+::__next_step ! {
                        #crate_version, Traitdef {
                            appending_constraints: [
                                #constraints
                            ]
                        }, [#pattern_converted :$($wt)*], {$($coinduction)+}, $($t)*
                    }
                };
            }
            (#crate_version, None, [
                 :: $seg0:ident $(:: $segs:ident)* $(<$($arg:ty),*$(,)?>)?
                 :$($wt:tt)*
            ], {$($coinduction:tt)+}, $($t:tt)*) => {
                :: $seg0 $(:: $segs)* ! {
                    #crate_version, None, [
                        $ty0: :: $seg0 $(:: $segs)* $(<$($arg),*>)?
                        :$($wt)*
                    ], {$($coinduction)+}, $($t)*
                }
            };
            (#crate_version, None, [
                 $seg0:ident $(:: $segs:ident)* $(<$($arg:ty),*$(,)?>)?
                 :$($wt:tt)*
            ], {$($coinduction:tt)+}, $($t:tt)*) => {
                 $seg0 $(:: $segs)*! {
                    #crate_version, None, [
                        $seg0 $(:: $segs)* $(<$($arg),*>)?
                        :$($wt)*
                    ], {$($coinduction)+}, $($t)*
                }
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports, unused_macros, dead_code)]
        #{&item.vis} use #temporal_mac_name as #{&item.ident};
    }
}
