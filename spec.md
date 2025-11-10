# Coinduction Crate Specification

## Overview

The `coinduction` crate provides attribute macros that enable circular type references and recursive trait implementations in Rust. This crate solves the problem of defining types that need to implement traits based on circular dependencies, where the implementation of a trait on one type depends on whether another type implements the same trait.

## Core Concepts

### Coinduction
Coinduction is a mathematical principle that allows for the definition of infinite or recursive structures. In the context of this crate, it enables defining trait implementations that can reference themselves circularly.

### Circular Type References
Types that reference each other in their trait implementations, creating a dependency cycle that would normally be impossible to resolve statically.

## Attribute Macros

### `#[traitdef]`

**Purpose**: Marks a trait or module as participating in coinductive reasoning.

**Usage**:
- Applied to trait definitions directly
- Applied to parent modules containing trait definitions
- Supports special arguments for unnamed types

**Syntax**:
```rust
#[traitdef]
trait MyTrait { ... }

// Or on module
#[traitdef]
mod my_module {
    trait MyTrait { ... }
}
```

**Special Arguments for Unnamed Types**:
```rust
#[traitdef(
    ref_types = "&T",           // Reference types
    tuple_types = "()",         // Unit type
    tuple_types = "(T,)",       // Single-element tuples
    array_types = "[T; N]",     // Arrays with const generic size
    // ... other unnamed type patterns
)]
trait MyTrait { ... }
```

### `#[typedef(path::to::Trait)]`

**Purpose**: Marks a module as containing type definitions that implement a specific trait.

**Usage**:
- Applied to modules containing type definitions and their implementations
- The path parameter specifies which trait the types in this module implement
- Types and their impl blocks must be in the same module

**Syntax**:
```rust
#[typedef(path::to::MyTrait)]
mod my_types {
    struct MyStruct<T> { ... }
    
    impl<T> MyTrait for MyStruct<T> { ... }
}
```

**Requirements**:
- Type definition and impl block must be in the same module
- Types may have generic parameters
- For std types, delegation must be used with type aliases

**Std Type Delegation**:
```rust
#[typedef(path::to::MyTrait)]
mod std_impls {
    type MyString = String;
    
    impl MyTrait for MyString { ... }
}
```

### `#[recursive(Trait)]`

**Purpose**: Marks a module as containing recursive type definitions with circular trait dependencies.

**Usage**:
- Applied to modules containing types with circular trait implementation dependencies
- The trait parameter specifies which trait has circular dependencies
- Handles where predicates that create circular dependencies

**Syntax**:
```rust
#[recursive(MyTrait)]
mod recursive_types {
    struct RecursiveType<T> 
    where
        T: MyTrait,           // This creates a circular dependency
        SomeOtherType<T>: MyTrait,  // if SomeOtherType's impl depends on T: MyTrait
    {
        data: T,
    }
    
    impl<T> MyTrait for RecursiveType<T> 
    where
        T: MyTrait,
        SomeOtherType<T>: MyTrait,
    { ... }
}
```

## Workflow

### 1. Define the Trait
```rust
#[traitdef]
trait MyTrait {
    fn my_method(&self) -> bool;
}
```

### 2. Define Non-Circular Types
```rust
#[typedef(MyTrait)]
mod basic_types {
    struct SimpleType;
    
    impl MyTrait for SimpleType {
        fn my_method(&self) -> bool { true }
    }
}
```

### 3. Define Circular/Recursive Types
```rust
#[recursive(MyTrait)]
mod recursive_types {
    struct Node<T> {
        value: T,
        children: Vec<Node<T>>,
    }
    
    impl<T> MyTrait for Node<T> 
    where
        T: MyTrait,
        Vec<Node<T>>: MyTrait,  // Circular dependency
    {
        fn my_method(&self) -> bool {
            self.value.my_method() && 
            self.children.iter().all(|child| child.my_method())
        }
    }
}
```

## Module Organization Requirements

1. **Trait Definition**: Must be marked with `#[traitdef]` or be in a module marked with `#[traitdef]`

2. **Type Definitions**: Must be in modules marked with `#[typedef(path::to::Trait)]`
   - Type definition and impl block in same module
   - One module per trait implementation group

3. **Recursive Definitions**: Must be in modules marked with `#[recursive(Trait)]`
   - Contains types with circular trait dependencies
   - Handles complex where clauses with circular predicates

## Unnamed Type Support

The `#[traitdef]` macro supports special syntax for implementing traits on unnamed types:

```rust
#[traitdef(
    ref_types = "&T",
    tuple_types = "()", "(T,)", "(T, U)",
    array_types = "[T; N]",
    slice_types = "[T]",
    fn_types = "fn() -> T",
    // ... other patterns
)]
trait MyTrait { ... }
```

## Error Handling

The macros will generate compile-time errors for:
- Incorrect module organization
- Missing trait definitions
- Circular dependencies that cannot be resolved
- Invalid where clause predicates
- Mismatched type and impl locations

## Limitations

1. All participating types and traits must use the coinduction macros
2. Complex generic bounds may require careful ordering
3. Some edge cases with associated types may not be supported
4. Performance implications for deeply recursive type hierarchies

## Examples

See the examples directory for complete working examples of:
- Basic coinductive trait implementations
- Recursive data structures
- Standard library type implementations
- Complex generic scenarios