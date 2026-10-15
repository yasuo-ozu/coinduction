//! The module of an announced type and the modules using it name the trait differently.
#![allow(dead_code)]
use coinduction::*;

pub mod traits {
    #[announce::announce]
    pub trait Count {
        fn count(&self) -> usize;
    }
}

#[coinduction]
pub mod wrappers {
    use announce::announce;
    #[coinduction]
    use super::traits::Count;

    #[announce]
    pub struct Boxed<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

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
pub mod full_paths {
    use announce::announce;
    #[coinduction]
    use super::traits::Count;

    // The impl names the trait by its path
    #[announce]
    pub struct Pair<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> super::traits::Count for Pair<T>
    where
        T: super::traits::Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 2
        }
    }
}

// Uses the trait by its path
#[coinduction]
pub mod by_path {
    use super::wrappers::Boxed;
    #[coinduction]
    use super::traits::Count;

    pub struct Node(pub Boxed<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl super::traits::Count for Node
    where
        Boxed<Leaf>: super::traits::Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl super::traits::Count for Leaf
    where
        Node: super::traits::Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

// The type names the trait by its path, this module by the imported name
#[coinduction]
pub mod imported {
    use super::full_paths::Pair;
    #[coinduction]
    use super::traits::Count;

    pub struct Node(pub Pair<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        Pair<Leaf>: Count,
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

use traits::Count;

#[test]
fn by_path() {
    use by_path::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(wrappers::Boxed(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.count(), 4);
}

#[test]
fn imported() {
    use imported::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(full_paths::Pair(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.count(), 5);
}
