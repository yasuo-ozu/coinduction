//! A crate without a dependency on announce uses it through `coinduction::announce`, and its types
//! are used in a cycle of another crate.
#![allow(dead_code)]
use coinduction::coinduction;
use subcrate::subcrate;

// Depends on `coinduction` only
#[subcrate(::coinduction)]
pub mod reexport {
    #![allow(dead_code)]
    use coinduction::coinduction;

    // Outside a `#[coinduction]` module, the path to announce is given
    #[coinduction::announce::announce(
        crate = ::coinduction::announce,
        meta = coinduction::rules((($T:ty, $U:ty)) => { $T: Count, $U: Count }, (u8) => {})
    )]
    pub trait Count {
        fn count(&self) -> usize;
    }

    impl Count for u8 {
        fn count(&self) -> usize {
            1
        }
    }

    impl<T: Count, U: Count> Count for (T, U) {
        fn count(&self) -> usize {
            self.0.count() + self.1.count()
        }
    }

    // In a `#[coinduction]` module, `crate = ::coinduction::announce` is added
    #[coinduction]
    pub mod wrappers {
        #[coinduction]
        use super::Count;
        use coinduction::announce as ann;
        use coinduction::announce::announce as announced;

        #[coinduction::announce::announce]
        pub struct ByPath<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> Count for ByPath<T>
        where
            T: Count,
        {
            fn count(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.count()) + 1
            }
        }

        #[announced]
        pub struct Imported<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> Count for Imported<T>
        where
            T: Count,
        {
            fn count(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.count()) + 2
            }
        }

        #[ann::announce(macro_name = CrateAliasMacro)]
        pub struct CrateAlias<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> Count for CrateAlias<T>
        where
            T: Count,
        {
            fn count(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.count()) + 3
            }
        }
    }

    // The attribute imported from the re-export under its own name, and the re-export under the name of
    // the crate (also with `self`): the imports shadow announce's own names
    #[coinduction]
    pub mod own_names {
        #[coinduction]
        use super::Count;
        use coinduction::announce::announce;

        #[announce]
        pub struct Plain<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> Count for Plain<T>
        where
            T: Count,
        {
            fn count(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.count()) + 4
            }
        }
    }

    #[coinduction]
    pub mod crate_name {
        #[coinduction]
        use super::Count;
        use coinduction::announce;
        use coinduction::announce::{self as announce_self};

        #[announce::announce]
        pub struct CrateName<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> Count for CrateName<T>
        where
            T: Count,
        {
            fn count(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.count()) + 5
            }
        }

        #[announce_self::announce]
        pub struct SelfAlias<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> Count for SelfAlias<T>
        where
            T: Count,
        {
            fn count(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.count()) + 6
            }
        }
    }

    // A trait announced in a `#[coinduction]` module gets `crate = ::coinduction::announce` too
    #[coinduction]
    pub mod local {
        #[coinduction::announce::announce]
        pub trait Depth {
            fn depth(&self) -> usize;
        }

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
}

#[coinduction]
pub mod cycle {
    use reexport::crate_name::{CrateName, SelfAlias};
    use reexport::own_names::Plain;
    use reexport::wrappers::{ByPath, CrateAlias, CrateAliasMacro as CrateAlias, Imported};
    #[coinduction]
    use reexport::Count;

    // Through the announced types
    pub struct Node(
        pub ByPath<Leaf>,
        pub Imported<Leaf>,
        pub CrateAlias<Leaf>,
        pub Plain<Leaf>,
        pub CrateName<Leaf>,
        pub SelfAlias<Leaf>,
    );
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        ByPath<Leaf>: Count,
        Imported<Leaf>: Count,
        CrateAlias<Leaf>: Count,
        Plain<Leaf>: Count,
        CrateName<Leaf>: Count,
        SelfAlias<Leaf>: Count,
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

// The trait announced in a module of `reexport`
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
    use reexport::crate_name::{CrateName, SelfAlias};
    use reexport::own_names::Plain;
    use reexport::wrappers::*;
    use reexport::Count;
    let node = Node(
        ByPath(None),
        Imported(None),
        CrateAlias(None),
        Plain(None),
        CrateName(None),
        SelfAlias(None),
    );
    assert_eq!(node.count(), 1 + 2 + 3 + 4 + 5 + 6);
    assert_eq!(Leaf(Some(Box::new(node))).count(), 22);
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
