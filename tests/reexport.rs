//! The types of `tests/reexport`, a crate that uses announce through `coinduction::announce`, in a
//! cycle of another crate.
#![allow(dead_code)]
use coinduction::coinduction;

#[coinduction]
pub mod cycle {
    use reexport::wrappers::{ByPath, CrateAlias, CrateAliasMacro as CrateAlias, Imported};
    #[coinduction]
    use reexport::Count;

    // Through the announced types
    pub struct Node(pub ByPath<Leaf>, pub Imported<Leaf>, pub CrateAlias<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        ByPath<Leaf>: Count,
        Imported<Leaf>: Count,
        CrateAlias<Leaf>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + self.1.count() + self.2.count()
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

    // Through the rules
    pub struct List(pub Option<Box<(u8, List)>>);

    impl Count for List
    where
        (u8, List): Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.count())
        }
    }
}

// The trait announced in a module of `tests/reexport`
#[coinduction]
pub mod depth {
    #[coinduction]
    use reexport::local::Depth;

    pub struct Node(pub Option<Box<Leaf>>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Depth for Node
    where
        Leaf: Depth,
    {
        fn depth(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.depth()) + 1
        }
    }

    impl Depth for Leaf
    where
        Node: Depth,
    {
        fn depth(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.depth()) + 1
        }
    }
}

#[test]
fn reexport() {
    use cycle::{Leaf, List, Node};
    use reexport::wrappers::*;
    use reexport::Count;
    let node = Node(ByPath(None), Imported(None), CrateAlias(None));
    assert_eq!(node.count(), 1 + 2 + 3);
    assert_eq!(Leaf(Some(Box::new(node))).count(), 7);
    assert_eq!(List(Some(Box::new((1, List(None))))).count(), 1);
}

#[test]
fn trait_in_module() {
    use depth::{Leaf, Node};
    use reexport::local::Depth;
    assert_eq!(Leaf(Some(Box::new(Node(None)))).depth(), 2);
    let local = reexport::local::Leaf(Some(Box::new(reexport::local::Node(None))));
    assert_eq!(local.depth(), 2);
}
