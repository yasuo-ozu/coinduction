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
            #(for (pattern, constraints) in &args.rules) {
                (#crate_version, None, [#pattern  $(,$($wt:tt)*)?], $coinduction:path, $($t:tt)*) => {
                    $coinduction::__next_step ! {
                        #crate_version, Traitdef {
                            appending_constraints: [
                                #constraints
                            ]
                        }, [_ $(,$($wt)*)?], $coinduction, $($t)*
                    }
                };
            }
            (#crate_version, None, [
                 :: $seg0:ident $(:: $segs:ident)* $(<$($arg:ty),*$(,)?>)?
                 :$($wt:tt)*
            ], $coinduction:path, $($t:tt)*) => {
                :: $seg0 $(:: $segs)* ! {
                    #crate_version, None, [
                        $ty0: :: $seg0 $(:: $segs)* $(<$($arg),*>)?
                        :$($wt)*
                    ], $coinduction, $($t)*
                }
            };
            (#crate_version, None, [
                 $seg0:ident $(:: $segs:ident)* $(<$($arg:ty),*$(,)?>)?
                 :$($wt:tt)*
            ], $coinduction:path, $($t:tt)*) => {
                 $seg0 $(:: $segs)*! {
                    #crate_version, None, [
                        $seg0 $(:: $segs)* $(<$($arg),*>)?
                        :$($wt)*
                    ], $coinduction, $($t)*
                }
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports, unused_macros, dead_code)]
        #{&item.vis} use #temporal_mac_name as #{&item.ident};
    }
}
