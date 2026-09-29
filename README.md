# Coinduction

[![Crates.io](https://img.shields.io/crates/v/coinduction.svg)](https://crates.io/crates/coinduction)
[![Documentation](https://docs.rs/coinduction/badge.svg)](https://docs.rs/coinduction)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Attribute macros for defining circular type references and recursive trait implementations in Rust.

## Overview

Coinduction is a Rust library that provides a single attribute macro, **`#[coinduction]`**, for working with circular type dependencies and mutually recursive trait implementations:

- On a **module**, it breaks circular trait dependencies between the impls in the module, and exports the impls of its types so that other `#[coinduction]` modules can see through bounds on them
- On a **trait**, it makes the trait usable from `#[coinduction]` modules

The library enables you to define types and traits with circular dependencies that would normally be impossible in Rust's type system, making it particularly useful for recursive data structures, graph-like types, and complex generic scenarios.


## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
coinduction = "0.1.0"
```

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
# use coinduction::*;
#[coinduction]
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
`#[coinduction] use ...;` or by defining them in the module with `#[coinduction] trait ...`:

```rust
# use coinduction::*;
#[coinduction]
mod calculator {
    #[coinduction]
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
    }

    impl Evaluate for Term
    where
        Expr: Evaluate,
    {
        // ...
    }
}
```

If no trait is marked, every trait implemented in the module is used.

### Circular Dependencies Across Modules

Types defined in a `#[coinduction]` module can be used in the bounds of another `#[coinduction]` module:

```rust
# use coinduction::*;
# use std::marker::PhantomData;

#[coinduction]
trait ProcessData {
    fn process(&self) -> String;
}

// First define generic types in a #[coinduction] module
#[coinduction]
mod data_types {
    # use std::marker::PhantomData;
    trait ProcessData {
        fn process(&self) -> String;
    }

    pub struct DataStruct<T> {
        pub value: String,
        pub phantom: PhantomData<T>,
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
    trait ProcessData {
        fn process(&self) -> String;
    }
    
    pub struct DataStruct<T> {
        pub value: String,
        pub phantom: PhantomData<T>,
    }

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

    // Base implementation for simple types to break the infinite recursion
    impl ProcessData for String {
        fn process(&self) -> String {
            format!("String: {}", self)
        }
    }
}

// Example usage:
// Test with simple type
let data = data_types::DataStruct::<String> {
    value: "hello".to_string(),
    phantom: PhantomData,
};

// Test cyclic structures using the coinduction module  
let cyclic_a = circular_impls::CyclicA {
    data: circular_impls::DataStruct::<String> {
        value: "cyclic".to_string(),
        phantom: PhantomData,
    },
};

let cyclic_b = circular_impls::CyclicB::<String> {
    value: "test".to_string(),
    phantom: PhantomData,
};
// Note: This demonstrates cyclic induction with DataStruct<T> in T position
```

## Attribute Arguments

- **Module**: `#[coinduction(marker = path, coinduction = path)]`
  - `marker`: a type used to refer to relative paths in the impls exported to other modules. Without it, types whose impls use relative paths are not exported
  - `coinduction`: override the path used to refer to this crate (default `::coinduction`)
- **Trait**: `#[coinduction((pattern) => { predicates }, ..., macro_name = ident)]`
  - each rule gives the predicates of the impls of the trait for types matching `pattern`, e.g. `#[coinduction((($t1: ty, $t2: ty)) => { $t1: TraitA, $t2: TraitB })]`
  - `macro_name`: the name the macro generated for the trait is exported as (default: the trait name)
- **Inside a module**:
  - `#[coinduction] use ...;` takes no arguments
  - `#[coinduction(...)] trait` takes the same arguments as a trait outside a module
  - `#[coinduction(macro_name = ident)]` on a `struct`, `enum` or `union`: the name the macro generated for the type is exported as (default: the type name)

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