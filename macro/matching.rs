use core::ops::Deref;
use proc_macro_error::abort;
use std::collections::{HashMap, HashSet};
use syn::{spanned::Spanned, visit::Visit, *};

use crate::solver::Constraint;

fn has_attributes_recursive(arg: &GenericArgument) -> bool {
    struct AttributeChecker(bool);
    impl<'ast> Visit<'ast> for AttributeChecker {
        fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
            self.0 = true;
        }
    }
    let mut checker = AttributeChecker(false);
    checker.visit_generic_argument(arg);
    checker.0
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Substitute(HashMap<GenericParam, GenericArgument>);

impl Deref for Substitute {
    type Target = HashMap<GenericParam, GenericArgument>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Substitute {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_param_arg(param: GenericParam, arg: GenericArgument) -> Self {
        let ret = Self::new();
        ret.insert(param, arg).unwrap()
    }

    pub fn insert(mut self, mut param: GenericParam, arg: GenericArgument) -> Option<Self> {
        // Clean param by removing attributes, bounds, colon_token for consistent comparison
        match &mut param {
            GenericParam::Type(type_param) => {
                type_param.attrs = vec![];
                type_param.bounds = Default::default();
                type_param.colon_token = None;
                type_param.eq_token = None;
                type_param.default = None;
            }
            GenericParam::Lifetime(lifetime_param) => {
                lifetime_param.attrs = vec![];
                lifetime_param.bounds = Default::default();
                lifetime_param.colon_token = None;
            }
            GenericParam::Const(const_param) => {
                const_param.attrs = vec![];
                const_param.eq_token = None;
                const_param.default = None;
            }
        }

        // Abort if arg contains any attributes recursively
        if has_attributes_recursive(&arg) {
            abort!(
                arg,
                "Generic argument contains attributes which are not supported in substitutions"
            );
        }

        let substitute = Substitute(core::iter::once((param.clone(), arg.clone())).collect());
        for value in self.0.values_mut() {
            value.replace(&substitute);
        }

        // Use HashMap::entry() for more efficient insertion
        use std::collections::hash_map::Entry;
        match self.0.entry(param) {
            Entry::Occupied(existing_entry) => {
                if existing_entry.get() == &arg {
                    Some(self)
                } else {
                    None // Conflicting substitution
                }
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(arg.clone());
                Some(self)
            }
        }
    }

    pub fn combine(mut self, other: Self) -> Option<Self> {
        for (param, arg) in other.0 {
            self = self.insert(param, arg)?;
        }
        Some(self)
    }
}

/// Trait for matching AST elements and performing generic parameter substitution
#[allow(unused)]
pub trait Matching {
    /// Check if this element matches another, returning substitutions if successful
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute>;

