use coinduction::*;
use traitdef::{TestTrait, LocalTrait, CircularTrait, TestMarker, LocalMarker, CoinductionTestMarker};
use typedef::{TypedefMarker, LocalTypeMarker};

// Local types that exist only in this crate
pub struct InternalType(pub f64);

// Local trait for coinduction testing
#[traitdef]
pub trait CoinductionLocalTrait {
    fn coinduction_method(&self) -> f64;
}

#[coinduction]
pub mod integration_circular {
    use super::*;
    use traitdef::{CircularTrait, LocalTrait, TestTrait};
    use std::marker::PhantomData;

    // Define mutually recursive structures
    pub struct NodeA<T> {
        pub value: String,
        pub child_b: Option<Box<NodeB<T>>>,
        pub phantom: PhantomData<T>,
    }

    pub struct NodeB<T> {
        pub count: usize,
        pub child_a: Option<Box<NodeA<T>>>,
        pub internal: InternalType,
        pub phantom: PhantomData<T>,
    }

    pub struct NodeC<T> {
        pub data: f64,
        pub ref_a: Option<Box<NodeA<T>>>,
        pub ref_b: Option<Box<NodeB<T>>>,
        pub phantom: PhantomData<T>,
    }

    // Implementations with where clauses using traits and types from test crates
    impl<T> TestTrait for NodeA<T>
    where
        typedef::circular_types::CircularA: LocalTrait,
        typedef::circular_types::CircularB: CircularTrait,
    {
        fn test_method(&self) -> String {
            format!("NodeA: {}", self.value)
        }
    }

    impl<T> LocalTrait for NodeB<T>
    where
        typedef::circular_types::CircularA: TestTrait,
        InternalType: CoinductionLocalTrait,
    {
        fn local_method(&self) -> usize {
            self.count + (self.internal.coinduction_method() as usize)
        }
    }

    impl<T: 'static> CircularTrait for NodeA<T>
    where
        typedef::circular_types::CircularA: LocalTrait,
        typedef::circular_types::CircularB: CircularTrait,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait> {
            Box::new(NodeB::<T> {
                count: self.value.len(),
                child_a: None,
                internal: InternalType(42.0),
                phantom: PhantomData,
            })
        }
    }

    impl<T: 'static> CircularTrait for NodeB<T>
    where
        typedef::circular_types::CircularA: LocalTrait,
        typedef::circular_types::CircularB: CircularTrait,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait> {
            Box::new(NodeC::<T> {
                data: self.count as f64,
                ref_a: None,
                ref_b: None,
                phantom: PhantomData,
            })
        }
    }

    impl<T: 'static> CircularTrait for NodeC<T>
    where
        NodeA<T>: TestTrait,
        NodeB<T>: LocalTrait,
        typedef::circular_types::CircularB: LocalTrait + CircularTrait,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait> {
            Box::new(NodeA::<T> {
                value: format!("Generated from NodeC: {}", self.data),
                child_b: None,
                phantom: PhantomData,
            })
        }
    }

    impl<T> CoinductionLocalTrait for NodeA<T>
    where
        typedef::circular_types::CircularA: LocalTrait,
        LocalTypeMarker: coinduction::TypeRef<String>,
    {
        fn coinduction_method(&self) -> f64 {
            self.value.len() as f64
        }
    }

    impl<T> CoinductionLocalTrait for NodeB<T>
    where
        TestMarker: coinduction::TypeRef<NodeA<T>>,
        LocalMarker: coinduction::TypeRef<NodeB<T>>,
    {
        fn coinduction_method(&self) -> f64 {
            self.count as f64 * self.internal.0
        }
    }

    impl<T> CoinductionLocalTrait for NodeC<T>
    where
        CoinductionTestMarker: coinduction::TypeRef<NodeC<T>>,
        NodeA<T>: TestTrait + CoinductionLocalTrait,
        NodeB<T>: LocalTrait + CoinductionLocalTrait,
    {
        fn coinduction_method(&self) -> f64 {
            self.data * 2.0
        }
    }
}

// Implementation for the local type
impl CoinductionLocalTrait for InternalType {
    fn coinduction_method(&self) -> f64 {
        self.0
    }
}

#[test]
fn test_traitdef_marker_typeref() {
    use coinduction::TypeRef;
    
    // Test that TypeRef implementation is generated for traitdef marker
    let _: <TestMarker as TypeRef<String>>::Type = String::new();
}

