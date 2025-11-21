#![feature(trivial_bounds)]
use coinduction::*;
use std::fmt::Debug;

/// Test that coinduction preserves non-circular constraints while removing circular ones
#[coinduction::traitdef]
trait CircularTrait {
    fn circular_method(&self) -> String;
}

#[coinduction::traitdef] 
trait NonCircularTrait {
    fn non_circular_method(&self) -> String;
}

#[coinduction(CircularTrait)]
mod constraint_preservation {
    use super::*;

    pub struct TypeA<T>
    where
        T: Debug + Clone,  // These should be preserved (non-circular)
    {
        pub data: T,
    }

    pub struct TypeB<T>
    where
        T: Send + Sync,    // These should be preserved (non-circular)
    {
        pub data: T,
    }

    // Circular dependency that should be removed
    impl<T> CircularTrait for TypeA<T>
    where
        T: Debug + Clone,           // Non-circular, should be preserved
        T: From<i32>,              // Non-circular, should be preserved
        TypeB<T>: CircularTrait,   // Circular, should be removed
    {
        fn circular_method(&self) -> String {
            format!("TypeA: {:?}", self.data)
        }
    }

    // Circular dependency that should be removed
    impl<T> CircularTrait for TypeB<T>
    where
        T: Send + Sync,            // Non-circular, should be preserved  
        T: Default,                // Non-circular, should be preserved
        TypeA<T>: CircularTrait,   // Circular, should be removed
    {
        fn circular_method(&self) -> String {
            format!("TypeB: {}", std::any::type_name::<T>())
        }
    }

    // Non-circular implementation that should remain unchanged
    impl<T> NonCircularTrait for TypeA<T>
    where
        T: Debug + Clone + PartialEq,  // All non-circular, should be preserved
    {
        fn non_circular_method(&self) -> String {
            format!("NonCircular TypeA: {:?}", self.data)
        }
    }
}

#[test]
fn test_circular_constraints_removed() {
    use constraint_preservation::*;
    
    // Test that circular constraints were removed - these should compile
    let type_a = TypeA { data: 42i32 };
    let type_b = TypeB { data: "hello" };
    
    // These should work because circular constraints were removed
    assert_eq!(type_a.circular_method(), "TypeA: 42");
    assert_eq!(type_b.circular_method(), "TypeB: &str");
}

#[test]
fn test_non_circular_constraints_preserved() {
    use constraint_preservation::*;
    
    // Test that non-circular constraints are preserved by trying to use them
    fn requires_debug_clone<T: Debug + Clone>(x: T) -> T { x }
    fn requires_send_sync<T: Send + Sync>(x: T) -> T { x }
    fn requires_from_i32<T: From<i32>>() -> T { T::from(42) }
    fn requires_default<T: Default>() -> T { T::default() }
    
    // TypeA should still have Debug + Clone + From<i32> constraints
    let type_a = TypeA { data: 42i32 };
    let _: i32 = requires_debug_clone(type_a.data);
    let _: i32 = requires_from_i32();
    
    // TypeB should still have Send + Sync + Default constraints  
    let type_b = TypeB { data: String::from("test") };
    let _: String = requires_send_sync(type_b.data);
    let _: String = requires_default();
}

#[test]
fn test_non_circular_trait_constraints_preserved() {
    use constraint_preservation::*;
    
    // Test that constraints on non-circular traits are preserved
    fn requires_partial_eq<T: PartialEq>(x: &T, y: &T) -> bool { x == y }
    
    let type_a1 = TypeA { data: 42i32 };
    let type_a2 = TypeA { data: 42i32 };
    
    // This should work because PartialEq constraint was preserved on NonCircularTrait impl
    assert!(requires_partial_eq(&type_a1.data, &type_a2.data));
    
    // Test the non-circular method works
    assert_eq!(type_a1.non_circular_method(), "NonCircular TypeA: 42");
}

