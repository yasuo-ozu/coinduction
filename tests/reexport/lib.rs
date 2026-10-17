//! A crate without a dependency on announce: it uses announce through `coinduction::announce`. Its
//! types are used in a cycle of another crate in `tests/reexport.rs`.
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