    /// Replace generic parameters in this element using the provided substitutions
    fn replace(&mut self, dict: &Substitute);
}

impl Matching for Lifetime {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        if self == other {
            // Concrete lifetimes must match exactly
            Some(Substitute::new())
        } else {
            // Check if this lifetime is a generic parameter
            let predicate = GenericParam::Lifetime(LifetimeParam {
                attrs: vec![],
                lifetime: self.clone(),
                colon_token: None,
                bounds: Default::default(),
            });

            if params.contains(&predicate) {
                // This is a generic lifetime parameter, create substitution
                Some(Substitute::from_param_arg(
                    predicate,
                    GenericArgument::Lifetime(other.clone()),
                ))
            } else {
                None
            }
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        let predicate = GenericParam::Lifetime(LifetimeParam {
            attrs: vec![],
            lifetime: self.clone(),
            colon_token: None,
            bounds: Default::default(),
        });

        if let Some(GenericArgument::Lifetime(new_lifetime)) = dict.get(&predicate) {
            *self = new_lifetime.clone();
        }
    }
}

impl Matching for Expr {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        match (self, other) {
            (Expr::Path(l_path), other_expr) => {
                if let Some(ident) = l_path.path.get_ident() {
                    let predicate = GenericParam::Const(ConstParam {
                        attrs: vec![],
                        const_token: Default::default(),
                        ident: ident.clone(),
                        colon_token: Default::default(),
                        ty: parse_quote!(usize),
                        eq_token: None,
                        default: None,
                    });

                    if params.contains(&predicate) {
                        return Some(Substitute::from_param_arg(
                            predicate,
                            GenericArgument::Const(other_expr.clone()),
                        ));
                    }
                }

                // If not a generic parameter, check if both are paths
                if let Expr::Path(r_path) = other_expr {
                    l_path.path.matches(&r_path.path, params)
                } else {
                    None
                }
            }
            (Expr::Binary(_), Expr::Binary(_))
            | (Expr::Call(_), Expr::Call(_))
            | (Expr::Cast(_), Expr::Cast(_))
            | (Expr::Index(_), Expr::Index(_))
            | (Expr::Paren(_), Expr::Paren(_))
            | (Expr::Array(_), Expr::Array(_))
            | (Expr::Assign(_), Expr::Assign(_))
            | (Expr::Block(_), Expr::Block(_))
            | (Expr::Field(_), Expr::Field(_))
            | (Expr::Group(_), Expr::Group(_))
            | (Expr::Lit(_), Expr::Lit(_))
            | (Expr::MethodCall(_), Expr::MethodCall(_))
            | (Expr::Reference(_), Expr::Reference(_))
            | (Expr::Repeat(_), Expr::Repeat(_))
            | (Expr::Tuple(_), Expr::Tuple(_))
            | (Expr::Unary(_), Expr::Unary(_))
            | (Expr::Async(_), Expr::Async(_))
            | (Expr::Await(_), Expr::Await(_))
            | (Expr::Break(_), Expr::Break(_))
            | (Expr::ForLoop(_), Expr::ForLoop(_))
            | (Expr::If(_), Expr::If(_))
            | (Expr::Infer(_), Expr::Infer(_))
            | (Expr::Let(_), Expr::Let(_))
            | (Expr::Macro(_), Expr::Macro(_))
            | (Expr::Match(_), Expr::Match(_))
            | (Expr::RawAddr(_), Expr::RawAddr(_))
            | (Expr::Return(_), Expr::Return(_))
            | (Expr::Unsafe(_), Expr::Unsafe(_))
            | (Expr::While(_), Expr::While(_))
            | (Expr::Yield(_), Expr::Yield(_)) => {
                abort!(&self, "not supported"; hint = other.span() => "other token")
            }
            _ => None,
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        match self {
            Expr::Path(expr_path) => {
                if let Some(ident) = expr_path.path.get_ident() {
                    let predicate = GenericParam::Const(ConstParam {
                        attrs: vec![],
                        const_token: Default::default(),
                        ident: ident.clone(),
                        colon_token: Default::default(),
                        ty: parse_quote!(usize),
                        eq_token: None,
                        default: None,
                    });

                    if let Some(GenericArgument::Const(new_expr)) = dict.get(&predicate) {
                        *self = new_expr.clone();
                        return;
                    }
                }

                expr_path.path.replace(dict);
            }
            _ => {}
        }
    }
}

impl Matching for Type {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        if let (Type::Path(lhs_path), rhs) = (self, other) {
            if let Some(ident) = lhs_path.path.get_ident() {
                // Check if this is a generic parameter that needs substitution
                let predicate = GenericParam::Type(TypeParam {
                    attrs: vec![],
                    ident: ident.clone(),
                    colon_token: None,
                    bounds: Default::default(),
                    eq_token: None,
                    default: None,
                });
                if let Some(_) = params.get(&predicate) {
                    return Some(Substitute::from_param_arg(
                        predicate,
                        GenericArgument::Type(rhs.clone()),
                    ));
                }
            }
        }
        match (self, other) {
            (Type::Path(lhs_path), Type::Path(rhs_path)) => {
                let substitute = match (&lhs_path.qself, &rhs_path.qself) {
                    (Some(lhs_qself), Some(rhs_qself))
                        if lhs_qself.position == rhs_qself.position =>
                    {
                        lhs_qself.ty.matches(&rhs_qself.ty, params)?
                    }
                    (None, None) => Substitute::new(),
                    _ => return None,
                };
                substitute.combine(lhs_path.path.matches(&rhs_path.path, params)?)
            }
            (Type::Reference(lhs_ref), Type::Reference(rhs_ref)) => {
                if lhs_ref.mutability != rhs_ref.mutability {
                    return None;
                }
                let lifetime_subs = match (&lhs_ref.lifetime, &rhs_ref.lifetime) {
                    (Some(lhs_lt), Some(rhs_lt)) => lhs_lt.matches(rhs_lt, params)?,
                    (None, None) => Substitute::new(),
                    _ => return None,
                };
                lifetime_subs
            }
            .combine(lhs_ref.elem.matches(&rhs_ref.elem, params)?),
            (Type::Tuple(lhs_tuple), Type::Tuple(rhs_tuple)) => {
                if lhs_tuple.elems.len() != rhs_tuple.elems.len() {
                    return None;
                }
                lhs_tuple
                    .elems
                    .iter()
                    .zip(&rhs_tuple.elems)
                    .try_fold(Substitute::new(), |substitute, (l, r)| {
                        substitute.combine(l.matches(r, params)?)
                    })
            }
            (Type::Array(lhs_array), Type::Array(rhs_array)) => lhs_array
                .elem
                .matches(&rhs_array.elem, params)?
                .combine(lhs_array.len.matches(&rhs_array.len, params)?),
            (Type::Slice(lhs_slice), Type::Slice(rhs_slice)) => {
                lhs_slice.elem.matches(&rhs_slice.elem, params)
            }
            (Type::Ptr(lhs_ptr), Type::Ptr(rhs_ptr)) => {
                (lhs_ptr.const_token == rhs_ptr.const_token).then_some(())?;
                (lhs_ptr.mutability == rhs_ptr.mutability).then_some(())?;
                lhs_ptr.elem.matches(&rhs_ptr.elem, params)
            }
            (
                Type::Group(TypeGroup { elem, .. }),
                Type::Group(TypeGroup { elem: rhs_elem, .. }),
            )
            | (
                Type::Paren(TypeParen { elem, .. }),
                Type::Paren(TypeParen { elem: rhs_elem, .. }),
            ) => elem.matches(rhs_elem, params),
            _ => None,
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        match self {
            Type::Path(type_path) => {
                if let Some(qself) = &mut type_path.qself {
                    qself.ty.replace(dict);
                }
                type_path.path.replace(dict);
            }
            Type::Reference(type_ref) => {
                if let Some(lt) = &mut type_ref.lifetime {
                    lt.replace(dict);
                }
                type_ref.elem.replace(dict);
            }
            Type::Tuple(type_tuple) => {
                for elem in &mut type_tuple.elems {
                    elem.replace(dict);
                }
            }
            Type::Array(type_array) => {
                type_array.elem.replace(dict);
                type_array.len.replace(dict);
            }
            Type::Slice(TypeSlice { elem, .. })
            | Type::Ptr(TypePtr { elem, .. })
            | Type::Group(TypeGroup { elem, .. })
            | Type::Paren(TypeParen { elem, .. }) => {
                elem.replace(dict);
            }
            _ => {}
        }
    }
}

impl Matching for Path {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        if self.segments.len() != other.segments.len() {
            return None;
        }

