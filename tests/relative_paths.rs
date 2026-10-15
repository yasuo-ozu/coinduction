//! The impls of announced types are announced as written: generic trait arguments, imported
//! types and `crate::` paths are matched by modules that write them the same way.
#![allow(dead_code)]
use coinduction::*;

#[announce::announce]
pub trait Conv<T> {
    fn conv(&self) -> usize;
}

pub struct Local;

#[coinduction]
pub mod wrappers {
    use super::Local;
    #[coinduction]
    use super::Conv;

    // An imported type in the trait arguments
    #[announce::announce]
    pub struct Imported<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Conv<Local> for Imported<T>
    where
        T: Conv<Local>,
    {
        fn conv(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.conv()) + 1
        }
    }

    // A `crate::` path in the trait arguments
    #[announce::announce]
    pub struct Absolute<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Conv<crate::Local> for Absolute<T>
    where
        T: Conv<crate::Local>,
    {
        fn conv(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.conv()) + 2
        }
    }

    // A primitive in the trait arguments
    #[announce::announce]
    pub struct Primitive<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Conv<u8> for Primitive<T>
    where
        T: Conv<u8>,
    {
        fn conv(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.conv()) + 3
        }
    }
}

#[coinduction]
pub mod imported {
    use super::wrappers::Imported;
    use super::Local;
    #[coinduction]
    use super::Conv;

    pub struct Node(pub Imported<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Conv<Local> for Node
    where
        Imported<Leaf>: Conv<Local>,
    {
        fn conv(&self) -> usize {
            self.0.conv() + 1
        }
    }

    impl Conv<Local> for Leaf
    where
        Node: Conv<Local>,
    {
        fn conv(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.conv()) + 1
        }
    }
}

#[coinduction]
pub mod absolute {
    use super::wrappers::Absolute;
    #[coinduction]
    use super::Conv;

    pub struct Node(pub Absolute<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Conv<crate::Local> for Node
    where
        Absolute<Leaf>: Conv<crate::Local>,
    {
        fn conv(&self) -> usize {
            self.0.conv() + 1
        }
    }

    impl Conv<crate::Local> for Leaf
    where
        Node: Conv<crate::Local>,
    {
        fn conv(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.conv()) + 1
        }
    }
}

#[coinduction]
pub mod primitive {
    use super::wrappers::Primitive;
    #[coinduction]
    use super::Conv;

    pub struct Node(pub Primitive<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Conv<u8> for Node
    where
        Primitive<Leaf>: Conv<u8>,
    {
        fn conv(&self) -> usize {
            self.0.conv() + 1
        }
    }

    impl Conv<u8> for Leaf
    where
        Node: Conv<u8>,
    {
        fn conv(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.conv()) + 1
        }
    }
}

#[test]
fn imported_type() {
    use imported::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(wrappers::Imported(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.conv(), 4);
}

#[test]
fn absolute_path() {
    use absolute::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(wrappers::Absolute(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.conv(), 5);
}

#[test]
fn primitive_argument() {
    use primitive::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(wrappers::Primitive(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.conv(), 6);
}
