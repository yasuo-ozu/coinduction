use proc_macro::TokenStream;
use template_quote::quote;
use syn::{parse_macro_input, ItemMod, Path, Item, ItemImpl};
use crate::common::TypeConstraint;

#[derive(Debug)]
struct TypeDefArgs {
    trait_paths: Vec<Path>,
    coinduction_path: Path,
    marker_path: Option<Path>,
}

impl syn::parse::Parse for TypeDefArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut trait_paths = Vec::new();
        let mut coinduction_path = syn::parse_quote! { ::coinduction };
        let mut marker_path = None;

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
            } else {
                trait_paths.push(input.parse()?);
            }

            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(TypeDefArgs {
            trait_paths,
            coinduction_path,
            marker_path,
        })
    }
}

pub fn typedef_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let crate_version = env!("CARGO_PKG_VERSION");
    let args = parse_macro_input!(args as TypeDefArgs);
    let module = parse_macro_input!(input as ItemMod);

    let items = match &module.content {
        Some((_, items)) => items,
        None => return syn::Error::new_spanned(module, "Module must have content").to_compile_error().into(),
    };

    // Only validate traits when explicitly provided as arguments
    // Auto-detected traits don't need validation since they're defined in the same module
    let trait_validations = if !args.trait_paths.is_empty() {
        args.trait_paths.iter().filter_map(|trait_path| {
            // Validate all trait paths, but extract the final segment for the macro call
            if let Some(last_segment) = trait_path.segments.last() {
                // Check that the final segment has no generics
                if last_segment.arguments.is_empty() {
                    let trait_name = &last_segment.ident;
                    Some(quote! {
                        const _: () = {
                            // Version check
                            #trait_name! { @version_check #crate_version }
                            
                            macro_rules! this_trait_is_not_defined_with_coinduction_traitdef {
                                () => {
                                    #trait_name! { trait [#trait_path] is not defined with #[coinduction::traitdef] macro, so is not used as an argument for #[typedef] macro }
                                };
                            }
                            this_trait_is_not_defined_with_coinduction_traitdef!();
                        };
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }).collect()
    } else {
        Vec::new()
    };

    // Find type definitions and their corresponding impls
    let mut type_macros = Vec::new();

    for item in items.iter() {
        match item {
            Item::Struct(struct_item) => {
                if let Some(type_macro) = generate_type_macro(&struct_item.ident, items, &args) {
                    type_macros.push(type_macro);
                }
            }
            Item::Enum(enum_item) => {
                if let Some(type_macro) = generate_type_macro(&enum_item.ident, items, &args) {
                    type_macros.push(type_macro);
                }
            }
            Item::Union(union_item) => {
                if let Some(type_macro) = generate_type_macro(&union_item.ident, items, &args) {
                    type_macros.push(type_macro);
                }
            }
            Item::Type(type_alias) => {
                if let Some(type_macro) = generate_type_macro(&type_alias.ident, items, &args) {
                    type_macros.push(type_macro);
                }
            }
            _ => {}
        }
    }

    let coinduction_path = &args.coinduction_path;
    
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

    let result = quote! {
        // Validate all traits are defined with #[traitdef]
        #(#trait_validations)*
        
        #module
        
        #typeref_impl
        
        #(#type_macros)*
    };

    result.into()
}

fn generate_type_macro(
    type_ident: &syn::Ident,
    items: &[Item],
    args: &TypeDefArgs,
) -> Option<proc_macro2::TokenStream> {
    // Find impl blocks for this type that implement our target traits
    let mut impl_constraints = Vec::new();

    for item in items.iter() {
        if let Item::Impl(impl_item) = item {
            if let Some((_, trait_path, _)) = &impl_item.trait_ {
                // Check if this impl is for our type and target trait
                if is_impl_for_type(impl_item, type_ident) && 
                   args.trait_paths.iter().any(|tp| paths_equal(tp, trait_path)) {
                    // Extract constraints from this impl
                    if let Ok(constraints) = extract_impl_constraints(impl_item) {
                        impl_constraints.extend(constraints);
                    }
                }
            }
        }
    }

    if impl_constraints.is_empty() {
        return None;
    }

    let coinduction_path = &args.coinduction_path;
    
    // Create a random identifier with __ prefix
    let random_suffix = std::process::id(); // Use process ID for uniqueness
    let temporal_mac_name = syn::Ident::new(
        &format!("__{}_temporal_{}", type_ident, random_suffix),
        type_ident.span(),
    );

    Some(quote! {
        #[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #temporal_mac_name {
            // Handle the full macro arguments from recurse/internal
            ($module:item, $working_list:expr, $target_constraint:expr, $trait_names:expr, $graphs:expr) => {
                #coinduction_path::__internal! {
                    $module, $working_list, $target_constraint, $trait_names, $graphs, vec![#(#impl_constraints),*]
                }
            };
            
            // Simple fallback for direct invocation
            ($target:ty) => {
                #coinduction_path::__internal! {
                    mod empty {}, vec![], (), vec![], vec![], vec![#(#impl_constraints),*]
                }
            };
        }

        #[doc(hidden)]
        #[allow(unused_imports, unused_macros, dead_code)]
        pub use #temporal_mac_name as #type_ident;
    })
}

fn is_impl_for_type(impl_item: &ItemImpl, type_ident: &syn::Ident) -> bool {
    match &*impl_item.self_ty {
        syn::Type::Path(type_path) => {
            type_path.path.segments.last()
                .map(|seg| seg.ident == *type_ident)
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn extract_impl_constraints(impl_item: &ItemImpl) -> syn::Result<Vec<TypeConstraint>> {
    let mut constraints = Vec::new();

    // Extract from generic parameters
    for param in &impl_item.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            for bound in &type_param.bounds {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    constraints.push(TypeConstraint {
                        ty: syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: type_param.ident.clone().into(),
                        }),
                        trait_path: trait_bound.path.clone(),
                    });
                }
            }
        }
    }

    // Extract from where clause
    if let Some(where_clause) = &impl_item.generics.where_clause {
        for predicate in &where_clause.predicates {
            if let syn::WherePredicate::Type(type_predicate) = predicate {
                for bound in &type_predicate.bounds {
                    if let syn::TypeParamBound::Trait(trait_bound) = bound {
                        constraints.push(TypeConstraint {
                            ty: type_predicate.bounded_ty.clone(),
                            trait_path: trait_bound.path.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(constraints)
}


fn paths_equal(a: &Path, b: &Path) -> bool {
    if a.segments.len() != b.segments.len() {
        return false;
    }
    
    a.segments.iter().zip(b.segments.iter()).all(|(seg_a, seg_b)| {
        seg_a.ident == seg_b.ident
    })
}