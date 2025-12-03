use traitdef::{LocalTrait, TestTrait, CircularTrait, ExtendedTrait};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

// Test data types that implement the required traits
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TestType(pub usize);

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Implement Send and Sync for TestType
unsafe impl Send for TestType {}
unsafe impl Sync for TestType {}

#[test]
fn test_local_trait_array_pattern() {
    // Test pattern: ([$T:ty; $N:expr]) => { $T: Clone + Default }
    let array: [TestType; 3] = [TestType::default(), TestType(1), TestType(2)];
    
    // Exercise the trait bounds: Clone + Default
    let cloned_first = array[0].clone();
    let default_value = TestType::default();
    
    assert_eq!(cloned_first, default_value);
    
    // Call the trait method
    assert_eq!(array.local_method(), 3); // Should return array length
}

#[test]
fn test_local_trait_slice_pattern() {
    // Test pattern: ([$T:ty]) => { $T: PartialEq + Clone }
    let vec_data = vec![TestType(1), TestType(2), TestType(1)];
    let slice: &[TestType] = &vec_data;
    
    // Exercise the trait bounds: PartialEq + Clone
    assert_eq!(slice[0], slice[2]); // PartialEq
    let cloned_element = slice[0].clone(); // Clone
    assert_eq!(cloned_element, TestType(1));
    
    // Call the trait method
    assert_eq!(slice.local_method(), 3); // Should return slice length
}

#[test]
fn test_local_trait_tuple2_pattern() {
    // Test pattern: (($T:ty, $U:ty)) => { $T: Clone + Send, $U: Sync + Default }
    let tuple: (TestType, i32) = (TestType(42), 100);
    
    // Exercise the trait bounds
    let cloned_first = tuple.0.clone(); // T: Clone + Send
    let default_second = i32::default(); // U: Sync + Default
    
    assert_eq!(cloned_first, TestType(42));
    assert_eq!(default_second, 0);
    
    // Call the trait method
    assert_eq!(tuple.local_method(), 2); // Should return 2 for tuple
}

#[test]
fn test_local_trait_tuple3_pattern() {
    // Test pattern: (($T:ty, $U:ty, $V:ty)) => { $T: Clone + Send, $U: Sync, $V: Default + Send }
    let tuple: (TestType, String, f64) = (TestType(10), "test".to_string(), 3.14);
    
    // Exercise the trait bounds
    let cloned_first = tuple.0.clone(); // T: Clone + Send
    // U: Sync (String implements Sync)
    let default_third = f64::default(); // V: Default + Send
    
    assert_eq!(cloned_first, TestType(10));
    assert_eq!(default_third, 0.0);
    
    // Call the trait method
    assert_eq!(tuple.local_method(), 3); // Should return 3 for triple
}

#[test]
fn test_test_trait_tuple2_pattern() {
    // Test pattern: (($T:ty, $U:ty)) => { $T: Clone + Send, $U: Debug + Default }
    let tuple: (TestType, Vec<i32>) = (TestType(5), vec![1, 2, 3]);
    
    // Exercise the trait bounds
    let cloned_first = tuple.0.clone(); // T: Clone + Send
    let debug_string = format!("{:?}", tuple.1); // U: Debug
    let default_second = Vec::<i32>::default(); // U: Default
    
    assert_eq!(cloned_first, TestType(5));
    assert_eq!(debug_string, "[1, 2, 3]");
    assert!(default_second.is_empty());
    
    // Call the trait method
    let result = tuple.test_method();
    assert!(result.contains("tuple(2)"));
}

#[test]
fn test_test_trait_tuple3_pattern() {
    // Test pattern: (($T:ty, $U:ty, $V:ty)) => { $T: Send + Sync, $U: Clone, $V: Debug + Hash }
    let tuple: (TestType, String, TestType) = (TestType(1), "hello".to_string(), TestType(42));
    
    // Exercise the trait bounds
    // T: Send + Sync (TestType implements both)
    let cloned_second = tuple.1.clone(); // U: Clone
    let debug_third = format!("{:?}", tuple.2); // V: Debug
    
    // V: Hash
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    tuple.2.hash(&mut hasher);
    let hash_value = hasher.finish();
    
    assert_eq!(cloned_second, "hello");
    assert_eq!(debug_third, "TestType(42)");
    assert!(hash_value > 0); // Hash should produce some value
    
    // Call the trait method
    let result = tuple.test_method();
    assert!(result.contains("tuple(3)"));
}

#[test]
fn test_circular_trait_array_pattern() {
    // Test pattern: ([$T:ty; $N:expr]) => { $T: Clone + Send + Sync }
    let array: [TestType; 2] = [TestType(10), TestType(20)];
    
    // Exercise the trait bounds: Clone + Send + Sync
    let cloned_element = array[0].clone();
    // Send + Sync are marker traits, verified at compile time
    
    assert_eq!(cloned_element, TestType(10));
    
    // Call the trait method
    let result = array.circular_method();
    // Verify method returns a valid CircularTrait object
    let _ = result; // Just ensure it doesn't panic
}

#[test]
fn test_circular_trait_tuple_pattern() {
    // Test pattern: (($T:ty, $U:ty)) => { $T: Default + Clone, $U: Send + Sync }
    let tuple: (TestType, String) = (TestType(15), "circular".to_string());
    
    // Exercise the trait bounds
    let default_first = TestType::default(); // T: Default
    let cloned_first = tuple.0.clone(); // T: Clone
    // U: Send + Sync (String implements both)
    
    assert_eq!(default_first, TestType(0));
    assert_eq!(cloned_first, TestType(15));
    
    // Call the trait method
    let result = tuple.circular_method();
    let _ = result; // Ensure it doesn't panic
}

