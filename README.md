# Coinduction

[![Crates.io](https://img.shields.io/crates/v/coinduction.svg)](https://crates.io/crates/coinduction)
[![Documentation](https://docs.rs/coinduction/badge.svg)](https://docs.rs/coinduction)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Attribute macros for defining circular type references and recursive trait implementations in Rust.

## Overview

Coinduction is a Rust library that provides a single attribute macro, **`#[coinduction]`**, for working with circular type dependencies and mutually recursive trait implementations:

- On a **module**, it breaks circular trait dependencies between the impls in the module, and exports the impls of its types announced with [`announce`](https://crates.io/crates/announce) so that other `#[coinduction]` modules can see through bounds on them
- A trait is made usable from `#[coinduction]` modules by announcing it with `#[announce]`

The library enables you to define types and traits with circular dependencies that would normally be impossible in Rust's type system, making it particularly useful for recursive data structures, graph-like types, and complex generic scenarios.


## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
coinduction = "0.1.0"
announce = "0.2.0"
```

`coinduction` also re-exports announce as `coinduction::announce`.

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

    impl super::Evaluate for Expr
    where
        Term: super::Evaluate,
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

    impl super::Evaluate for Term
    where
        Expr: super::Evaluate,
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

A type announced in a `#[coinduction]` module can be used in the bounds of another `#[coinduction]` module:

```rust
# use announce::announce;
# use coinduction::*;
# use std::marker::PhantomData;

#[announce]
trait ProcessData {
    fn process(&self) -> ::std::string::String;
}

// Base implementation for simple types to break the infinite recursion
// (outside the modules: they implement traits only for their own types)
impl ProcessData for String {
    fn process(&self) -> String {
        format!("String: {}", self)
    }
}

// First define generic types in a #[coinduction] module
#[coinduction]
mod data_types {
    #[coinduction]
    use super::ProcessData;
    use announce::announce;

    // Announced, with paths that resolve in other crates
    #[announce]
    pub struct DataStruct<T> {
        pub value: ::std::string::String,
        pub phantom: ::core::marker::PhantomData<T>,
    }

    // ProcessData impl for DataStruct when T: ProcessData
    impl<T> ProcessData for DataStruct<T>
    where
        T: ProcessData,
    {
        fn process(&self) -> String {
            format!("DataStruct({})", self.value)
        }
    }
}

// Then use #[coinduction] to implement cyclic induction with T position circularity
#[coinduction]
mod circular_impls {
    # use std::marker::PhantomData;
    #[coinduction]
    use super::ProcessData;
    use super::data_types::DataStruct;

    // Cyclic induction with constraints using DataStruct<T>
    // where the circular dependency occurs in the T position
    pub struct CyclicA<T> {
        pub data: DataStruct<T>,
    }
    
    pub struct CyclicB<T> {
        pub value: String,
        pub phantom: PhantomData<T>,
    }

    // CyclicA implementation with circular dependency in T position
    impl<T> ProcessData for CyclicA<T>
    where
        T: Clone,
        DataStruct<CyclicB<T>>: ProcessData,  // Circular dependency in T position
    {
        fn process(&self) -> String {
            format!("CyclicA with DataStruct: {}", self.data.value)
        }
    }

    // CyclicB implementation with circular dependency in T position  
    impl<T> ProcessData for CyclicB<T>
    where
        T: Clone,
        DataStruct<CyclicA<T>>: ProcessData,  // Circular dependency in T position
    {
        fn process(&self) -> String {
            format!("CyclicB: {}", self.value)
        }
    }
}

# fn main() {
// Example usage:
// Test with simple type
let data = data_types::DataStruct::<String> {
    value: "hello".to_string(),
    phantom: PhantomData,
};
assert_eq!(data.process(), "DataStruct(hello)");

// Test cyclic structures using the coinduction module  
let cyclic_a = circular_impls::CyclicA {
    data: data_types::DataStruct::<String> {
        value: "cyclic".to_string(),
        phantom: PhantomData,
    },
};
assert_eq!(cyclic_a.process(), "CyclicA with DataStruct: cyclic");

let cyclic_b = circular_impls::CyclicB::<String> {
    value: "test".to_string(),
    phantom: PhantomData,
};
assert_eq!(cyclic_b.process(), "CyclicB: test");
// Note: This demonstrates cyclic induction with DataStruct<T> in T position
# }
```

## Attribute Arguments

- **Module**: `#[coinduction(coinduction = path)]`
  - `coinduction`: override the path used to refer to this crate (default `::coinduction`)
- **Trait**: `#[announce(meta = coinduction::rules((pattern) => { predicates }, ...))]`
  - each rule gives the predicates of the impls of the trait for types matching `pattern`, e.g. `#[announce(meta = coinduction::rules((($t1: ty, $t2: ty)) => { $t1: TraitA, $t2: TraitB }))]`
- **Inside a module**:
  - `#[coinduction] use ...;` takes no arguments
  - `#[announce(macro_name = ident)]` on a trait or type: the name the macro generated for it is exported as (default: the trait or type name)

  `#[coinduction]` modules look up these macros by the trait or type name, so when a macro is
  renamed, import it under that name where the trait or type is used, e.g.
  `use path::{Count, CountMacro as Count};`

## Requirements

- Rust 2021 edition or later
- No additional runtime dependencies

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

MIT

## Acknowledgments

This library is inspired by coinduction principles in type theory and aims to bring these powerful concepts to practical Rust programming.