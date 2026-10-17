//! Announce's attribute and derive are recognized in `cfg_attr`, so that the impls of the type are
//! announced. Other names for them are tested in `tests/renamed`.
#![allow(dead_code)]
use coinduction::*;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::Count;

    // The derive in `cfg_attr`: the helper attribute is added under the same condition
    #[cfg_attr(not(any()), derive(Debug, announce::Announce))]
    pub struct CfgDerive<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for CfgDerive<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 3
        }
    }

    // The attribute in nested `cfg_attr`s
    #[cfg_attr(all(), cfg_attr(not(any()), announce::announce))]
    pub struct CfgAttr<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for CfgAttr<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 4
        }
    }

    // A disabled derive: no helper attribute may be left without it
    #[cfg_attr(any(), derive(announce::Announce))]
    pub struct Disabled<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Disabled<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

#[coinduction]
pub mod tree {
    use super::wrappers::{CfgAttr, CfgDerive};
    #[coinduction]
    use super::Count;

    pub struct Node(pub CfgDerive<Leaf>, pub CfgAttr<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        CfgDerive<Leaf>: Count,
        CfgAttr<Leaf>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + self.1.count()
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
fn recognized() {
    use tree::{Leaf, Node};
    use wrappers::*;
    let node = Node(CfgDerive(None), CfgAttr(None));
    assert_eq!(node.count(), 3 + 4);
    assert_eq!(Leaf(Some(Box::new(node))).count(), 8);
    assert_eq!(format!("{:?}", CfgDerive::<u8>(None)), "CfgDerive(None)");
}
