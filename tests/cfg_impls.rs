//! Impls under `#[cfg(...)]` take part in the resolution, and are announced, only where they
//! exist.
#![allow(dead_code)]
use coinduction::*;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

pub trait Never {}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::Count;

    // The disabled impl would add `T: Never` to the impls of the modules using `Boxed`
    #[announce::announce]
    #[derive(Debug)]
    pub struct Boxed<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    #[cfg(any())]
    impl<T> Count for Boxed<T>
    where
        T: Count + super::Never,
    {
        fn count(&self) -> usize {
            0
        }
    }

    #[cfg(test)]
    impl<T> Count for Boxed<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }

    // Several conditions, and an impl without one
    #[derive(announce::Announce)]
    pub struct Pair<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Pair<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 2
        }
    }

    #[cfg(test)]
    #[cfg(not(test))]
    impl<T> Count for Pair<T>
    where
        T: Count + super::Never,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

#[coinduction]
pub mod tree {
    use super::wrappers::{Boxed, Pair};
    #[coinduction]
    use super::Count;

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

    pub struct PairNode(pub Pair<PairLeaf>);
    pub struct PairLeaf(pub Option<Box<PairNode>>);

    impl Count for PairNode
    where
        Pair<PairLeaf>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for PairLeaf
    where
        PairNode: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

// A disabled impl in the module of the cycle, with `$` and inner attributes in the module
#[coinduction]
pub mod local {
    #![allow(unused_macros)]
    #[coinduction]
    use super::Count;

    macro_rules! plus_one {
        ($x:expr) => {
            $x + 1
        };
    }

    pub struct Node(pub Option<Box<Leaf>>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        Leaf: Count,
    {
        fn count(&self) -> usize {
            plus_one!(self.0.as_ref().map_or(0, |n| n.count()))
        }
    }

    #[cfg(any())]
    impl Count for Leaf
    where
        Node: Count + super::Never,
    {
        fn count(&self) -> usize {
            0
        }
    }

    #[cfg(not(any()))]
    impl Count for Leaf
    where
        Node: Count,
    {
        fn count(&self) -> usize {
            plus_one!(self.0.as_ref().map_or(0, |n| n.count()))
        }
    }
}

// A disabled impl of a type announced in another crate
#[coinduction]
pub mod cross_crate {
    #[coinduction]
    use traitdef::LocalTrait;
    use typedef::generic_types::Chain;

    pub struct Link(pub Chain<Link>);

    impl LocalTrait for Link
    where
        Chain<Link>: LocalTrait,
    {
        fn local_method(&self) -> usize {
            self.0.local_method() + 1
        }
    }
}

#[test]
fn cross_crate_disabled_impl() {
    use cross_crate::Link;
    use traitdef::LocalTrait;
    use typedef::generic_types::Chain;
    let link = Link(Chain(Some(Box::new(Link(Chain(None))))));
    assert_eq!(link.local_method(), 4);
}

#[test]
fn local_disabled_impl() {
    let leaf = local::Leaf(Some(Box::new(local::Node(None))));
    assert_eq!(leaf.count(), 2);
}

#[test]
fn disabled_impl() {
    use tree::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(wrappers::Boxed(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.count(), 4);
    // Other attributes of the type are kept
    assert_eq!(format!("{:?}", wrappers::Boxed::<u8>(None)), "Boxed(None)");
}

#[test]
fn several_conditions() {
    use tree::{PairLeaf, PairNode};
    let leaf = PairLeaf(Some(Box::new(PairNode(wrappers::Pair(Some(Box::new(PairLeaf(None))))))));
    assert_eq!(leaf.count(), 5);
}
