//! The announce dependency is renamed to `ann` in `Cargo.toml`, and its attribute and derive are
//! also imported under other names.
#![allow(dead_code)]
use coinduction::coinduction;

#[ann::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::Count;
    use ann::announce as announced;
    use ann::Announce as Derived;

    #[ann::announce]
    pub struct ByPath<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for ByPath<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }

    #[derive(ann::Announce)]
    pub struct DeriveByPath<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for DeriveByPath<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 2
        }
    }

    #[announced]
    pub struct Imported<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Imported<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 3
        }
    }

    #[derive(Derived)]
    pub struct DeriveImported<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for DeriveImported<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 4
        }
    }
}

#[coinduction]
pub mod tree {
    use super::wrappers::{ByPath, DeriveByPath, DeriveImported, Imported};
    #[coinduction]
    use super::Count;

    pub struct Node(
        pub ByPath<Leaf>,
        pub DeriveByPath<Leaf>,
        pub Imported<Leaf>,
        pub DeriveImported<Leaf>,
    );
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        ByPath<Leaf>: Count,
        DeriveByPath<Leaf>: Count,
        Imported<Leaf>: Count,
        DeriveImported<Leaf>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + self.1.count() + self.2.count() + self.3.count()
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

#[cfg(test)]
mod tests {
    use super::tree::{Leaf, Node};
    use super::wrappers::*;
    use super::Count;

    #[test]
    fn renamed() {
        let node = Node(ByPath(None), DeriveByPath(None), Imported(None), DeriveImported(None));
        assert_eq!(node.count(), 1 + 2 + 3 + 4);
        assert_eq!(Leaf(Some(Box::new(node))).count(), 11);
    }
}
