#![feature(trivial_bounds)]
use coinduction::*;
use std::fmt::Debug;

/// Test that traitdef pattern matching preserves constraints properly
#[allow(unused)]
#[traitdef(
    (($a:ty, $b: ty)) => {$a: ProcessWithConstraints, $b: ProcessWithConstraints},
    (($a:ty, $b: ty, $c: ty)) => {$a: ProcessWithConstraints, $b: ProcessWithConstraints, $c: ProcessWithConstraints},
)]
trait ProcessWithConstraints {}

mod constraint_pattern_test {
    use super::*;

    // Test struct with constraints that should be preserved
    pub struct ConstrainedTuple<A, B>
    where
        A: Clone + Debug + Send,  // These should be preserved
        B: Default + PartialEq,   // These should be preserved
    {
        pub first: A,
        pub second: B,
    }

    // Implementation with pattern-based bounds and additional constraints
    impl<A, B> super::ProcessWithConstraints for ConstrainedTuple<A, B>
    where
        A: Clone + Debug + Send + Sync,    // Additional constraints beyond struct bounds
        B: Default + PartialEq + Eq,       // Additional constraints beyond struct bounds
        (A, B): super::ProcessWithConstraints,  // Pattern-based constraint
    {
    }

    // Implementation for tuples with preserved constraints
    impl<A, B> super::ProcessWithConstraints for (A, B)
    where
        A: Clone + Debug + Send,    // These should be preserved
        B: Default + PartialEq,     // These should be preserved
    {
    }

    // Implementation for three-tuples
    impl<A, B, C> super::ProcessWithConstraints for (A, B, C)
    where
        A: Clone + Debug + Send,    // These should be preserved
        B: Default + PartialEq,     // These should be preserved  
        C: From<usize> + Into<String> + Clone,  // These should be preserved
    {
    }
}

#[test]
fn test_traitdef_pattern_constraint_preservation() {
    use constraint_pattern_test::*;
    
    // Test that struct-level constraints are preserved
    fn requires_clone_debug_send<T: Clone + Debug + Send>(x: T) -> T { x }
    fn requires_default_partial_eq<T: Default + PartialEq>() -> T { T::default() }
    
    let constrained_tuple = ConstrainedTuple {
        first: 42i32,
        second: String::from("test"),
    };
    
    // Test struct field constraints are preserved
    let _: i32 = requires_clone_debug_send(constrained_tuple.first);
    let _: String = requires_default_partial_eq();
    
    // Test that the struct can be used in trait context
    fn test_trait_impl<T: ProcessWithConstraints>(_: T) {}
    test_trait_impl(constrained_tuple);
}

#[test]
fn test_tuple_constraint_preservation() {
    // Test that tuple implementations preserve their constraints
    fn requires_clone_debug_send<T: Clone + Debug + Send>(x: T) -> T { x }
    fn requires_default_partial_eq<T: Default + PartialEq>() -> T { T::default() }
    
    let tuple = (42i32, String::from("test"));
    
    // Test individual element constraints
    let _: i32 = requires_clone_debug_send(tuple.0);
    let _: String = requires_default_partial_eq();
    
    // Test that tuples work with the trait
    fn test_trait_impl<T: ProcessWithConstraints>(_: T) {}
    test_trait_impl(tuple);
}

// Test type that implements the required traits
#[derive(Clone, Debug, Default, PartialEq)]
struct TestValueType(usize);

impl From<usize> for TestValueType {
    fn from(value: usize) -> Self {
        TestValueType(value)
    }
}

impl Into<String> for TestValueType {
    fn into(self) -> String {
        self.0.to_string()
    }
}

#[test] 
fn test_triple_tuple_constraint_preservation() {
    // Test three-tuple specific constraints
    fn requires_from_usize<T: From<usize>>() -> T { T::from(42) }
    fn requires_into_string<T: Into<String>>(x: T) -> String { x.into() }
    
    let triple = (42i32, String::from("test"), TestValueType(42));
    
    // Test the From<usize> constraint
    let _: TestValueType = requires_from_usize();
    
    // Test the Into<String> constraint
    let result: String = requires_into_string(triple.2.clone());
    assert_eq!(result, "42");
    
    // Test that triple tuples work with the trait
    fn test_trait_impl<T: ProcessWithConstraints>(_: T) {}
    test_trait_impl(triple);
}

// Test constraint preservation with lifetime parameters
#[traitdef]
trait LifetimeTrait<'a> {
    fn lifetime_method(&self) -> &'a str;
}

mod lifetime_constraints {
    use super::*;
    use std::fmt::Display;

    pub struct LifetimeStruct<'a, T>
    where
        T: 'a + Clone + Debug,  // Lifetime and trait constraints should be preserved
    {
        pub reference: &'a T,
        pub owned: T,
    }

    impl<'a, T> super::LifetimeTrait<'a> for LifetimeStruct<'a, T>
    where
        T: 'a + Clone + Debug + Display,  // All constraints should be preserved
    {
        fn lifetime_method(&self) -> &'a str {
            "lifetime test"
        }
    }
}

#[test]
fn test_lifetime_constraint_preservation() {
    use lifetime_constraints::*;
    use std::fmt::Display;
    
    let value = 42i32;
    let lifetime_struct = LifetimeStruct {
        reference: &value,
        owned: value,
    };
    
    // Test lifetime constraints
    fn with_lifetime<'a, T: 'a + Clone + Debug>(x: &LifetimeStruct<'a, T>) -> &'a T {
        x.reference
    }
    
    let ref_value = with_lifetime(&lifetime_struct);
    assert_eq!(*ref_value, 42);
    
    // Test trait constraint preservation
    fn requires_display<T: Display>(x: T) -> T { x }
    let _: i32 = requires_display(lifetime_struct.owned);
    
    // Test the trait method works
    assert_eq!(lifetime_struct.lifetime_method(), "lifetime test");
}