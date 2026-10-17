# Coinduction

[![Crates.io](https://img.shields.io/crates/v/coinduction.svg)](https://crates.io/crates/coinduction)
[![Documentation](https://docs.rs/coinduction/badge.svg)](https://docs.rs/coinduction)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Attribute macros for defining circular type references and recursive trait implementations in Rust.

## Overview

Coinduction is a Rust library that provides an attribute macro, **`#[coinduction]`**, for
working with circular type dependencies and mutually recursive trait implementations. On a
**module**, it breaks circular trait dependencies between the impls in the module.

Traits and types are made visible to `#[coinduction]` modules elsewhere (other modules, other
crates) with the [`announce`](https://crates.io/crates/announce) crate:

- A trait that `#[coinduction]` modules work on is announced with `#[announce]`. Its rules for
  types that no `#[coinduction]` module describes (tuples, arrays, foreign types) go in the
  announcement: `#[announce(meta = coinduction::rules(...))]`.
- A struct, enum or union in a `#[coinduction]` module is exported by announcing it with
  `#[announce]`. The module adds the predicates of the type's impls to the announcement, so that
  other `#[coinduction]` modules can see through bounds on it.

`#[coinduction]` recognizes announce's attribute as `announce`, as a path from the announce crate
(also when it is renamed in `Cargo.toml`) or from `coinduction::announce`, under a name imported
with `use announce::announce as ..`, and inside `#[cfg_attr(..)]`.

The library enables you to define types and traits with circular dependencies that would normally be impossible in Rust's type system, making it particularly useful for recursive data structures, graph-like types, and complex generic scenarios.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
coinduction = "0.2.0"
announce = "0.2.0"
```

A crate that only defines traits (with their rules) depends on `announce` alone.

`coinduction` re-exports announce as `coinduction::announce`, so a crate using `#[coinduction]` can
leave out the `announce` dependency. In a `#[coinduction]` module, announcements through that path
(`#[coinduction::announce::announce]`, or a name imported from there) get `crate = ::coinduction::announce`, the path announce's generated code
uses. Outside a module, write it:
`#[coinduction::announce::announce(crate = ::coinduction::announce)]`.

## Why Coinduction?

Without coinduction, circular trait dependencies cause compilation errors. Here's what happens when trying to create a simple calculator parser:

```rust,compile_fail
trait Evaluate {
    fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32;
}
pub struct Expr;
pub struct Term;

// ERROR: Cannot prove Term: Evaluate
impl Evaluate for Expr
where
    Term: Evaluate,  // Expr depends on Term...
{
    fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
        let left_val = Term.evaluate(input, index);
        let op = input[*index];
        *index += 1;
        let right_val = Term.evaluate(input, index);
        match op {
            "+" => left_val + right_val,
            "-" => left_val - right_val,
            _ => left_val,
        }
    }
}

// ERROR: Cannot prove Expr: Evaluate  
impl Evaluate for Term
where
    Expr: Evaluate,  // ...and Term depends on Expr!
{
    fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
        let token = input[*index];
        *index += 1;
        if token == "(" {
            let result = Expr.evaluate(input, index);
            *index += 1; // skip closing ')'
            result
        } else {
            token.parse::<i32>().unwrap()
        }
    }
}
```

The `#[coinduction]` macro solves this by breaking the circular dependency cycle.

## Examples

### Basic Circular Dependencies

This example shows how to break circular trait dependencies using `#[coinduction]` with a simple calculator:

```rust
# use announce::announce;
# use coinduction::*;
#[announce]
trait Evaluate {
    fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32;
}

#[coinduction]
mod calculator {
    #[coinduction]
    use super::Evaluate;

    pub struct Expr;
    pub struct Term;

    impl Evaluate for Expr
    where
        Term: Evaluate,
    {
        // same as above
#        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
#            let left_val = Term.evaluate(input, index);
#            let op = input[*index];
#            *index += 1;
#            let right_val = Term.evaluate(input, index);
#            match op {
#                "+" => left_val + right_val,
#                "-" => left_val - right_val,
#                _ => left_val,
#            }
#        }
    }

    impl Evaluate for Term
    where
        Expr: Evaluate,
    {
        // same as above
#        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 {
#            let token = input[*index];
#            *index += 1;
#            if token == "(" {
#                let result = Expr.evaluate(input, index);
#                *index += 1; // skip closing ')'
#                result
#            } else {
#                token.parse::<i32>().unwrap()
#            }
#        }
    }
}

fn main() {
    let input = vec!["2", "+", "3"];
    let mut index = 0;
    assert_eq!(calculator::Expr.evaluate(&input, &mut index), 5);
}
```

The traits to work on are marked inside the module, either by importing them with
`#[coinduction] use ...;` or by announcing them in the module with `#[announce] trait ...`:

```rust
# use coinduction::*;
#[coinduction]
mod calculator {
    #[announce::announce]
    pub trait Evaluate {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32;
    }

    pub struct Expr;
    pub struct Term;

    impl Evaluate for Expr
    where
        Term: Evaluate,
    {
        // ...
#        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 { 0 }
    }

    impl Evaluate for Term
    where
        Expr: Evaluate,
    {
        // ...
#        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32 { 0 }
    }
}
# fn main() {}
```

If no trait is marked, every trait implemented in the module is used.

### Circular Dependencies Across Modules

A type announced in a `#[coinduction]` module can be used in the bounds of another
`#[coinduction]` module, also in another crate:

```rust
# use announce::announce;
# use coinduction::*;
#[announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::Count;
    use announce::announce;

    // Announced: its impls are visible to other `#[coinduction]` modules
    #[announce]
    pub struct Boxed<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Boxed<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }
}

#[coinduction]
pub mod tree {
    use super::wrappers::Boxed;
    #[coinduction]
    use super::Count;

    pub struct Node(pub Boxed<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    // Node -> Boxed<Leaf> -> Leaf -> Node
    impl Count for Node
    where
        Boxed<Leaf>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for Leaf
    where
        Node: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

# fn main() {
use tree::{Leaf, Node};
use wrappers::Boxed;

let leaf = Leaf(Some(Box::new(Node(Boxed(Some(Box::new(Leaf(None))))))));
assert_eq!(leaf.count(), 4);
# }
```

The announced item is used again in the crates that read it, so `announce` asks for paths that
resolve there: `::core::option::Option` rather than `Option` (or
`#[announce(allow_std_prelude = true)]`), and `crate::Local` rather than an imported `Local`. See
the [`announce` documentation](https://docs.rs/announce) for the rules, and its `repeater` option
for types that are private to the defining crate.

The impls of an announced type are added to its announcement as written. Other `#[coinduction]`
modules compare them with their own bounds token by token: a `crate::` path matches the same
`crate::` path, and an imported name such as `Local` matches where it is imported under the same
name. A predicate of these impls can end up in the bounds of the impls of another module, so it
should resolve there too.

### Rules for Other Types

A bound on a type that no `#[coinduction]` module describes, such as a tuple, is resolved with
the rules of the trait. Each rule is a `macro_rules!` pattern for the type and the predicates
that the trait's impl for such a type needs:

```rust
# use announce::announce;
# use coinduction::*;
#[announce(meta = coinduction::rules(
    (($T:ty, $U:ty)) => { $T: Size, $U: Size },
    (u8) => {}
))]
pub trait Size {
    fn size(&self) -> usize;
}

impl Size for u8 {
    fn size(&self) -> usize { 1 }
}

impl<T: Size, U: Size> Size for (T, U) {
    fn size(&self) -> usize { self.0.size() + self.1.size() }
}

#[coinduction]
mod list {
    #[coinduction]
    use super::Size;

    pub struct List(pub Option<Box<(u8, List)>>);

    // List -> (u8, List) -> List
    impl Size for List
    where
        (u8, List): Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size())
        }
    }
}

# fn main() {
let list = list::List(Some(Box::new((1, list::List(Some(Box::new((2, list::List(None)))))))));
assert_eq!(list.size(), 2);
# }
```

`crate::` paths in the rules name the crate that defines the trait, wherever the rules are used.

## Attribute Arguments

- **Module**: `#[coinduction(coinduction = path)]`
  - `coinduction`: override the path used to refer to this crate (default `::coinduction`)
- **Inside a module**: `#[coinduction] use ...;` marks the imported traits; it takes no arguments
- **Trait**: `#[announce(meta = coinduction::rules((pattern) => { predicates }, ...))]`
  - each rule gives the predicates of the impls of the trait for types matching `pattern`, e.g.
    `(($t1: ty, $t2: ty)) => { $t1: TraitA, $t2: TraitB }`
- **Renaming**: `#[announce(macro_name = Name)]` on a trait or type names its trampoline. The
  trait or type name may be taken, for example by a derive macro. `#[coinduction]` modules look
  up trampolines by the trait or type name, so import a renamed one under that name where the
  trait or type is used, e.g. `use path::{Count, CountMacro as Count};`

## Requirements

- Rust 1.71 or later, 2021 edition
- The [`announce`](https://crates.io/crates/announce) crate, to announce traits and types

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

MIT

## Acknowledgments

This library is inspired by coinduction principles in type theory and aims to bring these powerful concepts to practical Rust programming.