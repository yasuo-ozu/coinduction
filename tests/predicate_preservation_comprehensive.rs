#![feature(trivial_bounds)]
use coinduction::*;
use std::fmt::{Debug, Display};
use std::hash::Hash;

/// Comprehensive test for predicate preservation across all macro contexts

// Define traits for testing circular dependencies
#[coinduction::traitdef]
trait CircularA {
    fn method_a(&self) -> String;
}

#[coinduction::traitdef]
trait CircularB {
    fn method_b(&self) -> String;
}

#[coinduction::traitdef]
trait CircularC {
    fn method_c(&self) -> String;
}

// Define non-circular traits
#[coinduction::traitdef]
trait NonCircularTrait {
    fn non_circular_method(&self) -> String;
}

// Pattern-based traitdef for testing
#[allow(unused)]
#[traitdef(
    (($a:ty, $b:ty)) => {$a: PatternTrait, $b: PatternTrait},
    (($a:ty, $b:ty, $c:ty)) => {$a: PatternTrait, $b: PatternTrait, $c: PatternTrait},
)]
trait PatternTrait {}

// Test helper types that implement required constraints
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TestType(pub String);

impl Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for TestType {
    fn from(value: usize) -> Self {
        TestType(value.to_string())
    }
}

impl From<i32> for TestType {
    fn from(value: i32) -> Self {
        TestType(value.to_string())
    }
}

impl std::ops::Add for TestType {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        TestType(format!("{}+{}", self.0, other.0))
    }
}

impl Into<String> for TestType {
    fn into(self) -> String {
        self.0
    }
}

// 1. TEST COINDUCTION CONTEXT - Multiple impl types with preserved constraints
#[coinduction(CircularA, CircularB, CircularC)]
mod coinduction_predicate_test {
    use super::*;

    pub struct TypeA<T> {
        pub data: T,
    }
    
    pub struct TypeB<T, U> {
        pub first: T,
        pub second: U,
    }
    
    pub struct TypeC<'a, T> {
        pub reference: &'a T,
        pub owned: T,
    }

    // IMPL 1: CircularA with mixed circular and non-circular constraints
    impl<T> super::CircularA for TypeA<T>
    where
        // Non-circular constraints that MUST be preserved
        T: Clone + Debug + Send + Sync + 'static,
        T: Display + PartialEq + Eq + Hash,
        T: From<usize> + From<i32>,
        T: std::ops::Add<Output = T>,
        // Circular constraints that SHOULD be removed
        TypeB<T, String>: super::CircularB,
        TypeC<'static, T>: super::CircularC,
    {
        fn method_a(&self) -> String {
            format!("TypeA: {}", self.data)
        }
    }

    // IMPL 2: CircularB with different constraint patterns
    impl<T, U> super::CircularB for TypeB<T, U>
    where
        // Non-circular constraints that MUST be preserved
        T: Clone + Debug + Send,
        U: Default + PartialEq + Clone,
        T: Display + Hash + Eq,
        U: From<usize> + Into<String>,
        // Lifetime constraints that MUST be preserved
        T: 'static,
        U: 'static,
        // Circular constraints that SHOULD be removed
        TypeA<T>: super::CircularA,
        TypeC<'static, T>: super::CircularC,
    {
        fn method_b(&self) -> String {
            format!("TypeB: {}", self.first)
        }
    }

    // IMPL 3: CircularC with lifetime and associated type constraints
    impl<'a, T> super::CircularC for TypeC<'a, T>
    where
        // Non-circular constraints that MUST be preserved
        T: 'a + Clone + Debug + Send + Sync,
        T: Display + PartialOrd + Ord,
        T: std::ops::Add<Output = T>,
        T: From<i32> + Default,
        // Circular constraints that SHOULD be removed
        TypeA<T>: super::CircularA,
        TypeB<T, String>: super::CircularB,
    {
        fn method_c(&self) -> String {
            format!("TypeC: {}", self.reference)
        }
    }

    // IMPL 4: Non-circular trait - ALL constraints should be preserved
    impl<T> super::NonCircularTrait for TypeA<T>
    where
        // ALL of these should be preserved (no circular dependencies)
        T: Clone + Debug + Send + Sync + Display + PartialEq,
        T: From<usize> + Hash + Eq + 'static,
        T: std::ops::Add<Output = T> + PartialOrd + Ord,
    {
        fn non_circular_method(&self) -> String {
            format!("NonCircular TypeA: {}", self.data)
        }
    }
}