#[test]
fn test_typedef_marker_typeref() {
    use coinduction::TypeRef;
    
    // Test that TypeRef implementation is generated for typedef marker
    let _: <TypedefMarker as TypeRef<i32>>::Type = 42i32;
}

#[test]
fn test_basic_functionality() {
    use typedef::test_types::TestStruct;

    let test_struct = TestStruct("Hello, World!".to_string());
    assert_eq!(test_struct.test_method(), "Hello, World!");
}

#[test]
fn test_circular_coinduction() {
    use integration_circular::*;
    use std::marker::PhantomData;
    
    // Test circular structures with coinduction
    let node_a = NodeA::<()> {
        value: "Test Node A".to_string(),
        child_b: None,
        phantom: PhantomData,
    };

    let internal = InternalType(3.14);
    let node_b = NodeB::<()> {
        count: 5,
        child_a: None,
        internal,
        phantom: PhantomData,
    };

    let node_c = NodeC::<()> {
        data: 2.5,
        ref_a: None,
        ref_b: None,
        phantom: PhantomData,
    };

    // Test trait implementations
    assert_eq!(node_a.test_method(), "NodeA: Test Node A");
    assert_eq!(node_b.local_method(), 8); // 5 + 3 (truncated from 3.14)
    assert_eq!(node_a.coinduction_method(), 11.0); // "Test Node A".len() = 11
    assert!((node_b.coinduction_method() - 15.7).abs() < 1e-10); // 5 * 3.14 ≈ 15.7
    assert_eq!(node_c.coinduction_method(), 5.0); // 2.5 * 2.0 = 5.0

    // Test circular trait method
    let result = node_c.circular_method();
    // This should create a NodeA, but we can't directly test it since it's a trait object
    // We can at least verify the method doesn't panic
    let _ = result;
}

#[test]
fn test_complex_circular_references() {
    use typedef::circular_types::{CircularA, CircularB};

    // Test the complex circular structure
    let circ_b = CircularB {
        value: 10,
        reference_a: None,
    };

    let circ_a = CircularA {
        data: "circular test".to_string(),
        reference_b: Some(Box::new(circ_b)),
    };

    assert_eq!(circ_a.local_method(), 13); // "circular test".len() = 13

    if let Some(ref b) = circ_a.reference_b {
        assert_eq!(b.local_method(), 10);
    }

    // Test circular trait methods
    let result_a = circ_a.circular_method();
    let result_b = circ_a.reference_b.as_ref().unwrap().circular_method();

    // Verify methods don't panic
    let _ = result_a;
    let _ = result_b;
}

#[test]
fn test_typeref_with_complex_types() {
    use integration_circular::*;
    use coinduction::TypeRef;
    use std::marker::PhantomData;
    
    // Test TypeRef implementations with complex types from coinduction
    let _: <CoinductionTestMarker as TypeRef<NodeA<()>>>::Type = NodeA::<()> {
        value: "test".to_string(),
        child_b: None,
        phantom: PhantomData,
    };

    let _: <LocalTypeMarker as TypeRef<InternalType>>::Type = InternalType(1.0);
}

#[test]
fn test_circular_types_usage() {
    // Test using CircularA and CircularB imports
    use typedef::circular_types::{CircularA, CircularB};
    
    let circular_a = CircularA {
        data: "test circular A".to_string(),
        reference_b: None,
    };

    let circular_b = CircularB {
        value: 42,
        reference_a: None,
    };

    // Test their methods
    assert_eq!(circular_a.local_method(), 15); // "test circular A".len() = 15
    assert_eq!(circular_b.local_method(), 42);

    // Test circular trait methods
    let _result_a = circular_a.circular_method();
    let _result_b = circular_b.circular_method();
}

#[test]
fn test_typeref_direct_usage() {
    use integration_circular::*;
    use coinduction::TypeRef;
    use std::marker::PhantomData;
    
    // Test direct TypeRef usage with various markers
    let _: <TestMarker as TypeRef<String>>::Type = String::from("test");
    let _: <LocalMarker as TypeRef<NodeB<()>>>::Type = NodeB::<()> {
        count: 10,
        child_a: None,
        internal: InternalType(2.5),
        phantom: PhantomData,
    };
    let _: <CoinductionTestMarker as TypeRef<NodeC<()>>>::Type = NodeC::<()> {
        data: 3.14,
        ref_a: None,
        ref_b: None,
        phantom: PhantomData,
    };
}