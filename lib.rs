#![doc = include_str!("./README.md")]
#[doc(hidden)]
pub use coinduction_macro::{__finalize, __internal};

/// Coinduction attribute macro for breaking circular trait dependencies.
///
/// This macro applies coinduction to break circular trait bound dependencies that would
/// otherwise cause compilation errors. When applied to a module, it removes circular
/// where clauses from trait implementations, allowing mutually recursive trait bounds
/// to compile successfully.
///
/// ## Syntax
///
/// ```rust
/// # use coinduction::*;
/// #[coinduction]
/// mod my_module {
///     // Circular trait implementations
/// }
/// ```
///
/// Or with specific traits (specifying valid trait paths without type arguments):
///
/// ```rust
/// # use coinduction::*;
/// # #[traitdef]
/// # trait TraitA {}
/// # #[traitdef]
/// # trait TraitB {}
/// #[coinduction(super::TraitA, super::TraitB)]
/// mod my_module {
///     // Implementations using TraitA and TraitB
/// }
/// ```
///
/// ## Trait Path Specification
///
/// When specifying traits as arguments to the `#[coinduction]` macro:
/// - Use **valid trait paths** that are resolvable within the module context
/// - **Do not include type arguments** (e.g., use `MyTrait`, not `MyTrait<T>`)
/// - Use appropriate path qualifications (e.g., `super::MyTrait`, `crate::MyTrait`)
///
/// Examples:
/// ```rust,ignore
/// #[coinduction(MyTrait)]                    // Trait in current module scope
/// #[coinduction(super::MyTrait)]             // Trait in parent module
/// #[coinduction(crate::my_mod::MyTrait)]     // Fully qualified path
/// #[coinduction(TraitA, TraitB, TraitC)]     // Multiple traits
/// ```
///
/// ## Example
///
/// ```rust
/// # use coinduction::*;
///
/// #[coinduction]
/// mod circular {
///
///     #[coinduction::traitdef]
///     pub trait Display {
///         fn display(&self) -> String;
///     }
///
///     pub struct TypeA;
///     pub struct TypeB;
///
///     impl Display for TypeA
///     where
///         TypeB: Display,  // This circular dependency is resolved by coinduction
///     {
///         fn display(&self) -> String {
///             "TypeA".to_string()
///         }
///     }
///
///     impl Display for TypeB
///     where
///         TypeA: Display,  // This circular dependency is resolved by coinduction
///     {
///         fn display(&self) -> String {
///             "TypeB".to_string()
///         }
///     }
/// }
/// ```
///
/// ## How it works
///
/// The `coinduction` macro processes trait implementations within the module and:
/// 1. Identifies circular trait bound dependencies
/// 2. Removes problematic where clauses that would cause infinite recursion
/// 3. Allows the Rust compiler to accept mutually recursive trait implementations
///
/// This is particularly useful for:
/// - Mutually recursive data structures
/// - Circular trait dependencies
/// - Complex type relationships that naturally form cycles
pub use coinduction_macro::coinduction;

/// Trait definition macro for creating traits with marker types.
///
/// This macro extends regular trait definitions by automatically generating marker types
/// that can be used with the [`TypeRef`] trait for type-level programming and with
/// coinduction for breaking circular dependencies.
///
/// ## Syntax
///
/// ```rust
/// # use coinduction::*;
/// #[traitdef]
/// trait MyTrait {
///     fn my_method(&self);
/// }
/// ```
///
/// ## Example
///
/// ```rust
/// # use coinduction::*;
/// #[traitdef]
/// trait Display {
///     fn display(&self) -> String;
/// }
///
/// // The macro generates a marker type that can be used with TypeRef
/// struct MyType;
///
/// impl Display for MyType {
///     fn display(&self) -> String {
///         "MyType".to_string()
///     }
/// }
///
/// // Generated marker can be used for type-level programming
/// fn test_display() {
///     let instance = MyType;
///     assert_eq!(instance.display(), "MyType");
/// }
/// ```
///
/// ## Generated Code
///
/// The `traitdef` macro generates:
/// - The original trait definition
/// - A marker type (default name based on trait name + "Marker")
/// - Implementations enabling the marker to work with [`TypeRef`]
///
/// ## Marker Traits and Type-Level Programming
///
/// You can specify marker traits for trait definitions.
/// For more information about marker traits and their role in avoiding type leakage
/// in complex generic scenarios, see the [`type-leak`](https://docs.rs/type-leak/) crate
/// documentation, which provides detailed explanations of how marker types help
/// maintain type safety and prevent unintended type exposure in generic contexts.
///
/// ```rust
/// # use coinduction::*;
/// # struct MyMarker;
/// #[traitdef(marker = MyMarker)]
/// trait MyTrait {
///     fn my_method(&self);
/// }
/// ```
///
/// ## Use Cases
///
/// - Creating traits that work with coinduction
/// - Type-level programming with marker types
/// - Building complex trait hierarchies with circular dependencies
/// - Enabling compile-time type checking and inference
/// - Preventing type leakage in complex generic scenarios
pub use coinduction_macro::traitdef;

