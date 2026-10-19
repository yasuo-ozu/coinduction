/// Attribute macro for breaking circular trait dependencies, applied to a `mod`.
///
/// It rewrites the impls of the module so that mutually recursive `where` bounds resolve. The
/// traits to work on are marked inside the module with `#[coinduction] use path::{TraitA, TraitB};`
/// or by announcing them there (`#[announce] trait TraitC { ... }`); if none are marked, every
/// trait implemented in the module is used. The only argument is `coinduction = <path>` (the path
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
/// - A struct, enum or union in a `#[coinduction]` module that is announced, with `#[announce]`,
///   carries the predicates of its impls of the traits the module works on in its announcement, so
///   that other `#[coinduction]` modules can see through bounds on it.
///
/// `#[coinduction]` modules look up the trampolines by the trait or type name, so a trampoline
/// renamed with `#[announce(macro_name = ...)]` must be imported under that name where the trait
/// or type is used.
pub use coinduction_macro::coinduction;

#[doc(hidden)]
pub use coinduction_macro::__next_step;

/// The [`announce`](https://docs.rs/announce) crate, so that a crate using `#[coinduction]` need
/// not depend on it: `#[coinduction::announce::announce]`. In a `#[coinduction]` module, the
/// announcements through this path get `crate = ::coinduction::announce` (with the `coinduction`
/// argument of the module); elsewhere, write it:
/// `#[coinduction::announce::announce(crate = ::coinduction::announce)]`.
///
/// A `#[coinduction]` module recognizes announce's attribute under the name `announce` or through
/// this path (also imported under other names), not under a name given to announce in
/// `Cargo.toml` (`ann = { package = "announce", .. }`): with such a rename, announce through this
/// path.
pub use announce;

#[cfg(doctest)]
#[doc = include_str!("README.md")]
struct ReadmeDoctests;