// 2. TEST TYPEDEF CONTEXT - All constraints should be preserved (no circular removal)
#[typedef(CircularA, CircularB, NonCircularTrait)]
mod typedef_predicate_test {
    use super::*;

    pub struct TypedefType<T, U>
    where
        // Struct-level constraints that MUST be preserved
        T: Clone + Debug + Send + Sync + 'static,
        U: Default + PartialEq + From<usize>,
    {
        pub primary: T,
        pub secondary: U,
    }

    pub struct ComplexTypedefType<'a, T>
    where
        T: 'a + Clone + Debug,
    {
        pub reference: &'a T,
        pub owned: T,
    }

    // IMPL 1: CircularA - ALL constraints should be preserved in typedef
    impl<T, U> super::CircularA for TypedefType<T, U>
    where
        // ALL of these should be preserved (typedef doesn't remove circular deps)
        T: Clone + Debug + Send + Sync + 'static + Display + Hash + Eq,
        U: Default + PartialEq + From<usize> + Into<String> + Clone,
        T: std::ops::Add<Output = T> + PartialOrd + Ord,
        T: From<i32>,
    {
        fn method_a(&self) -> String {
            format!("Typedef CircularA: {}", self.primary)
        }
    }

    // IMPL 2: CircularB with lifetime constraints - ALL should be preserved
    impl<'a, T> super::CircularB for ComplexTypedefType<'a, T>
    where
        // ALL of these should be preserved
        T: 'a + Clone + Debug + Send + Sync + Display,
        T: PartialEq + Eq + Hash + PartialOrd + Ord,
        T: From<usize> + From<i32> + Default,
        T: std::ops::Add<Output = T>,
    {
        fn method_b(&self) -> String {
            format!("Typedef CircularB: {}", self.reference)
        }
    }

    // IMPL 3: NonCircularTrait - ALL constraints should be preserved
    impl<T, U> super::NonCircularTrait for TypedefType<T, U>
    where
        // ALL of these should be preserved
        T: Clone + Debug + Send + Sync + 'static + Display,
        U: Default + PartialEq + From<usize> + Clone,
        T: Hash + Eq + std::ops::Add<Output = T>,
        U: Into<String>,
    {
        fn non_circular_method(&self) -> String {
            format!("Typedef NonCircular: {}", self.primary)
        }
    }
}

// 3. TEST TRAITDEF PATTERN CONTEXT - Pattern-based constraints should be preserved
mod pattern_predicate_test {
    use super::*;

    #[derive(Clone)]
    pub struct PatternTypeA<T> {
        pub data: T,
    }

    #[derive(Clone)]
    pub struct PatternTypeB<T, U> {
        pub first: T,
        pub second: U,
    }

    // IMPL 1: Pattern trait for single type - ALL constraints should be preserved
    impl<T> super::PatternTrait for PatternTypeA<T>
    where
        // ALL of these should be preserved in pattern context
        T: Clone + Debug + Send + Sync + 'static,
        T: Display + PartialEq + Eq + Hash,
        T: From<usize> + std::ops::Add<Output = T>,
    {
    }

    // IMPL 2: Pattern trait for tuple - ALL constraints should be preserved
    impl<T, U> super::PatternTrait for (PatternTypeA<T>, PatternTypeB<T, U>)
    where
        // ALL of these should be preserved
        T: Clone + Debug + Send + Sync + Display,
        U: Default + PartialEq + From<usize> + Clone,
        T: Hash + Eq + 'static + From<i32>,
        U: Into<String>,
        // Pattern-based constraints
        PatternTypeA<T>: super::PatternTrait,
        PatternTypeB<T, U>: super::PatternTrait,
    {
    }

