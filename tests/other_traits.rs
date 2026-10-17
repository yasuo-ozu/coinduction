//! The rules of a trait and the impls of an announced type name another trait differently from
//! the module using them.
#![allow(dead_code)]
use coinduction::*;

pub mod traits {
    #[announce::announce(meta = coinduction::rules(
        (($T:ty, $U:ty)) => { $T: crate::traits::Weight, $U: crate::traits::Size }
    ))]
    pub trait Size {
        fn size(&self) -> usize;
    }

    #[announce::announce]
    pub trait Weight {
        fn weight(&self) -> usize;
    }

    impl<T: Weight, U: Size> Size for (T, U) {
        fn size(&self) -> usize {
            self.0.weight() + self.1.size()
        }
    }
}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::traits::{Size, Weight};

    // The impl names the other trait by its path
    #[announce::announce]
    pub struct Tagged<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Size for Tagged<T>
    where
        T: super::traits::Weight,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.weight()) + 1
        }
    }
}

#[coinduction]
pub mod user {
    use super::wrappers::Tagged;
    #[coinduction]
    use super::traits::{Size, Weight};

    // List -> (List, List) -> List: Weight -> List
    pub struct List(pub Option<Box<(List, List)>>);

    impl Size for List
    where
        (List, List): Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size()) + 1
        }
    }

    impl Weight for List
    where
        List: Size,
    {
        fn weight(&self) -> usize {
            self.size() * 10
        }
    }

    // Node -> Tagged<Node> -> Node: Weight -> Node
    pub struct Node(pub Tagged<Node>);

    impl Size for Node
    where
        Tagged<Node>: Size,
    {
        fn size(&self) -> usize {
            self.0.size() + 1
        }
    }

    impl Weight for Node
    where
        Node: Size,
    {
        fn weight(&self) -> usize {
            self.size() * 10
        }
    }
}

// Names the other trait by its path
#[coinduction]
pub mod by_path {
    use super::wrappers::Tagged;
    #[coinduction]
    use super::traits::{Size, Weight};

    pub struct List(pub Option<Box<(List, List)>>);

    impl Size for List
    where
        (List, List): Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size()) + 1
        }
    }

    impl super::traits::Weight for List
    where
        List: Size,
    {
        fn weight(&self) -> usize {
            self.size() * 10
        }
    }

    pub struct Node(pub Tagged<Node>);

    impl Size for Node
    where
        Tagged<Node>: Size,
    {
        fn size(&self) -> usize {
            self.0.size() + 1
        }
    }

    impl super::traits::Weight for Node
    where
        Node: Size,
    {
        fn weight(&self) -> usize {
            self.size() * 10
        }
    }
}

// Imports the other trait under another name
#[coinduction]
pub mod aliased {
    use super::wrappers::Tagged;
    #[coinduction]
    use super::traits::{Size, Weight as Heavy};

    pub struct List(pub Option<Box<(List, List)>>);

    impl Size for List
    where
        (List, List): Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size()) + 1
        }
    }

    impl Heavy for List
    where
        List: Size,
    {
        fn weight(&self) -> usize {
            self.size() * 10
        }
    }

    pub struct Node(pub Tagged<Node>);

    impl Size for Node
    where
        Tagged<Node>: Size,
    {
        fn size(&self) -> usize {
            self.0.size() + 1
        }
    }

    impl Heavy for Node
    where
        Node: Size,
    {
        fn weight(&self) -> usize {
            self.size() * 10
        }
    }
}

// The rules of `LocalTrait` name `::core::clone::Clone`, which is not the module's working trait
// `Clone` although it has the same name: the module marks no traits, so its impl of `Clone` makes
// that one
#[coinduction]
pub mod std_trait {
    use traitdef::LocalTrait;

    #[derive(Default)]
    pub struct Node(pub Option<Box<(u8, Node)>>);

    impl Clone for Node {
        fn clone(&self) -> Self {
            Node(self.0.clone())
        }
    }

    impl LocalTrait for Node
    where
        (u8, Node): LocalTrait,
    {
        fn local_method(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.local_method()) + 1
        }
    }
}

use traits::{Size, Weight};

#[test]
fn rules() {
    use user::List;
    let list = List(Some(Box::new((List(None), List(None)))));
    assert_eq!(list.size(), 12);
    assert_eq!(List(None).weight(), 10);
}

#[test]
fn impls() {
    use user::Node;
    use wrappers::Tagged;
    let node = Node(Tagged(Some(Box::new(Node(Tagged(None))))));
    assert_eq!(node.size(), 22);
}

#[test]
fn by_path() {
    use by_path::{List, Node};
    use wrappers::Tagged;
    assert_eq!(List(Some(Box::new((List(None), List(None))))).size(), 12);
    assert_eq!(Node(Tagged(Some(Box::new(Node(Tagged(None)))))).size(), 22);
}

#[test]
fn aliased() {
    use aliased::{List, Node};
    use wrappers::Tagged;
    assert_eq!(List(Some(Box::new((List(None), List(None))))).size(), 12);
    assert_eq!(Node(Tagged(Some(Box::new(Node(Tagged(None)))))).size(), 22);
}

#[test]
fn std_trait() {
    use std_trait::Node;
    use traitdef::LocalTrait;
    let node = Node(Some(Box::new((1, Node(None)))));
    assert_eq!(node.local_method(), 3);
}
