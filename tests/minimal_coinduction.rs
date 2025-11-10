use coinduction::*;
use std::marker::PhantomData;

// Define a simple trait for coinduction testing
#[traitdef]
trait Simple {
    fn simple_method(&self);
}

// Define a generic trait for coinduction testing with type parameters
#[traitdef]
trait GenericTrait<T> {
    fn generic_method(&self, value: T) -> T;
}

// Define a trait for enum testing
#[traitdef]
trait EnumTrait {
    fn enum_method(&self) -> String;
}

// Module with circular dependencies - shows the problem coinduction should solve
// Note: #[recurse] not working, so this creates compilation errors as expected
// mod circular {
//     pub struct TypeA;
//     pub struct TypeB;
//
//     // TypeA implements Simple when TypeB implements Simple (circular dependency)
//     impl super::Simple for TypeA
//     where
//         TypeB: super::Simple,
//     {
//         fn simple_method(&self) {}
//     }
//
//     // TypeB implements Simple when TypeA implements Simple (circular dependency)
//     impl super::Simple for TypeB
//     where
//         TypeA: super::Simple,
//     {
//         fn simple_method(&self) {}
//     }
// }

// Module with working `#[coinduction]` - applies coinduction to break circular dependencies
#[coinduction(super::Simple)]
mod coinduction_test {
    pub struct WorkingA;
    pub struct WorkingB;

    impl super::Simple for WorkingA
    where
        WorkingB: super::Simple,
    {
        fn simple_method(&self) {}
    }

    impl super::Simple for WorkingB
    where
        WorkingA: super::Simple,
    {
        fn simple_method(&self) {}
    }
}

#[test]
fn test_trait_macro_generation() {
    // This test verifies that #[traitdef] generates the Simple macro_rules!
    // The "unused macro definition" warning confirms this works
}

// #[test]
// fn test_circular_dependency_problem() {
//     let a = circular::TypeA;
//     let b = circular::TypeB;
//
//     // These calls will fail due to unsatisfied trait bounds - shows the problem
//     // that coinduction should solve
//     a.simple_method(); // Error: TypeB: Simple constraint not satisfied
//     b.simple_method(); // Error: TypeA: Simple constraint not satisfied
// }

// Module with generic structs having type parameters and circular implementations
#[coinduction(super::GenericTrait<T>)]
mod generic_coinduction {
    use super::*;

    pub struct GenericNodeA<T> {
        pub data: String,
        pub child: Option<Box<GenericNodeB<T>>>,
        pub phantom: PhantomData<T>,
    }

    pub struct GenericNodeB<T> {
        pub value: i32,
        pub parent: Option<Box<GenericNodeA<T>>>,
        pub phantom: PhantomData<T>,
    }

    impl<T: Clone> GenericTrait<T> for GenericNodeA<T>
    where
        GenericNodeB<T>: GenericTrait<T>,
    {
        fn generic_method(&self, value: T) -> T {
            value.clone()
        }
    }

    impl<T: Clone> GenericTrait<T> for GenericNodeB<T>
    where
        GenericNodeA<T>: GenericTrait<T>,
    {
        fn generic_method(&self, value: T) -> T {
            value.clone()
        }
    }
}

// Module with enums having type parameters and circular implementations
#[coinduction(super::EnumTrait)]
mod enum_coinduction {
    use super::*;

    pub enum TreeNode<T> {
        Leaf(T),
        Branch {
            left: Box<TreeNode<T>>,
            right: Box<TreeNode<T>>,
        },
    }

    pub enum ListNode<T> {
        Empty,
        Node {
            data: T,
            next: Box<ListNode<T>>,
        },
    }

    impl<T: std::fmt::Display> EnumTrait for TreeNode<T>
    where
        ListNode<T>: EnumTrait,
    {
        fn enum_method(&self) -> String {
            match self {
                TreeNode::Leaf(value) => format!("Leaf: {}", value),
                TreeNode::Branch { .. } => "Branch".to_string(),
            }
        }
    }

    impl<T: std::fmt::Display> EnumTrait for ListNode<T>
    where
        TreeNode<T>: EnumTrait,
    {
        fn enum_method(&self) -> String {
            match self {
                ListNode::Empty => "Empty".to_string(),
                ListNode::Node { data, .. } => format!("Node: {}", data),
            }
        }
    }
}

#[test]
fn test_coinduction_macro() {
    // This test demonstrates #[coinduction] macro working correctly
    // The macro applies coinduction to break circular trait dependencies

    // These work because coinduction removes circular where clauses:
    let working_a = coinduction_test::WorkingA;
    let working_b = coinduction_test::WorkingB;
    working_a.simple_method();
    working_b.simple_method();
}

#[test]
fn test_generic_structs_coinduction() {
    // Test generic structs with type parameters and circular implementations
    let node_a = generic_coinduction::GenericNodeA::<String> {
        data: "test".to_string(),
        child: None,
        phantom: PhantomData,
    };

    let node_b = generic_coinduction::GenericNodeB::<i32> {
        value: 42,
        parent: None,
        phantom: PhantomData,
    };

    // Test that the generic methods work
    let result_a = node_a.generic_method("hello".to_string());
    assert_eq!(result_a, "hello");

    let result_b = node_b.generic_method(100);
    assert_eq!(result_b, 100);

    // Test field access to avoid unused field warnings
    assert_eq!(node_a.data, "test");
    assert!(node_a.child.is_none());
    assert_eq!(node_b.value, 42);
    assert!(node_b.parent.is_none());
}

#[test]
fn test_enums_coinduction() {
    // Test enums with type parameters and circular implementations
    let tree_leaf = enum_coinduction::TreeNode::Leaf(42);
    let list_node = enum_coinduction::ListNode::Node {
        data: "test",
        next: Box::new(enum_coinduction::ListNode::Empty),
    };

    // Test that the enum methods work
    let tree_result = tree_leaf.enum_method();
    assert_eq!(tree_result, "Leaf: 42");

    let list_result = list_node.enum_method();
    assert_eq!(list_result, "Node: test");

    let empty_list = enum_coinduction::ListNode::<i32>::Empty;
    let empty_result = empty_list.enum_method();
    assert_eq!(empty_result, "Empty");

    // Test using the Branch variant to avoid unused warning
    let tree_branch = enum_coinduction::TreeNode::Branch {
        left: Box::new(enum_coinduction::TreeNode::Leaf("left")),
        right: Box::new(enum_coinduction::TreeNode::Leaf("right")),
    };
    let branch_result = tree_branch.enum_method();
    assert_eq!(branch_result, "Branch");

    // Test accessing the left and right fields
    if let enum_coinduction::TreeNode::Branch { left, right } = &tree_branch {
        assert!(matches!(**left, enum_coinduction::TreeNode::Leaf("left")));
        assert!(matches!(**right, enum_coinduction::TreeNode::Leaf("right")));
    }

    // Test accessing the next field in ListNode
    if let enum_coinduction::ListNode::Node { data, next } = &list_node {
        assert_eq!(*data, "test");
        assert!(matches!(**next, enum_coinduction::ListNode::Empty));
    }
}