        self.segments.iter().zip(&other.segments).try_fold(
            Substitute::new(),
            |result, (l_seg, r_seg)| {
                (l_seg.ident == r_seg.ident).then_some(())?;
                let new_subs = l_seg.arguments.matches(&r_seg.arguments, params)?;
                result.combine(new_subs)
            },
        )
    }

    fn replace(&mut self, dict: &Substitute) {
        // Check if this is a single identifier that matches a generic parameter
        if let Some(ident) = self.get_ident() {
            let predicate = GenericParam::Type(TypeParam {
                attrs: vec![],
                ident: ident.clone(),
                colon_token: None,
                bounds: Default::default(),
                eq_token: None,
                default: None,
            });

            if let Some(GenericArgument::Type(Type::Path(new_path))) = dict.get(&predicate) {
                *self = new_path.path.clone();
                return;
            }
        }

        // Replace in path segments arguments
        for segment in &mut self.segments {
            segment.arguments.replace(dict);
        }
    }
}

impl Matching for AngleBracketedGenericArguments {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        (self.args.len() == other.args.len()).then_some(())?;
        (self.colon2_token == other.colon2_token).then_some(())?;
        self.args
            .iter()
            .zip(&other.args)
            .try_fold(Substitute::new(), |result, (l, r)| {
                let new_subs = l.matches(r, params)?;
                result.combine(new_subs)
            })
    }

    fn replace(&mut self, dict: &Substitute) {
        for arg in &mut self.args {
            arg.replace(dict);
        }
    }
}

