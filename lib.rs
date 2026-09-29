/// Attribute macro for breaking circular trait dependencies.
///
/// - On a `mod`, it rewrites the impls of the module so that mutually recursive `where` bounds
///   resolve, and exports the predicates of the impls on local types so that other
///   `#[coinduction]` modules can see through bounds on them. The traits to work on are marked
///   inside the module with `#[coinduction] use path::{TraitA, TraitB};` or
///   `#[coinduction] trait TraitC { ... }`; if none are marked, every trait implemented in the
///   module is used. Accepted arguments are `marker = <path>` (a type used to refer to relative
///   paths in exported impls) and `coinduction = <path>` (the path to this crate).
/// - On a `trait`, it makes the trait usable from `#[coinduction]` modules. Arguments of the form
///   `(<pattern>) => { <predicates> }` give the predicates of impls of the trait for types
///   matching `<pattern>`, and `macro_name = <ident>` renames the macro generated for the trait
///   (by default, the trait name).
/// - On a `struct`, `enum` or `union` in a `#[coinduction]` module, `macro_name = <ident>` renames
///   the macro generated for the type (by default, the type name).
///
/// `#[coinduction]` modules look up these macros by the trait or type name, so a renamed macro
/// must be imported under that name where the trait or type is used.
pub use coinduction_macro::coinduction;

#[doc(hidden)]
pub use coinduction_macro::__next_step;

#[doc(hidden)]
/// Trait for referencing types with markers
pub trait TypeRef<const RANDOM: u64, const IX0: usize, const IX: usize, ARG: ?Sized> {
    type Type: ?Sized;
}
