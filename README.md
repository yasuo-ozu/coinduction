# Coinduction

[![Crates.io](https://img.shields.io/crates/v/coinduction.svg)](https://crates.io/crates/coinduction)
[![Documentation](https://docs.rs/coinduction/badge.svg)](https://docs.rs/coinduction)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Attribute macros for defining circular type references and recursive trait implementations in Rust.

## Overview

Coinduction is a Rust library that provides three powerful attribute macros for working with circular type dependencies and mutually recursive trait implementations:

- **`#[coinduction]`** - Breaks circular trait dependencies in modules
- **`#[traitdef]`** - Creates traits with marker types for type-level programming  
- **`#[typedef]`** - Organizes type definitions with trait implementations

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
#[traitdef]
trait Evaluate {
    fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32;
}

#[coinduction]
mod calculator {
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

### Generic Types with Circular Dependencies

This example demonstrates circular dependencies with generic types:

```rust
use coinduction::*;
use std::marker::PhantomData;

#[coinduction]
mod generic_circular {
    use std::marker::PhantomData;
    
    pub trait Process<T> {
        fn process(&self, input: T) -> T;
    }

    pub struct NodeA<T> {
        pub phantom: PhantomData<T>,
    }

    pub struct NodeB<T> {
        pub phantom: PhantomData<T>,
    }

    // NodeA's implementation depends on NodeB implementing Process<T>
    impl<T: Clone> Process<T> for NodeA<T>
    where
        NodeB<T>: Process<T>,
    {
        fn process(&self, input: T) -> T {
            input.clone()
        }
    }

    // NodeB's implementation depends on NodeA implementing Process<T>
    impl<T: Clone> Process<T> for NodeB<T>
    where
        NodeA<T>: Process<T>,
    {
        fn process(&self, input: T) -> T {
            input.clone()
        }
    }
}

// Example usage:
use generic_circular::{NodeA, Process};
let node_a = NodeA::<String> {
    phantom: PhantomData,
};
let result = node_a.process("test".to_string());
assert_eq!(result, "test");
```

### Organizing Types with `typedef`

This example shows how to organize related types using `#[typedef]` combined with `#[coinduction]`:

```rust
use coinduction::*;
use std::marker::PhantomData;

#[traitdef]
trait ProcessData {
    fn process(&self) -> String;
}

// First define generic types with #[typedef] to organize them
#[typedef]
mod data_types {
    use std::marker::PhantomData;
    
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
    use std::marker::PhantomData;
    
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

## Requirements

- Rust 2021 edition or later
- No additional runtime dependencies

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

MIT

## Acknowledgments

This library is inspired by coinduction principles in type theory and aims to bring these powerful concepts to practical Rust programming.