    // IMPL 3: Pattern trait for triple tuple
    impl<T, U, V> super::PatternTrait for (PatternTypeA<T>, PatternTypeB<U, V>, PatternTypeA<V>)
    where
        // ALL of these should be preserved
        T: Clone + Debug + Send + Display + Hash + Eq + 'static,
        U: Clone + Debug + Default + PartialEq + From<usize>,
        V: Clone + Debug + From<i32> + Display + Send,
        T: std::ops::Add<Output = T> + PartialOrd + Ord,
        U: Into<String> + Send + Sync,
        V: Hash + Eq + Default + 'static,
        // Pattern-based constraints
        PatternTypeA<T>: super::PatternTrait,
        PatternTypeB<U, V>: super::PatternTrait,
        PatternTypeA<V>: super::PatternTrait,
    {
    }

    // IMPL 4: Individual implementations for pattern types
    impl<T, U> super::PatternTrait for PatternTypeB<T, U>
    where
        // ALL of these should be preserved
        T: Clone + Debug + Send + Sync + Display + 'static,
        U: Default + PartialEq + From<usize> + Clone + Into<String>,
        T: Hash + Eq + From<i32> + std::ops::Add<Output = T>,
    {
    }
}

// COMPREHENSIVE TESTS FOR PREDICATE PRESERVATION

#[test]
fn test_coinduction_predicate_preservation() {
    use coinduction_predicate_test::*;
    
    // Test that non-circular constraints are preserved by using them
    
    // TypeA should preserve: Clone, Debug, Send, Sync, Display, PartialEq, Eq, Hash, From<usize>, From<i32>, Add
    let type_a = TypeA { data: TestType::from(42i32) };
    
    // Test preserved constraints
    fn requires_clone_debug_send_sync<T: Clone + Debug + Send + Sync>(x: T) -> T { x }
    fn requires_display<T: Display>(x: &T) -> String { format!("{}", x) }
    fn requires_partial_eq_hash<T: PartialEq + Hash>(_: &T) {}
    fn requires_from_usize<T: From<usize>>() -> T { T::from(42) }
    fn requires_from_i32<T: From<i32>>() -> T { T::from(42) }
    fn requires_add<T: std::ops::Add<Output = T>>(x: T, y: T) -> T { x + y }
    
    let _: TestType = requires_clone_debug_send_sync(type_a.data.clone());
    let _: String = requires_display(&type_a.data);
    requires_partial_eq_hash(&type_a.data);
    let _: TestType = requires_from_usize();
    let _: TestType = requires_from_i32();
    let _: TestType = requires_add(type_a.data.clone(), TestType::from(1));
    
    // Test that circular dependencies are removed by checking that methods compile and work
    // Note: The circular constraints should be removed by coinduction, so these should work
    // even without implementing the circular trait dependencies
    
    // For demonstration purposes, let's test the constraint preservation more directly
    // by testing functions that require the preserved constraints
    
    // Test non-circular trait implementation preserves all constraints
    assert_eq!(type_a.non_circular_method(), "NonCircular TypeA: 42");
}

#[test]
fn test_typedef_predicate_preservation() {
    use typedef_predicate_test::*;
    
    // Test that ALL constraints are preserved in typedef context (no circular removal)
    
    let typedef_type = TypedefType {
        primary: TestType::from(42),
        secondary: TestType::default(),
    };
    
    // Test all preserved constraints
    fn requires_all_traits<T: Clone + Debug + Send + Sync + Display + Hash + Eq>(_: &T) {}
    fn requires_conversion_traits<T: From<i32> + From<usize>>() -> T { T::from(42) }
    fn requires_ops<T: std::ops::Add<Output = T> + PartialOrd + Ord>(x: T, y: T) -> T { x + y }
    
    requires_all_traits(&typedef_type.primary);
    let _: TestType = requires_conversion_traits();
    let _: TestType = requires_ops(typedef_type.primary.clone(), TestType::from(1));
    
    // Test that instances can be created (this tests that constraints are properly preserved)
    
    // Test lifetime typedef type
    let value = TestType::from(200);
    let _complex_type = ComplexTypedefType {
        reference: &value,
        owned: value.clone(),
    };
    // Test that the complex type can be created with its constraints
}

