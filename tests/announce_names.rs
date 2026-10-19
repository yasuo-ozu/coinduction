//! Announce's attribute is recognized in `cfg_attr`, among other attributes, through announce
//! imported under another name and through a re-export in this crate, so that the impls of the type
//! are announced. Other names for it are tested in `tests/reexport`.
#![allow(dead_code)]
use coinduction::*;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

pub mod reexports {
    pub use announce::announce;
}

#[coinduction]
pub mod wrappers {
    use announce as my;
    #[coinduction]
    use super::Count;

    // The attribute in `cfg_attr` with another attribute
    #[cfg_attr(not(any()), derive(Debug), announce::announce)]
    pub struct CfgWith<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for CfgWith<T>
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

    // Two announcements, the first one disabled: both carry the impls
    #[cfg_attr(any(), announce::announce(allow_primitive = true))]
    #[announce::announce]
    pub struct Merged<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Merged<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 5
        }
    }

    // Another attribute macro on the item: announce makes the trampoline after it
    #[announce::announce]
    #[rustversion::since(1.0)]
    pub struct Runner<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Runner<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 6
        }
    }

    // Announce imported under another name
    #[my::announce]
    pub struct Aliased<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Aliased<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 7
        }
    }

    // A re-export in this crate
    #[crate::reexports::announce]
    pub struct Reexported<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Reexported<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 8
        }
    }

    // A disabled attribute
    #[cfg_attr(any(), announce::announce)]
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
    use super::wrappers::{Aliased, CfgAttr, CfgWith, Merged, Reexported, Runner};
    #[coinduction]
    use super::Count;

    pub struct Node(
        pub CfgWith<Leaf>,
        pub CfgAttr<Leaf>,
        pub Merged<Leaf>,
        pub Runner<Leaf>,
        pub Aliased<Leaf>,
        pub Reexported<Leaf>,
    );
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        CfgWith<Leaf>: Count,
        CfgAttr<Leaf>: Count,
        Merged<Leaf>: Count,
        Runner<Leaf>: Count,
        Aliased<Leaf>: Count,
        Reexported<Leaf>: Count,
    {
        fn count(&self) -> usize {
            let Node(a, b, c, d, e, f) = self;
            a.count() + b.count() + c.count() + d.count() + e.count() + f.count()
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
    let node = Node(
        CfgWith(None),
        CfgAttr(None),
        Merged(None),
        Runner(None),
        Aliased(None),
        Reexported(None),
    );
    assert_eq!(node.count(), 3 + 4 + 5 + 6 + 7 + 8);
    assert_eq!(Leaf(Some(Box::new(node))).count(), 34);
    assert_eq!(format!("{:?}", CfgWith::<u8>(None)), "CfgWith(None)");
}
