use proc_macro2::TokenStream;
use template_quote::{quote, ToTokens};
use syn::{parse::Parse, punctuated::Punctuated, Path, Token, Type};

struct TarjanState {
    index_counter: usize,
    stack: Vec<usize>,
    indices: Vec<Option<usize>>,
    lowlinks: Vec<usize>,
    on_stack: Vec<bool>,
    sccs: Vec<Vec<usize>>,
}

/// A constraint in the form `Type: Trait`
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub ty: Type,
    pub trait_path: Path,
}

impl Parse for TypeConstraint {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse()?;
        let _: Token![:] = input.parse()?;
        let trait_path: Path = input.parse()?;
        Ok(TypeConstraint { ty, trait_path })
    }
}

impl ToTokens for TypeConstraint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty = &self.ty;
        let trait_path = &self.trait_path;
        tokens.extend(quote! { #ty: #trait_path });
    }
}

/// Simplified constraint graph (reverting to simple Vec storage due to gotgraph complexity)
#[derive(Debug, Clone)]
pub struct ConstraintGraph {
    constraints: Vec<TypeConstraint>,
    edges: Vec<(usize, usize)>,
    root_node: Option<usize>,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            edges: Vec::new(),
            root_node: None,
        }
    }

    pub fn set_root(&mut self, constraint: TypeConstraint) -> usize {
        let node_id = self.constraints.len();
        self.constraints.push(constraint);
        self.root_node = Some(node_id);
        node_id
    }

    pub fn add_constraint(&mut self, constraint: TypeConstraint) -> usize {
        let node_id = self.constraints.len();
        self.constraints.push(constraint);
        node_id
    }

    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.edges.push((from, to));
    }


    pub fn constraints(&self) -> impl Iterator<Item = &TypeConstraint> {
        self.constraints.iter()
    }

    pub fn get_constraint(&self, node_id: usize) -> Option<&TypeConstraint> {
        self.constraints.get(node_id)
    }

    pub fn find_constraint(&self, target: &TypeConstraint) -> Option<usize> {
        self.constraints.iter().position(|constraint| {
            constraints_match(constraint, target)
        })
    }


    pub fn neighbors(&self, node_id: usize) -> impl Iterator<Item = usize> + '_ {
        self.edges.iter()
            .filter(move |(from, _)| *from == node_id)
            .map(|(_, to)| *to)
    }

    pub fn node_count(&self) -> usize {
        self.constraints.len()
    }

    pub fn find_strongly_connected_components(&self) -> Vec<Vec<usize>> {
        // Simple SCC algorithm using Tarjan's
        let mut state = TarjanState {
            index_counter: 0,
            stack: Vec::new(),
            indices: vec![None; self.node_count()],
            lowlinks: vec![0; self.node_count()],
            on_stack: vec![false; self.node_count()],
            sccs: Vec::new(),
        };

        for node in 0..self.node_count() {
            if state.indices[node].is_none() {
                self.strongconnect(node, &mut state);
            }
        }

        state.sccs
    }

    fn strongconnect(&self, v: usize, state: &mut TarjanState) {
        state.indices[v] = Some(state.index_counter);
        state.lowlinks[v] = state.index_counter;
        state.index_counter += 1;
        state.stack.push(v);
        state.on_stack[v] = true;

        for w in self.neighbors(v) {
            if state.indices[w].is_none() {
                self.strongconnect(w, state);
                state.lowlinks[v] = state.lowlinks[v].min(state.lowlinks[w]);
            } else if state.on_stack[w] {
                state.lowlinks[v] = state.lowlinks[v].min(state.indices[w].unwrap());
            }
        }

        if state.lowlinks[v] == state.indices[v].unwrap() {
            let mut component = Vec::new();
            loop {
                let w = state.stack.pop().unwrap();
                state.on_stack[w] = false;
                component.push(w);
                if w == v {
                    break;
                }
            }
            state.sccs.push(component);
        }
    }
}

impl Parse for ConstraintGraph {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut graph = ConstraintGraph::new();
        
        if input.is_empty() {
            return Ok(graph);
        }

        let constraints: Punctuated<TypeConstraint, Token![,]> = 
            input.parse_terminated(TypeConstraint::parse, Token![,])?;
        
        for constraint in constraints {
            graph.add_constraint(constraint);
        }
        
        Ok(graph)
    }
}

impl ToTokens for ConstraintGraph {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let constraints: Vec<_> = self.constraints().cloned().collect();
        tokens.extend(quote! { 
            vec![#(#constraints),*]
        });
    }
}

/// Helper function to check if two constraints match
pub fn constraints_match(constraint1: &TypeConstraint, constraint2: &TypeConstraint) -> bool {
    // For now, do a simple string comparison of the constraint representation
    // In a more sophisticated implementation, this would handle type unification
    format!("{}", quote! { #constraint1 }) == format!("{}", quote! { #constraint2 })
}