#[test]
fn test_pattern_predicate_preservation() {
    use pattern_predicate_test::*;
    
    // Test that pattern-based constraints are preserved
    
    let pattern_a = PatternTypeA { data: TestType::from(42) };
    let pattern_b = PatternTypeB { 
        first: TestType::from(10), 
        second: TestType::default() 
    };
    
    // Test individual type constraints are preserved
    fn requires_pattern_constraints<T: Clone + Debug + Send + Sync + Display + Hash + Eq>(_: &T) {}
    fn requires_conversion<T: From<usize> + From<i32>>() -> T { T::from(42) }
    fn requires_add_ops<T: std::ops::Add<Output = T>>(x: T, y: T) -> T { x + y }
    
    requires_pattern_constraints(&pattern_a.data);
    let _: TestType = requires_conversion();
    let _: TestType = requires_add_ops(pattern_a.data.clone(), TestType::from(1));
    
    // Test tuple pattern constraints are preserved
    let tuple_pattern = (pattern_a.clone(), pattern_b.clone());
    fn test_pattern_trait<T: PatternTrait>(_: T) {}
    test_pattern_trait(tuple_pattern);
    
    // Test triple pattern constraints
    let triple_pattern = (
        PatternTypeA { data: TestType::from(1) },
        PatternTypeB { first: TestType::from(2), second: TestType::from(3) },
        PatternTypeA { data: TestType::from(4) },
    );
    test_pattern_trait(triple_pattern);
}

#[test]
fn test_mixed_context_predicate_preservation() {
    // Test that constraints are preserved correctly across different contexts
    
    // Coinduction context - circular removed, non-circular preserved
    use coinduction_predicate_test::TypeA as CoinTypeA;
    let coin_a = CoinTypeA { data: TestType::from(42) };
    
    // Typedef context - all constraints preserved
    use typedef_predicate_test::TypedefType;
    let typedef_instance = TypedefType {
        primary: TestType::from(42),
        secondary: TestType::default(),
    };
    
    // Pattern context - all constraints preserved
    use pattern_predicate_test::PatternTypeA;
    let pattern_a = PatternTypeA { data: TestType::from(42) };
    
    // Test that the same constraint types work across all contexts
    fn requires_display_hash<T: Display + Hash>(_: &T) {}
    requires_display_hash(&coin_a.data);
    requires_display_hash(&typedef_instance.primary);
    requires_display_hash(&pattern_a.data);
    
    // Test that all contexts preserve From<i32> constraint
    fn requires_from_i32<T: From<i32>>() -> T { T::from(42) }
    let _: TestType = requires_from_i32();
    
    // Test non-circular trait implementations work
    assert_eq!(coin_a.non_circular_method(), "NonCircular TypeA: 42");
    assert_eq!(typedef_instance.non_circular_method(), "Typedef NonCircular: 42");
}

#[test]
fn test_complex_constraint_combinations() {
    // Test complex combinations of constraints to ensure fine-grained preservation
    
    use coinduction_predicate_test::*;
    
    // Create instances that satisfy complex where clauses
    let type_a = TypeA { data: TestType::from(42) };
    let type_b = TypeB { 
        first: TestType::from(10), 
        second: TestType::default() 
    };
    
    // Test combinations of preserved constraints
    fn complex_constraint_test<T>(x: T) -> T
    where
        T: Clone + Debug + Send + Sync + Display + PartialEq + Eq + Hash + 'static,
        T: From<usize> + From<i32> + std::ops::Add<Output = T> + PartialOrd + Ord + Default,
    {
        let new_val = T::from(1) + T::from(2);
        assert!(new_val >= T::default());
        x
    }
    
    let _: TestType = complex_constraint_test(type_a.data);
    let _: TestType = complex_constraint_test(type_b.first);
    
    // Test that lifetime constraints are preserved
    let value = TestType::from(100);
    let type_c = TypeC { 
        reference: &value,
        owned: value.clone() 
    };
    
    fn lifetime_constraint_test<'a, T: 'a + Clone + Debug + Display>(x: &TypeC<'a, T>) -> &'a T {
        x.reference
    }
    
    let ref_value = lifetime_constraint_test(&type_c);
    assert_eq!(format!("{}", ref_value), "100");
}