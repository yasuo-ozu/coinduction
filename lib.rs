/// Attribute macro for breaking circular trait dependencies, applied to a `mod`.
///
/// It rewrites the impls of the module so that mutually recursive `where` bounds resolve. The
/// traits to work on are marked inside the module with `#[coinduction] use path::{TraitA, TraitB};`
/// or by announcing them there (`#[announce] trait TraitC { ... }`); if none are marked, every
/// trait implemented in the module is used. Accepted arguments are `marker = <path>` (a type used
/// to refer to relative paths in the impls of announced types) and `coinduction = <path>` (the path
/// to this crate).
///
/// Traits and types are read from other modules and crates through their
/// [`announce`](https://docs.rs/announce) trampolines:
///
/// - A trait that `#[coinduction]` modules work on must be announced, with `#[announce]`. Rules
///   for the impls of the trait on types that no `#[coinduction]` module describes (tuples, arrays,
///   foreign types) are given as `META`:
///   `#[announce(meta = coinduction::rules((<pattern>) => { <predicates> }, ...))]`, where
///   `<pattern>` is a `macro_rules!` pattern matched against the type.
/// - A struct, enum or union in a `#[coinduction]` module that is announced, with `#[announce]` or
///   `#[derive(Announce)]`, carries the predicates of its impls in its announcement, so that other
///   `#[coinduction]` modules can see through bounds on it.
///
/// `#[coinduction]` modules look up the trampolines by the trait or type name, so a trampoline
/// renamed with `#[announce(macro_name = ...)]` must be imported under that name where the trait
/// or type is used.
pub use coinduction_macro::coinduction;

#[doc(hidden)]
pub use coinduction_macro::__next_step;

#[doc(hidden)]
/// Trait for referencing types with markers
pub trait TypeRef<const RANDOM: u64, const IX0: usize, const IX: usize, ARG: ?Sized> {
    type Type: ?Sized;
}
