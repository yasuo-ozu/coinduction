use crate::common::TypeConstraint;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemTrait, Path};
use template_quote::quote;

#[derive(Debug, Clone)]
struct TraitDefArgs {
    coinduction_path: Path,
    marker_path: Option<Path>,
    patterns: Vec<PatternRule>,
}

#[derive(Debug, Clone)]
struct PatternRule {
    pattern: proc_macro2::TokenStream,
    constraints: Vec<TypeConstraint>,
}

impl PatternRule {
    fn parse_pattern_constraints(
        input: syn::parse::ParseStream,
    ) -> syn::Result<Vec<TypeConstraint>> {
        let mut constraints = Vec::new();

        while !input.is_empty() {
            let constraint = input.parse::<TypeConstraint>()?;
            constraints.push(constraint);

            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(constraints)
    }
}

impl syn::parse::Parse for TraitDefArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut coinduction_path = syn::parse_quote! { ::coinduction };
        let mut marker_path = None;
        let mut patterns = Vec::new();

        while !input.is_empty() {
            if input.peek(syn::Ident) && input.peek2(syn::Token![=]) {
                let name: syn::Ident = input.parse()?;
                let _: syn::Token![=] = input.parse()?;

                if name == "coinduction" {
                    coinduction_path = input.parse()?;
                } else if name == "marker" {
                    marker_path = Some(input.parse()?);
                } else {
                    return Err(syn::Error::new_spanned(name, "Unknown argument"));
                }
            } else if input.peek(syn::token::Paren) || input.peek(syn::token::Bracket) {
                // Parse pattern => { constraints }
                // Support patterns like ($t1:ty, $t2:ty) => {$t1: MyTrait1, $t2: MyTrait2}
                let pattern = input.parse::<proc_macro2::TokenStream>()?;
                let _: syn::Token![=>] = input.parse()?;

                let content;
                syn::braced!(content in input);
                let constraints = PatternRule::parse_pattern_constraints(&content)?;

                patterns.push(PatternRule {
                    pattern,
                    constraints,
                });
            }

            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(TraitDefArgs {
            coinduction_path,
            marker_path,
            patterns,
        })
    }
}

pub fn traitdef_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as TraitDefArgs);
    let trait_item = parse_macro_input!(input as ItemTrait);

    let trait_ident = &trait_item.ident;
    let trait_vis = &trait_item.vis;
    // Create a random identifier with __ prefix
    let random_suffix = std::process::id(); // Use process ID for uniqueness
    let temporal_mac_name = syn::Ident::new(
        &format!("__{}_temporal_{}", trait_ident, random_suffix),
        trait_ident.span(),
    );
    let coinduction_path = &args.coinduction_path;

    // Generate pattern matching arms for type patterns
    let pattern_arms = args.patterns.iter().map(|rule| {
        let pattern = &rule.pattern;
        let constraints = &rule.constraints;

        quote! {
            (#pattern, $module:item, $working_list:expr, $trait_names:expr, $graphs:expr) => {
                #coinduction_path::__internal! {
                    $module, $working_list, (), $trait_names, $graphs, vec![#(#constraints),*]
                }
            }
        }
    });

    // Generate TypeRef implementation if marker_path is provided
    let typeref_impl = if let Some(marker_path) = &args.marker_path {
        quote! {
            impl<T: ?Sized> #coinduction_path::TypeRef<T> for #marker_path {
                type Type = T;
            }
        }
    } else {
        quote! {}
    };

    let crate_version = env!("CARGO_PKG_VERSION");
    let generated = quote! {
        #trait_item

        #typeref_impl

        #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #temporal_mac_name {
            // Version check - accept correct version
            (@version_check #crate_version) => {};
            
            // Version check - emit error for wrong version
            (@version_check $version:literal) => {
                compile_error!(concat!("version of coinduction crate mismatch: ", $version, " != ", #crate_version));
            };
            
            // Handle full target constraint form: (Type: Trait)
            (($module:item, $working_list:expr, $constraint_ty:ty: #trait_ident, $trait_names:expr, $graphs:expr)) => {
                #temporal_mac_name! { $constraint_ty, $module, $working_list, $trait_names, $graphs }
            };

            // Pattern matching arms for type patterns
            #(#pattern_arms)*

            // Fallback to type macro for simple type matching
            ($constraint_ty:ty, $($args:tt)*) => {
                $constraint_ty! { $($args)* }
            };

            // Handle trait path that is not defined with #[coinduction::traitdef] macro
            (trait [$trait_name:path] is not defined with #[coinduction::traitdef] macro, so is not used as an argument for #[$macro_name:path] macro) => {
                // Accept the arguments gracefully without emitting anything
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports, unused_macros, dead_code)]
        #trait_vis use #temporal_mac_name as #trait_ident;
    };

    generated.into()
}
