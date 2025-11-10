use proc_macro::TokenStream;
use template_quote::quote;
use syn::{parse_macro_input, Path};
use crate::common::{ConstraintGraph, TypeConstraint};
use std::collections::HashMap;

#[derive(Debug)]
struct InternalArgs {
    module: syn::ItemMod,
    working_list: Vec<TypeConstraint>,
    target_constraint: Option<TypeConstraint>,
    trait_names: Vec<Path>,
    graphs: Vec<ConstraintGraph>,
    additional_constraints: Vec<TypeConstraint>,
    coinduction_path: Path,
}

pub fn internal_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as InternalArgs);
    
    // For each graph, find constraints matching the target constraint
    let mut updated_graphs = args.graphs;
    
    if let Some(ref target_constraint) = args.target_constraint {
        for graph in &mut updated_graphs {
            // Find node that matches the target constraint
            if let Some(target_node) = graph.find_constraint(target_constraint) {
                // Add additional constraints as new nodes connected to the target
                for additional_constraint in &args.additional_constraints {
                    let adapted_constraint = adapt_constraint_types(
                        additional_constraint,
                        target_constraint
                    );
                    let new_node = graph.add_constraint(adapted_constraint);
                    graph.add_edge(target_node, new_node);
                }
            }
        }
    }

    // Continue the recursion by calling the next trait in the working list
    if !args.working_list.is_empty() {
        let mut remaining_list = args.working_list;
        if let Some(next_target) = remaining_list.pop() {
            let trait_path = &next_target.trait_path;
            let module = &args.module;
            let module_tokens = quote! { #module };
            let working_list_tokens = quote! { vec![#(#remaining_list),*] };
            let trait_names = &args.trait_names;
            let trait_names_tokens = quote! { vec![#(#trait_names),*] };
            let graphs_tokens = quote! { vec![#(#updated_graphs),*] };
            
            let result = quote! {
                #trait_path!(#module_tokens, #working_list_tokens, #next_target, #trait_names_tokens, #graphs_tokens)
            };
            
            return result.into();
        }
    }

    // If no more constraints, call __finalize
    let coinduction_path = &args.coinduction_path;
    let module = &args.module;
    let module_tokens = quote! { #module };
    let graphs_tokens = quote! { vec![#(#updated_graphs),*] };
    
    let result = quote! {
        #coinduction_path::__finalize!(#module_tokens, #graphs_tokens)
    };

    result.into()
}


fn adapt_constraint_types(
    constraint: &TypeConstraint,
    target_constraint: &TypeConstraint
) -> TypeConstraint {
    // Create a simple type variable substitution map
    let mut substitution_map = HashMap::new();
    
    // Try to match type variables from the constraint with concrete types from the target
    if let (syn::Type::Path(constraint_path), syn::Type::Path(target_path)) = (&constraint.ty, &target_constraint.ty) {
        // Build substitution map for type parameters
        extract_type_substitutions(&constraint_path.path, &target_path.path, &mut substitution_map);
    }
    
    // Apply substitutions to create adapted constraint
    let adapted_type = substitute_type_variables(&constraint.ty, &substitution_map);
    
    TypeConstraint {
        ty: adapted_type,
        trait_path: constraint.trait_path.clone(),
    }
}

fn extract_type_substitutions(
    pattern_path: &syn::Path,
    concrete_path: &syn::Path,
    substitutions: &mut HashMap<String, syn::Type>,
) {
    // Simple pattern matching for path segments with generic arguments
    if pattern_path.segments.len() == 1 && concrete_path.segments.len() == 1 {
        let pattern_seg = &pattern_path.segments[0];
        let concrete_seg = &concrete_path.segments[0];
        
        if pattern_seg.ident == concrete_seg.ident {
            // Match generic arguments if they exist
            if let (
                syn::PathArguments::AngleBracketed(pattern_args),
                syn::PathArguments::AngleBracketed(concrete_args),
            ) = (&pattern_seg.arguments, &concrete_seg.arguments) {
                for (pattern_arg, concrete_arg) in pattern_args.args.iter().zip(concrete_args.args.iter()) {
                    if let (
                        syn::GenericArgument::Type(syn::Type::Path(pattern_type)),
                        syn::GenericArgument::Type(concrete_type),
                    ) = (pattern_arg, concrete_arg) {
                        if pattern_type.path.segments.len() == 1 {
                            let var_name = pattern_type.path.segments[0].ident.to_string();
                            substitutions.insert(var_name, concrete_type.clone());
                        }
                    }
                }
            }
        }
    }
}

fn substitute_type_variables(
    ty: &syn::Type,
    substitutions: &HashMap<String, syn::Type>,
) -> syn::Type {
    match ty {
        syn::Type::Path(type_path) => {
            if type_path.path.segments.len() == 1 {
                let segment = &type_path.path.segments[0];
                let ident_str = segment.ident.to_string();
                
                if let Some(substitution) = substitutions.get(&ident_str) {
                    return substitution.clone();
                }
            }
            ty.clone()
        }
        _ => ty.clone(),
    }
}

impl syn::parse::Parse for InternalArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse: module, working_list, target_constraint, trait_names, graphs, additional_constraints
        let module: syn::ItemMod = input.parse()?;
        let _: syn::Token![,] = input.parse()?;
        
        // Parse working list
        let content;
        syn::bracketed!(content in input);
        let mut working_list = Vec::new();
        while !content.is_empty() {
            working_list.push(content.parse::<TypeConstraint>()?);
            if !content.is_empty() {
                let _: syn::Token![,] = content.parse()?;
            }
        }
        let _: syn::Token![,] = input.parse()?;
        
        // Parse target constraint (optional)
        let target_constraint = if input.peek(syn::Ident) {
            Some(input.parse::<TypeConstraint>()?)
        } else if input.peek(syn::token::Paren) {
            // Skip empty parens
            let _content;
            syn::parenthesized!(_content in input);
            None
        } else {
            None
        };
        
        if target_constraint.is_some() {
            let _: syn::Token![,] = input.parse()?;
        }
        
        // Parse trait names
        let content;
        syn::bracketed!(content in input);
        let mut trait_names = Vec::new();
        while !content.is_empty() {
            trait_names.push(content.parse::<Path>()?);
            if !content.is_empty() {
                let _: syn::Token![,] = content.parse()?;
            }
        }
        let _: syn::Token![,] = input.parse()?;
        
        // Parse graphs
        let content;
        syn::bracketed!(content in input);
        let mut graphs = Vec::new();
        while !content.is_empty() {
            graphs.push(content.parse::<ConstraintGraph>()?);
            if !content.is_empty() {
                let _: syn::Token![,] = content.parse()?;
            }
        }
        let _: syn::Token![,] = input.parse()?;
        
        // Parse additional constraints
        let content;
        syn::bracketed!(content in input);
        let mut additional_constraints = Vec::new();
        while !content.is_empty() {
            additional_constraints.push(content.parse::<TypeConstraint>()?);
            if !content.is_empty() {
                let _: syn::Token![,] = content.parse()?;
            }
        }
        
        // Parse coinduction path if present
        let coinduction_path = if !input.is_empty() {
            let _: syn::Token![,] = input.parse()?;
            input.parse()?
        } else {
            syn::parse_quote! { ::coinduction }
        };

        Ok(InternalArgs {
            module,
            working_list,
            target_constraint,
            trait_names,
            graphs,
            additional_constraints,
            coinduction_path,
        })
    }
}

