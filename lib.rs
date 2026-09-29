/// Attribute macro for marking trait definitions that will support coinduction.
///
/// This is currently an empty attribute macro that serves as a placeholder for future
/// implementation of coinductive trait definition functionality.
pub use coinduction_macro::traitdef;

/// Attribute macro for marking type definitions involved in circular references.
///
/// This is currently an empty attribute macro that serves as a placeholder for future
/// implementation of circular type reference functionality.
pub use coinduction_macro::typedef;

/// Attribute macro for enabling coinductive reasoning on specific items.
///
/// This is currently an empty attribute macro that serves as a placeholder for future
/// implementation of coinductive reasoning functionality.
pub use coinduction_macro::coinduction;

#[doc(hidden)]
pub use coinduction_macro::__next_step;

#[doc(hidden)]
/// Trait for referencing types with markers
pub trait TypeRef<const RANDOM: u64, const IX0: usize, const IX: usize, ARG: ?Sized> {
    type Type: ?Sized;
}