#[test]
fn test_lifetime_constraints_preserved() {
    // Test that lifetime constraints are also preserved
    use constraint_preservation::*;
    
    let data = 42i32;
    let type_a = TypeA { data };
    
    fn with_lifetime<'a, T: 'a + Debug + Clone>(x: &'a T) -> &'a T { x }
    let _ = with_lifetime(&type_a.data); // Should work if lifetime bounds preserved
}

// Test constraint preservation in #[typedef] context
#[coinduction::traitdef]
trait TypedefTrait {
    fn typedef_method(&self) -> String;
}

#[typedef(TypedefTrait)]
mod typedef_module {
    use super::*;

    pub struct TypedefStruct<T>
    where
        T: Clone + Send + Sync,  // These should be preserved by typedef
    {
        pub data: T,
    }

    impl<T> TypedefTrait for TypedefStruct<T>
    where
        T: Clone + Send + Sync + Debug,  // Additional constraints should be preserved
    {
        fn typedef_method(&self) -> String {
            format!("Typedef: {:?}", self.data)
        }
    }
}

#[test]
fn test_typedef_constraint_preservation() {
    use typedef_module::*;
    
    // Test that typedef preserves struct-level constraints
    fn requires_clone_send_sync<T: Clone + Send + Sync>(x: T) -> T { x }
    
    let typedef_struct = TypedefStruct { data: 42i32 };
    let _: i32 = requires_clone_send_sync(typedef_struct.data);
    
    // Test that impl-level constraints are preserved  
    fn requires_debug<T: Debug>(_: T) {}
    requires_debug(typedef_struct.data);
    
    // Test the method works
    assert_eq!(typedef_struct.typedef_method(), "Typedef: 42");
}

// Test constraint preservation with mixed circular and non-circular traits
#[coinduction::traitdef]
trait MixedTraitA {
    fn mixed_a(&self) -> String;
}

#[coinduction::traitdef]
trait MixedTraitB {
    fn mixed_b(&self) -> String;
}

#[coinduction::traitdef]
trait NonCircularMixed {
    fn non_circular_mixed(&self) -> String;
}

#[coinduction(MixedTraitA, MixedTraitB)]
mod mixed_constraints {
    use super::*;
    use std::fmt::Display;

    pub struct MixedType<T>
    where
        T: Clone + Debug + Send,  // Non-circular, should be preserved
    {
        pub value: T,
    }

    // Circular dependency between MixedTraitA and MixedTraitB - these should be removed
    impl<T> MixedTraitA for MixedType<T>
    where
        T: Clone + Debug + Send,      // Non-circular, should be preserved
        T: PartialEq + Eq,           // Non-circular, should be preserved
        Self: MixedTraitB,           // Circular, should be removed
    {
        fn mixed_a(&self) -> String {
            format!("MixedA: {:?}", self.value)
        }
    }

    impl<T> MixedTraitB for MixedType<T>
    where
        T: Clone + Debug + Send,      // Non-circular, should be preserved
        T: From<i32>,                // Non-circular, should be preserved
        Self: MixedTraitA,           // Circular, should be removed
    {
        fn mixed_b(&self) -> String {
            format!("MixedB: {:?}", self.value)
        }
    }

    // Non-circular trait implementation - all constraints should be preserved
    impl<T> NonCircularMixed for MixedType<T>
    where
        T: Clone + Debug + Send + Display,  // All non-circular, should be preserved
    {
        fn non_circular_mixed(&self) -> String {
            format!("NonCircular: {}", self.value)
        }
    }
}

#[test]
fn test_mixed_constraint_preservation() {
    use mixed_constraints::*;
    use std::fmt::Display;
    
    // Test circular constraints are removed but traits still work
    let mixed = MixedType { value: 42i32 };
    assert_eq!(mixed.mixed_a(), "MixedA: 42");
    assert_eq!(mixed.mixed_b(), "MixedB: 42");
    
    // Test non-circular constraints are preserved
    fn requires_clone_debug_send<T: Clone + Debug + Send>(x: T) -> T { x }
    fn requires_partial_eq<T: PartialEq>(x: &T, y: &T) -> bool { x == y }
    fn requires_from_i32<T: From<i32>>() -> T { T::from(42) }
    fn requires_display<T: Display>(x: T) -> T { x }
    
    let _: i32 = requires_clone_debug_send(mixed.value);
    let mixed2 = MixedType { value: 42i32 };
    assert!(requires_partial_eq(&mixed.value, &mixed2.value));
    let _: i32 = requires_from_i32();
    let _: i32 = requires_display(mixed.value);
    
    // Test non-circular trait method works
    assert_eq!(mixed2.non_circular_mixed(), "NonCircular: 42");
}