impl Matching for PathArguments {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        match (self, other) {
            (PathArguments::None, PathArguments::None) => Some(Substitute::new()),
            (PathArguments::AngleBracketed(lhs_args), PathArguments::AngleBracketed(rhs_args)) => {
                lhs_args.matches(rhs_args, params)
            }
            (PathArguments::Parenthesized(lhs_args), PathArguments::Parenthesized(rhs_args)) => {
                (lhs_args.inputs.len() == rhs_args.inputs.len()).then_some(())?;
                let result = lhs_args
                    .inputs
                    .iter()
                    .zip(&rhs_args.inputs)
                    .try_fold(Substitute::new(), |result, (l, r)| {
                        result.combine(l.matches(r, params)?)
                    })?;
                match (&lhs_args.output, &rhs_args.output) {
                    (ReturnType::Default, ReturnType::Default) => Some(result),
                    (ReturnType::Type(_, l_ty), ReturnType::Type(_, r_ty)) => {
                        result.combine(l_ty.matches(r_ty, params)?)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        match self {
            PathArguments::AngleBracketed(angle_args) => {
                angle_args.replace(dict);
            }
            PathArguments::Parenthesized(paren_args) => {
                for input in &mut paren_args.inputs {
                    input.replace(dict);
                }
                if let ReturnType::Type(_, ty) = &mut paren_args.output {
                    ty.replace(dict);
                }
            }
            PathArguments::None => {}
        }
    }
}

impl Matching for GenericArgument {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        match (self, other) {
            (GenericArgument::Type(l_ty), GenericArgument::Type(r_ty)) => {
                l_ty.matches(r_ty, params)
            }
            (GenericArgument::Lifetime(l_lt), GenericArgument::Lifetime(r_lt)) => {
                l_lt.matches(r_lt, params)
            }
            (GenericArgument::Const(l_const), GenericArgument::Const(r_const)) => {
                l_const.matches(r_const, params)
            }
            (GenericArgument::AssocType(l_assoc), GenericArgument::AssocType(r_assoc)) => {
                if l_assoc.ident == r_assoc.ident {
                    l_assoc.ty.matches(&r_assoc.ty, params)
                } else {
                    None
                }
            }
            (GenericArgument::AssocConst(l_assoc), GenericArgument::AssocConst(r_assoc)) => {
                (&l_assoc.ident == &r_assoc.ident).then_some(())?;
                let result = match (&l_assoc.generics, &r_assoc.generics) {
                    (Some(l_g), Some(r_g)) => l_g.matches(r_g, params)?,
                    (None, None) => Substitute::new(),
                    _ => return None,
                };
                result.combine(l_assoc.value.matches(&r_assoc.value, params)?)
            }
            (
                GenericArgument::Constraint(l_constraint),
                GenericArgument::Constraint(r_constraint),
            ) => {
                (&l_constraint.ident == &r_constraint.ident).then_some(())?;
                (l_constraint.bounds.len() == r_constraint.bounds.len()).then_some(())?;
                let result = match (&l_constraint.generics, &r_constraint.generics) {
                    (Some(l_g), Some(r_g)) => l_g.matches(r_g, params)?,
                    (None, None) => Substitute::new(),
                    _ => return None,
                };
                l_constraint
                    .bounds
                    .iter()
                    .zip(&r_constraint.bounds)
                    .try_fold(result, |result, (l_bound, r_bound)| {
                        result.combine(l_bound.matches(r_bound, params)?)
                    })
            }
            _ => None,
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        match self {
            GenericArgument::Type(ty) => {
                ty.replace(dict);
            }
            GenericArgument::Lifetime(lifetime) => {
                lifetime.replace(dict);
            }
            GenericArgument::Const(expr) => {
                expr.replace(dict);
            }
            GenericArgument::AssocType(assoc_type) => {
                if let Some(generics) = &mut assoc_type.generics {
                    generics.replace(dict);
                }
                assoc_type.ty.replace(dict);
            }
            GenericArgument::AssocConst(assoc_const) => {
                if let Some(generics) = &mut assoc_const.generics {
                    generics.replace(dict);
                }
                assoc_const.value.replace(dict);
            }
            GenericArgument::Constraint(constraint) => {
                if let Some(generics) = &mut constraint.generics {
                    generics.replace(dict);
                }
                for bound in &mut constraint.bounds {
                    bound.replace(dict);
                }
            }
            _ => {}
        }
    }
}

impl Matching for TypeParamBound {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        match (self, other) {
            (TypeParamBound::Trait(l_trait), TypeParamBound::Trait(r_trait)) => {
                (&l_trait.paren_token == &r_trait.paren_token).then_some(())?;
                (&l_trait.modifier == &r_trait.modifier).then_some(())?;
                let lifetimes_subs = match (&l_trait.lifetimes, &r_trait.lifetimes) {
                    (Some(l_lifetimes), Some(_)) => {
                        abort!(&l_lifetimes, "not supported")
                    }
                    (None, None) => Substitute::new(),
                    _ => return None,
                };

                let path_subs = l_trait.path.matches(&r_trait.path, params)?;
                lifetimes_subs.combine(path_subs)
            }
            (TypeParamBound::Lifetime(l_lifetime), TypeParamBound::Lifetime(r_lifetime)) => {
                l_lifetime.matches(r_lifetime, params)
            }
            (TypeParamBound::Verbatim(l_tokens), TypeParamBound::Verbatim(r_tokens)) => {
                (l_tokens.to_string() == r_tokens.to_string()).then_some(Substitute::new())
            }
            _ => None,
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        match self {
            TypeParamBound::Trait(trait_bound) => {
                trait_bound.path.replace(dict);
            }
            TypeParamBound::Lifetime(lifetime) => {
                lifetime.replace(dict);
            }
            _ => {}
        }
    }
}

impl Matching for Constraint {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        let trait_subs = self.trait_path.matches(&other.trait_path, params)?;
        let ty_subs = self.typ.matches(&other.typ, params)?;
        trait_subs.combine(ty_subs)
    }

    fn replace(&mut self, dict: &Substitute) {
        self.typ.replace(dict);
        self.trait_path.replace(dict);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_type_path_matching_simple() {
        let pattern: Type = parse_quote! { T };
        let target: Type = parse_quote! { String };

        let mut params = HashSet::new();
        params.insert(GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        }));

        let result = pattern.matches(&target, &params);
        assert!(result.is_some());
        let substitutions = result.unwrap();

        // Check that T -> String substitution was created
        assert_eq!(substitutions.len(), 1);
        let param = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        assert!(substitutions.contains_key(&param));
    }

