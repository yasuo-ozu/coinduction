//! The types of `tests/reexport`, a crate that uses announce through `coinduction::announce`, in a
//! cycle of another crate.
#![allow(dead_code)]
use coinduction::coinduction;

#[coinduction]
pub mod cycle {
    use reexport::wrappers::{ByPath, CrateAlias, CrateAliasMacro as CrateAlias, DeriveImported};
    #[coinduction]
    use reexport::Count;

    // Through the announced types
    pub struct Node(pub ByPath<Leaf>, pub DeriveImported<Leaf>, pub CrateAlias<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        ByPath<Leaf>: Count,
        DeriveImported<Leaf>: Count,
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

#[test]
fn reexport() {
    use cycle::{Leaf, List, Node};
    use reexport::wrappers::*;
    use reexport::Count;
    let node = Node(ByPath(None), DeriveImported(None), CrateAlias(None));
    assert_eq!(node.count(), 1 + 2 + 3);
    assert_eq!(Leaf(Some(Box::new(node))).count(), 7);
    assert_eq!(List(Some(Box::new((1, List(None))))).count(), 1);
}
