//! Const generic arguments of announced types.
#![allow(dead_code)]
use coinduction::*;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
pub mod arrays {
    #[coinduction]
    use super::Count;

    #[announce::announce]
    pub struct Arr<T, const N: usize>(pub [::core::option::Option<::std::boxed::Box<T>>; N]);

    impl<T, const N: usize> Count for Arr<T, N>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.iter().flatten().map(|t| t.count()).sum::<usize>() + N
        }
    }

    // A const parameter of another type than `usize`
    #[announce::announce]
    pub struct Flag<T, const B: bool>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T, const B: bool> Count for Flag<T, B>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + B as usize
        }
    }

    // A literal const argument in the impl
    #[announce::announce]
    pub struct Fixed<T, const N: usize>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Fixed<T, 3>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 3
        }
    }
}

#[coinduction]
pub mod tree {
    use super::arrays::Arr;
    #[coinduction]
    use super::Count;

    pub struct Node(pub Arr<Leaf, 2>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Count for Node
    where
        Arr<Leaf, 2>: Count,
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

#[coinduction]
pub mod others {
    use super::arrays::{Arr, Fixed, Flag};
    #[coinduction]
    use super::Count;

    pub struct FlagNode(pub Flag<FlagLeaf, true>);
    pub struct FlagLeaf(pub Option<Box<FlagNode>>);

    impl Count for FlagNode
    where
        Flag<FlagLeaf, true>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for FlagLeaf
    where
        FlagNode: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }

    pub struct FixedNode(pub Fixed<FixedLeaf, 3>);
    pub struct FixedLeaf(pub Option<Box<FixedNode>>);

    impl Count for FixedNode
    where
        Fixed<FixedLeaf, 3>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for FixedLeaf
    where
        FixedNode: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }

    // The const parameter of this impl is passed on
    pub struct GenericNode<const M: usize>(pub Arr<GenericLeaf<M>, M>);
    pub struct GenericLeaf<const M: usize>(pub Option<Box<GenericNode<M>>>);

    impl<const M: usize> Count for GenericNode<M>
    where
        Arr<GenericLeaf<M>, M>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl<const M: usize> Count for GenericLeaf<M>
    where
        GenericNode<M>: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

#[test]
fn const_argument() {
    use tree::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(arrays::Arr([Some(Box::new(Leaf(None))), None])))));
    assert_eq!(leaf.count(), 5);
}

#[test]
fn bool_argument() {
    use others::{FlagLeaf, FlagNode};
    let leaf = FlagLeaf(Some(Box::new(FlagNode(arrays::Flag(Some(Box::new(FlagLeaf(None))))))));
    assert_eq!(leaf.count(), 4);
}

#[test]
fn literal_in_impl() {
    use others::{FixedLeaf, FixedNode};
    let leaf = FixedLeaf(Some(Box::new(FixedNode(arrays::Fixed(Some(Box::new(FixedLeaf(None))))))));
    assert_eq!(leaf.count(), 6);
}

#[test]
fn generic_argument() {
    use others::{GenericLeaf, GenericNode};
    let leaf = GenericLeaf::<1>(Some(Box::new(GenericNode(arrays::Arr([Some(Box::new(GenericLeaf(None)))])))));
    assert_eq!(leaf.count(), 4);
}