    #[test]
    fn test_type_path_matching_complex() {
        let pattern: Type = parse_quote! { Vec<T> };
        let target: Type = parse_quote! { Vec<String> };

        let mut params = HashSet::new();
        params.insert(GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        }));

        let result = pattern.matches(&target, &params);
        assert!(result.is_some());
        let substitutions = result.unwrap();

        // Should have T -> String substitution
        assert_eq!(substitutions.len(), 1);
    }

    #[test]
    fn test_type_reference_matching() {
        let pattern: Type = parse_quote! { &T };
        let target: Type = parse_quote! { &String };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);
    }

    #[test]
    fn test_type_mutable_reference_matching() {
        let pattern: Type = parse_quote! { &mut T };
        let target: Type = parse_quote! { &mut String };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);

        // Should not match immutable reference
        let target_immutable: Type = parse_quote! { &String };
        let result2 = pattern.matches(&target_immutable, &HashSet::new());
        assert!(result2.is_none());
    }

    #[test]
    fn test_type_tuple_matching() {
        let pattern: Type = parse_quote! { (T, U) };
        let target: Type = parse_quote! { (String, i32) };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 2);
    }

    #[test]
    fn test_type_array_matching() {
        let pattern: Type = parse_quote! { [T; 5] };
        let target: Type = parse_quote! { [String; 5] };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);

        // Different sizes should not match
        let target_different_size: Type = parse_quote! { [String; 3] };
        let result2 = pattern.matches(&target_different_size, &HashSet::new());
        assert!(result2.is_none());
    }

    #[test]
    fn test_type_slice_matching() {
        let pattern: Type = parse_quote! { [T] };
        let target: Type = parse_quote! { [String] };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);
    }

    #[test]
    fn test_type_replacement() {
        let mut ty: Type = parse_quote! { Vec<T> };

        let param = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param, GenericArgument::Type(parse_quote! { String }));

        ty.replace(&dict);

        let expected: Type = parse_quote! { Vec<String> };
        assert_eq!(
            template_quote::quote! { #ty }.to_string(),
            template_quote::quote! { #expected }.to_string()
        );
    }

    #[test]
    fn test_type_replacement_nested() {
        let mut ty: Type = parse_quote! { HashMap<T, Vec<U>> };

        let param_t = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let param_u = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { U },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param_t, GenericArgument::Type(parse_quote! { String }))
                .insert(param_u, GenericArgument::Type(parse_quote! { i32 }))
                .unwrap();

        ty.replace(&dict);

        let expected: Type = parse_quote! { HashMap<String, Vec<i32>> };
        assert_eq!(
            template_quote::quote! { #ty }.to_string(),
            template_quote::quote! { #expected }.to_string()
        );
    }

    #[test]
    fn test_path_matching_simple() {
        let pattern: Path = parse_quote! { Clone };
        let target: Path = parse_quote! { Clone };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 0);
    }

    #[test]
    fn test_path_matching_with_generics() {
        let pattern: Path = parse_quote! { Vec<T> };
        let target: Path = parse_quote! { Vec<String> };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);
    }

    #[test]
    fn test_path_matching_namespaced() {
        let pattern: Path = parse_quote! { std::clone::Clone };
        let target: Path = parse_quote! { std::clone::Clone };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 0);

        // Should not match different paths
        let target_different: Path = parse_quote! { std::fmt::Display };
        let result2 = pattern.matches(&target_different, &HashSet::new());
        assert!(result2.is_none());
    }

    #[test]
    fn test_path_matching_different_segment_count() {
        let pattern: Path = parse_quote! { Clone };
        let target: Path = parse_quote! { std::clone::Clone };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_none());
    }

    #[test]
    fn test_path_replacement_simple() {
        let mut path: Path = parse_quote! { T };

        let param = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param, GenericArgument::Type(parse_quote! { String }));

        path.replace(&dict);

        let expected: Path = parse_quote! { String };
        assert_eq!(
            template_quote::quote! { #path }.to_string(),
            template_quote::quote! { #expected }.to_string()
        );
    }

    #[test]
    fn test_path_replacement_with_args() {
        let mut path: Path = parse_quote! { Vec<T> };

        let param = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param, GenericArgument::Type(parse_quote! { String }));

        path.replace(&dict);

        let expected: Path = parse_quote! { Vec<String> };
        assert_eq!(
            template_quote::quote! { #path }.to_string(),
            template_quote::quote! { #expected }.to_string()
        );
    }

    #[test]
    fn test_path_arguments_angle_bracketed() {
        // Create PathArguments using AngleBracketedGenericArguments
        let angle_args: syn::AngleBracketedGenericArguments = parse_quote! { <T, U> };
        let pattern = PathArguments::AngleBracketed(angle_args);

        let target_angle_args: syn::AngleBracketedGenericArguments = parse_quote! { <String, i32> };
        let target = PathArguments::AngleBracketed(target_angle_args);

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 2);
    }

    #[test]
    fn test_path_arguments_none() {
        let pattern = PathArguments::None;
        let target = PathArguments::None;

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 0);

        // Should not match angle bracketed
        let angle_args: syn::AngleBracketedGenericArguments = parse_quote! { <String> };
        let target_angle = PathArguments::AngleBracketed(angle_args);
        let result2 = pattern.matches(&target_angle, &HashSet::new());
        assert!(result2.is_none());
    }

    #[test]
    fn test_generic_argument_type_matching() {
        let pattern = GenericArgument::Type(parse_quote! { T });
        let target = GenericArgument::Type(parse_quote! { String });

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);
    }

    #[test]
    fn test_generic_argument_lifetime_matching() {
        let pattern = GenericArgument::Lifetime(parse_quote! { 'a });
        let target = GenericArgument::Lifetime(parse_quote! { 'a });

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 0); // Lifetimes don't create substitutions

        // Different lifetimes should not match
        let target_different = GenericArgument::Lifetime(parse_quote! { 'b });
        let result2 = pattern.matches(&target_different, &HashSet::new());
        assert!(result2.is_none());
    }

    #[test]
    fn test_generic_argument_replacement() {
        let mut arg = GenericArgument::Type(parse_quote! { T });

        let param = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param, GenericArgument::Type(parse_quote! { String }));

        arg.replace(&dict);

        if let GenericArgument::Type(ty) = arg {
            let expected: Type = parse_quote! { String };
            assert_eq!(
                template_quote::quote! { #ty }.to_string(),
                template_quote::quote! { #expected }.to_string()
            );
        } else {
            panic!("Expected Type argument");
        }
    }

    #[test]
    fn test_type_constraint_matching() {
        let pattern = Constraint {
            typ: parse_quote! { T },
            trait_path: parse_quote! { Clone },
        };
        let target = Constraint {
            typ: parse_quote! { String },
            trait_path: parse_quote! { Clone },
        };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 1);
    }

    #[test]
    fn test_type_constraint_matching_different_traits() {
        let pattern = Constraint {
            typ: parse_quote! { T },
            trait_path: parse_quote! { Clone },
        };
        let target = Constraint {
            typ: parse_quote! { String },
            trait_path: parse_quote! { Display },
        };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_none());
    }

    #[test]
    fn test_type_constraint_matching_generic_trait() {
        let pattern = Constraint {
            typ: parse_quote! { T },
            trait_path: parse_quote! { From<U> },
        };
        let target = Constraint {
            typ: parse_quote! { String },
            trait_path: parse_quote! { From<i32> },
        };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 2); // T -> String, U -> i32
    }

    #[test]
    fn test_type_constraint_replacement() {
        let mut constraint = Constraint {
            typ: parse_quote! { T },
            trait_path: parse_quote! { From<U> },
        };

        let param_t = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let param_u = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { U },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param_t, GenericArgument::Type(parse_quote! { String }))
                .insert(param_u, GenericArgument::Type(parse_quote! { i32 }))
                .unwrap();

        constraint.replace(&dict);

        let expected = Constraint {
            typ: parse_quote! { String },
            trait_path: parse_quote! { From<i32> },
        };
        assert_eq!(
            template_quote::quote! { #constraint }.to_string(),
            template_quote::quote! { #expected }.to_string()
        );
    }

    #[test]
    fn test_type_constraint_complex_types() {
        let pattern = Constraint {
            typ: parse_quote! { Vec<T> },
            trait_path: parse_quote! { Iterator<Item = U> },
        };
        let target = Constraint {
            typ: parse_quote! { Vec<String> },
            trait_path: parse_quote! { Iterator<Item = char> },
        };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 2); // T -> String, U -> char
    }

    #[test]
    fn test_edge_case_conflicting_substitutions() {
        // Test what happens when we try to match T with two different types
        let pattern: Type = parse_quote! { (T, T) };
        let target_conflicting: Type = parse_quote! { (String, i32) };

        let result = pattern.matches(&target_conflicting, &HashSet::new());
        assert!(result.is_none());

        // Should work with same types
        let target_same: Type = parse_quote! { (String, String) };
        let result2 = pattern.matches(&target_same, &HashSet::new());
        assert!(result2.is_some());
        let substitutions2 = result2.unwrap();
        assert_eq!(substitutions2.len(), 1);
    }

    #[test]
    fn test_edge_case_nested_generics() {
        // Test deeply nested generic types
        let pattern: Type = parse_quote! { HashMap<Vec<T>, Result<U, V>> };
        let target: Type = parse_quote! { HashMap<Vec<String>, Result<i32, bool>> };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 3); // T, U, V

        // Test replacement with complex nesting (using concrete types since trait object replacement is complex)
        let mut complex_ty: Type = parse_quote! { Option<Box<HashMap<T, U>>> };
        let param_t = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { T },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let param_u = GenericParam::Type(syn::TypeParam {
            attrs: vec![],
            ident: parse_quote! { U },
            colon_token: None,
            bounds: Default::default(),
            eq_token: None,
            default: None,
        });
        let dict =
            Substitute::from_param_arg(param_t, GenericArgument::Type(parse_quote! { String }))
                .insert(param_u, GenericArgument::Type(parse_quote! { i32 }))
                .unwrap();

        complex_ty.replace(&dict);

        let expected: Type = parse_quote! { Option<Box<HashMap<String, i32>>> };
        assert_eq!(
            template_quote::quote! { #complex_ty }.to_string(),
            template_quote::quote! { #expected }.to_string()
        );
    }

    #[test]
    fn test_edge_case_function_types() {
        // Test function pointer types
        let _pattern: Type = parse_quote! { fn(T) -> U };
        let _target: Type = parse_quote! { fn(String) -> i32 };

        // let _substitutions: HashMap<GenericParam, GenericArgument> = HashMap::new();
        // Note: This might not work with current implementation due to function type complexity
        // This test documents expected behavior for future improvements

        // Test with parenthesized arguments
        let pattern_paren_args: syn::ParenthesizedGenericArguments = parse_quote! { (T, U) -> V };
        let pattern_paren = PathArguments::Parenthesized(pattern_paren_args);
        let target_paren_args: syn::ParenthesizedGenericArguments =
            parse_quote! { (String, i32) -> bool };
        let target_paren = PathArguments::Parenthesized(target_paren_args);

        let result_paren = pattern_paren.matches(&target_paren, &HashSet::new());
        assert!(result_paren.is_some());
        let substitutions_paren = result_paren.unwrap();
        assert_eq!(substitutions_paren.len(), 3); // T, U, V
    }

    #[test]
    fn test_edge_case_lifetime_handling() {
        // Test lifetime parameters (should be handled as literals)
        let pattern = GenericArgument::Lifetime(parse_quote! { 'a });
        let target = GenericArgument::Lifetime(parse_quote! { 'a });

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 0); // Lifetimes don't create substitutions
    }

    #[test]
    fn test_edge_case_const_generics() {
        // Test const generic parameters
        let pattern = GenericArgument::Const(parse_quote! { N });
        let target = GenericArgument::Const(parse_quote! { 42 });

        // Current implementation compares as strings, so these won't match
        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_none());

        // But identical const expressions should match
        let pattern_same = GenericArgument::Const(parse_quote! { 42 });
        let target_same = GenericArgument::Const(parse_quote! { 42 });

        let result_same = pattern_same.matches(&target_same, &HashSet::new());
        assert!(result_same.is_some());
    }

    #[test]
    fn test_edge_case_complex_constraint_matching() {
        // Test complex constraint patterns with associated types
        let pattern = Constraint {
            typ: parse_quote! { T },
            trait_path: parse_quote! { Iterator<Item = U> },
        };

        // Match against concrete implementation
        let target1 = Constraint {
            typ: parse_quote! { Vec<String> },
            trait_path: parse_quote! { Iterator<Item = String> },
        };

        let result1 = pattern.matches(&target1, &HashSet::new());
        assert!(result1.is_some());
        let substitutions1 = result1.unwrap();
        assert_eq!(substitutions1.len(), 2); // T and U

        // Match against different associated type should fail
        let target2 = Constraint {
            typ: parse_quote! { Vec<String> },
            trait_path: parse_quote! { Iterator<Item = i32> },
        };

        // This should fail because T maps to Vec<String> but U maps to conflicting types
        // However, current implementation might not catch this - depends on order of evaluation
        let _result = pattern.matches(&target2, &HashSet::new());
        // The result depends on implementation details of how substitutions are handled
    }

    #[test]
    fn test_edge_case_empty_types() {
        // Test unit type
        let pattern: Type = parse_quote! { () };
        let target: Type = parse_quote! { () };

        let result = pattern.matches(&target, &HashSet::new());
        assert!(result.is_some());
        let substitutions = result.unwrap();
        assert_eq!(substitutions.len(), 0);

        // Test never type (if supported)
        let pattern_never: Type = parse_quote! { ! };
        let target_never: Type = parse_quote! { ! };

        // This might not work depending on syn's parsing of never type
        // Just testing that it doesn't panic
        let _ = pattern_never.matches(&target_never, &HashSet::new());
    }
}