/// Type definition macro for creating modules with trait implementations.
///
/// This macro creates a module containing type definitions and their trait implementations,
/// with support for complex where clauses and marker types. It's designed to work
/// seamlessly with [`traitdef`] and [`coinduction`] for creating sophisticated type systems.
///
/// ## Syntax
///
/// ```rust
/// # use coinduction::*;
/// # #[traitdef]
/// # trait TraitA {}
/// # #[traitdef]
/// # trait TraitB {}
/// #[typedef(TraitA, TraitB)]
/// mod my_types {
///     // Type definitions and implementations
/// }
/// ```
///
/// ## Trait Path Specification
///
/// When specifying traits as arguments to the `#[typedef]` macro:
/// - Use **exact path structures** that match your impl blocks (no path resolution)
/// - **Do not include type arguments** (e.g., use `MyTrait`, not `MyTrait<T>`)
/// - Maintain **consistent path style** throughout the module
///
/// Examples:
/// ```rust,ignore
/// #[typedef(MyTrait)]                    // Local trait, matches: impl MyTrait
/// #[typedef(super::MyTrait)]             // Parent module, matches: impl super::MyTrait
/// #[typedef(crate::my_mod::MyTrait)]     // Qualified path, matches: impl crate::my_mod::MyTrait
/// ```
///
/// **Important**: The macro uses literal token matching - `MyTrait` and `super::MyTrait` 
/// are treated as different paths even if they resolve to the same trait.
///
/// ## Example
///
/// ```rust
/// # use coinduction::*;
/// # use std::fmt::Debug;
///
/// #[typedef(Process<T>, Validate<T>)]
/// mod processing_types {
///     use coinduction::*;
///     use std::fmt::Debug;
///
///     #[traitdef]
///     trait Process<T> {
///         fn process(&self, input: T) -> T;
///     }
///
///     #[traitdef]
///     trait Validate<T> {
///         fn validate(&self, value: &T) -> bool;
///     }
///
///     pub struct Processor<T> {
///         pub name: String,
///         pub _phantom: std::marker::PhantomData<T>,
///     }
///
///     pub struct Validator<T> {
///         pub threshold: i32,
///         pub _phantom: std::marker::PhantomData<T>,
///     }
///
///     // Implementation with where clause containing trait predicates
///     impl<T> Process<T> for Processor<T>
///     where
///         T: Clone + Debug,
///         Validator<T>: Validate<T>,  // Predicate using the trait
///     {
///         fn process(&self, input: T) -> T {
///             input.clone()
///         }
///     }
///
///     // Implementation with where clause containing trait predicates
///     impl<T> Validate<T> for Validator<T>
///     where
///         T: PartialOrd + Debug,
///         Processor<T>: Process<T>,  // Predicate using the trait
///     {
///         fn validate(&self, _value: &T) -> bool {
///             true
///         }
///     }
/// }
/// ```
///
/// ## Generated Code
///
/// The `typedef` macro generates:
/// - The module with type definitions
/// - Marker types for the module
/// - Helper implementations for working with [`TypeRef`]
///
/// ## Marker Traits and Type-Level Programming
///
/// You can specify marker traits for trait definitions.
/// For more information about marker traits and their role in avoiding type leakage
/// in complex generic scenarios, see the [`type-leak`](https://docs.rs/type-leak/) crate
/// documentation, which provides detailed explanations of how marker types help
/// maintain type safety and prevent unintended type exposure in generic contexts.
///
/// ```rust
/// # use coinduction::*;
/// # #[traitdef]
/// # trait TraitA {}
/// # struct MyMarker;
/// #[typedef(TraitA, marker = MyMarker)]
/// mod my_types {
///     // Type definitions and implementations
/// }
/// ```
///
/// ## Use Cases
///
/// - Organizing related types and their trait implementations
/// - Creating type families with shared behaviors
/// - Building modular type systems with clear boundaries
/// - Enabling type-level programming with organized marker types
pub use coinduction_macro::typedef;

#[doc(hidden)]
/// Trait for referencing types with markers
pub trait TypeRef<T: ?Sized> {
    type Type: ?Sized;
}