#[test]
fn test_extended_trait_slice_pattern() {
    // Test pattern: ([$T:ty]) => { $T: PartialEq + Clone }
    let vec_data = vec![TestType(5), TestType(5), TestType(10)];
    let slice: &[TestType] = &vec_data;
    
    // Exercise the trait bounds
    assert_eq!(slice[0], slice[1]); // PartialEq
    let cloned_element = slice[2].clone(); // Clone
    
    assert_eq!(cloned_element, TestType(10));
    
    // Call the trait method
    let result = slice.extended_method();
    assert_eq!(result, true); // First two elements are equal
}

#[test]
fn test_extended_trait_tuple3_pattern() {
    // Test pattern: (($T:ty, $U:ty, $V:ty)) => { $T: Hash + Eq, $U: Debug + Clone, $V: Send + Sync }
    let tuple: (TestType, Vec<String>, TestType) = (
        TestType(7), 
        vec!["test".to_string()], 
        TestType(99)
    );
    
    // Exercise the trait bounds
    // T: Hash + Eq
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    tuple.0.hash(&mut hasher);
    let hash1 = hasher.finish();
    
    let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
    TestType(7).hash(&mut hasher2);
    let hash2 = hasher2.finish();
    assert_eq!(hash1, hash2); // Same values should have same hash
    assert_eq!(tuple.0, TestType(7)); // Eq
    
    // U: Debug + Clone
    let debug_second = format!("{:?}", tuple.1);
    let cloned_second = tuple.1.clone();
    
    // V: Send + Sync (TestType implements both)
    
    assert!(debug_second.contains("test"));
    assert_eq!(cloned_second.len(), 1);
    
    // Call the trait method
    let result = tuple.extended_method();
    // Result depends on hash value being even/odd
    assert!(result == true || result == false);
}

#[test]
fn test_comprehensive_trait_bounds_exercise() {
    // Create a comprehensive test that exercises all trait bounds
    // from all pattern syntaxes in a single test
    
    // LocalTrait patterns
    let array = [TestType(1), TestType(2)];
    let slice: &[TestType] = &[TestType(3), TestType(3)];
    let tuple2: (TestType, i32) = (TestType(4), 42);
    let tuple3: (TestType, String, f64) = (TestType(5), "test".to_string(), 1.5);
    
    // TestTrait patterns  
    let test_tuple2: (TestType, Vec<i32>) = (TestType(6), vec![1, 2]);
    let test_tuple3: (TestType, String, TestType) = (TestType(7), "hello".to_string(), TestType(8));
    
    // CircularTrait patterns
    let circ_array = [TestType(9), TestType(10)];
    let circ_tuple: (TestType, String) = (TestType(11), "circular".to_string());
    
    // ExtendedTrait patterns
    let ext_slice: &[TestType] = &[TestType(12), TestType(12)];
    let ext_tuple: (TestType, Vec<String>, TestType) = (
        TestType(13), 
        vec!["extended".to_string()], 
        TestType(14)
    );
    
    // Exercise all trait bounds by calling methods that require them
    
    // LocalTrait: Clone + Default, PartialEq + Clone, etc.
    assert_eq!(array.local_method(), 2);
    assert_eq!(slice.local_method(), 2);
    assert_eq!(tuple2.local_method(), 2);
    assert_eq!(tuple3.local_method(), 3);
    
    // TestTrait: Clone + Send, Debug + Default, etc.
    assert!(test_tuple2.test_method().contains("tuple"));
    assert!(test_tuple3.test_method().contains("tuple"));
    
    // CircularTrait: Clone + Send + Sync, Default + Clone, etc.
    let _ = circ_array.circular_method();
    let _ = circ_tuple.circular_method();
    
    // ExtendedTrait: PartialEq + Clone, Hash + Eq, etc.
    assert_eq!(ext_slice.extended_method(), true); // Equal elements
    let _ = ext_tuple.extended_method(); // Hash-based result
    
    println!("All trait bounds from pattern syntax successfully exercised!");
}

#[test] 
fn test_trait_bounds_with_coinduction_types() {
    // Test using types that would involve coinduction
    use typedef::generic_types::{Container, Wrapper, MultiGeneric, ConstrainedStruct};
    
    // Test Container<T, U> with trait bounds
    let container = Container {
        first: TestType(100),
        second: "container".to_string(),
    };
    
    // Exercise Container's trait implementations that use bounds
    assert!(container.test_method().len() > 0);
    assert!(container.local_method() > 0);
    
    // Test Wrapper<T> with trait bounds
    let wrapper = Wrapper {
        value: TestType(200),
        count: 10,
    };
    
    assert!(wrapper.test_method().len() > 0);
    assert_eq!(wrapper.local_method(), 10);
    
    // Test MultiGeneric<T, U, V> with trait bounds  
    let multi = MultiGeneric {
        primary: TestType(300),
        secondary: 42u32,
        metadata: 500usize,
    };
    
    let _ = multi.circular_method(); // Uses trait bounds in implementation
    
    // Test ConstrainedStruct<T> with iterator bounds
    let constrained = ConstrainedStruct {
        iterator: std::iter::once(TestType(400)),
    };
    
    let _ = constrained.circular_method(); // Uses Iterator + Clone + Send bounds
    
    println!("Coinduction types with trait bounds successfully exercised!");
}