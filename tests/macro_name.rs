use coinduction::coinduction;

#[coinduction(macro_name = CountMacro)]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::{Count, CountMacro as Count};

    #[coinduction(macro_name = BoxedMacro)]
    pub struct Boxed<T>(pub Option<Box<T>>);

    impl<T> Count for Boxed<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }
}

#[coinduction]
pub mod tree {
    use super::wrappers::{Boxed, BoxedMacro as Boxed};
    #[coinduction]
    use super::{Count, CountMacro as Count};

    pub struct Node(pub Boxed<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        Boxed<Leaf>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for Leaf
    where
        Node: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

#[test]
fn test_macro_name() {
    use tree::{Leaf, Node};
    use wrappers::Boxed;

    let leaf = Leaf(Some(Box::new(Node(Boxed(Some(Box::new(Leaf(None))))))));
    // Leaf -> Node -> Boxed -> Leaf
    assert_eq!(leaf.count(), 4);
}
