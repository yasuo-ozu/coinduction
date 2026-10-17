use core::ops::Deref;
use proc_macro_error::abort;
use std::collections::{HashMap, HashSet};
use syn::{visit::Visit, *};

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

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Substitute(pub HashMap<GenericParam, GenericArgument>);

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

/// The const parameter in `params` named `ident`, whatever its type.
fn const_param<'a>(ident: &Ident, params: &'a HashSet<GenericParam>) -> Option<&'a GenericParam> {
    params
        .iter()
        .find(|param| matches!(param, GenericParam::Const(cp) if &cp.ident == ident))
}

/// The value bound to the const parameter named `ident` in `dict`.
fn const_value<'a>(ident: &Ident, dict: &'a Substitute) -> Option<&'a Expr> {
    dict.iter().find_map(|(param, arg)| match (param, arg) {
        (GenericParam::Const(cp), GenericArgument::Const(expr)) if &cp.ident == ident => Some(expr),
        _ => None,
    })
}

/// `expr` without the invisible groups that macro expansion may put around it.
fn unwrap_expr_group(expr: &Expr) -> &Expr {
    match expr {
        Expr::Group(group) => unwrap_expr_group(&group.expr),
        expr => expr,
    }
}

/// A const argument written as a type (`N` in `A<N>` is parsed as a type), as an expression.
fn type_as_expr(ty: &Type) -> Option<Expr> {
    match ty {
        Type::Path(TypePath { qself: None, path }) => Some(Expr::Path(ExprPath {
            attrs: vec![],
            qself: None,
            path: path.clone(),
        })),
        Type::Group(group) => type_as_expr(&group.elem),
        _ => None,
    }
}

impl Matching for Expr {
    fn matches(&self, other: &Self, params: &HashSet<GenericParam>) -> Option<Substitute> {
        let (lhs, rhs) = (unwrap_expr_group(self), unwrap_expr_group(other));
        if let Expr::Path(ExprPath { qself: None, path, .. }) = lhs {
            if let Some(param) = path.get_ident().and_then(|ident| const_param(ident, params)) {
                return Some(Substitute::from_param_arg(
                    param.clone(),
                    GenericArgument::Const(rhs.clone()),
                ));
            }
        }
        match (lhs, rhs) {
            (Expr::Path(l_path), Expr::Path(r_path)) if l_path.qself.is_none() && r_path.qself.is_none() => {
                l_path.path.matches(&r_path.path, params)
            }
            (Expr::Block(l_block), r) if l_block.block.stmts.len() == 1 => match &l_block.block.stmts[0] {
                Stmt::Expr(l_expr, None) => match r {
                    Expr::Block(r_block) if r_block.block.stmts.len() == 1 => match &r_block.block.stmts[0] {
                        Stmt::Expr(r_expr, None) => l_expr.matches(r_expr, params),
                        _ => None,
                    },
                    r => l_expr.matches(r, params),
                },
                _ => None,
            },
            (Expr::Paren(l_paren), Expr::Paren(r_paren)) => l_paren.expr.matches(&r_paren.expr, params),
            (l, r) => {
                // Anything else, such as a literal, matches when it is written the same way. The
                // impl renames its generic parameters, so they never appear in the target.
                (crate::comparison_key(l) == crate::comparison_key(r)).then(Substitute::new)
            }
        }
    }

    fn replace(&mut self, dict: &Substitute) {
        struct Replacer<'a>(&'a Substitute);
        impl syn::visit_mut::VisitMut for Replacer<'_> {
            fn visit_expr_mut(&mut self, expr: &mut Expr) {
                if let Expr::Path(ExprPath { qself: None, path, .. }) = expr {
                    if let Some(value) = path.get_ident().and_then(|ident| const_value(ident, self.0)) {
                        *expr = value.clone();
                        return;
                    }
                }
                syn::visit_mut::visit_expr_mut(self, expr);
            }
            fn visit_type_mut(&mut self, ty: &mut Type) {
                ty.replace(self.0);
            }
        }
        use syn::visit_mut::VisitMut;
        Replacer(dict).visit_expr_mut(self);
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
                if let (None, Some(ident)) = (&type_path.qself, type_path.path.get_ident()) {
                    let predicate = GenericParam::Type(TypeParam {
                        attrs: vec![],
                        ident: ident.clone(),
                        colon_token: None,
                        bounds: Default::default(),
                        eq_token: None,
                        default: None,
                    });
                    if let Some(GenericArgument::Type(new_ty)) = dict.get(&predicate) {
                        *self = new_ty.clone();
                        return;
                    }
                }
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
                (crate::comparison_key(&l_seg.ident) == crate::comparison_key(&r_seg.ident))
                    .then_some(())?;
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
            // A const parameter, parsed as a type (`N` in `A<N>`): it binds to a const argument,
            // or to a path (`A<M>`, also parsed as a type)
            (GenericArgument::Type(l_ty), r)
                if matches!(
                    type_as_expr(l_ty),
                    Some(Expr::Path(ExprPath { ref path, .. }))
                        if path.get_ident().is_some_and(|ident| const_param(ident, params).is_some())
                ) =>
            {
                let r_expr = match r {
                    GenericArgument::Const(expr) => expr.clone(),
                    GenericArgument::Type(ty) => type_as_expr(ty)?,
                    _ => return None,
                };
                type_as_expr(l_ty).unwrap().matches(&r_expr, params)
            }
            (GenericArgument::Type(l_ty), GenericArgument::Type(r_ty)) => {
                l_ty.matches(r_ty, params)
            }
            (GenericArgument::Const(l_const), GenericArgument::Type(r_ty)) => {
                l_const.matches(&type_as_expr(r_ty)?, params)
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
            GenericArgument::Type(Type::Path(TypePath { qself: None, path }))
                if path.get_ident().is_some_and(|ident| const_value(ident, dict).is_some()) =>
            {
                let value = const_value(path.get_ident().unwrap(), dict).unwrap().clone();
                *self = GenericArgument::Const(value);
            }
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
