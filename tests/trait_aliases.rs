//! Traits imported under another name, by the module of an announced type or by the module using
//! it, and in the rules of a trait.
#![allow(dead_code)]
use coinduction::*;

pub mod traits {
    #[announce::announce]
    pub trait Count {
        fn count(&self) -> usize;
    }

    #[announce::announce(meta = coinduction::rules(
        (($T:ty, $U:ty)) => { $T: Size, $U: Size },
        (u8) => {}
    ))]
    pub trait Size {
        fn size(&self) -> usize;
    }

    impl<T: Size, U: Size> Size for (T, U) {
        fn size(&self) -> usize {
            self.0.size() + self.1.size()
        }
    }

    impl Size for u8 {
        fn size(&self) -> usize {
            1
        }
    }

    // The trampoline is renamed: it is imported as `{Weight, WeightMacro as Weight}`, which is not
    // an alias of the trait
    #[announce::announce(macro_name = WeightMacro)]
    pub trait Weight {
        fn weight(&self) -> usize;
    }
}

#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::traits::Count;

    #[announce::announce]
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

// The module of the type imports the trait under another name
#[coinduction]
pub mod aliased_wrappers {
    #[coinduction]
    use super::traits::Count as Counted;

    #[announce::announce]
    pub struct Boxed<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Counted for Boxed<T>
    where
        T: Counted,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 2
        }
    }
}

#[coinduction]
pub mod weights {
    #[coinduction]
    use super::traits::{Weight, WeightMacro as Weight};

    #[announce::announce]
    pub struct Boxed<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Weight for Boxed<T>
    where
        T: Weight,
    {
        fn weight(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.weight()) + 1
        }
    }
}

#[coinduction]
pub mod weighted {
    use super::weights::Boxed;
    #[coinduction]
    use super::traits::{Weight, WeightMacro as Weight};

    pub struct Node(pub Boxed<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Weight for Node
    where
        Boxed<Leaf>: Weight,
    {
        fn weight(&self) -> usize {
            self.0.weight() + 1
        }
    }

    impl Weight for Leaf
    where
        Node: Weight,
    {
        fn weight(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.weight()) + 1
        }
    }
}

// The using module imports the trait under another name
#[coinduction]
pub mod aliased_user {
    use super::wrappers::Boxed;
    #[coinduction]
    use super::traits::Count as Counter;

    pub struct Node(pub Boxed<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Counter for Node
    where
        Boxed<Leaf>: Counter,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Counter for Leaf
    where
        Node: Counter,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

// Both modules use other names
#[coinduction]
pub mod both_aliased {
    use super::aliased_wrappers::Boxed;
    #[coinduction]
    use super::traits::Count as Counter;

    pub struct Node(pub Boxed<Leaf>);
    pub struct Leaf(pub Option<Box<Node>>);

    impl Counter for Node
    where
        Boxed<Leaf>: Counter,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Counter for Leaf
    where
        Node: Counter,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |n| n.count()) + 1
        }
    }
}

// The rules of a trait, used under another name and by its path
#[coinduction]
pub mod rules {
    #[coinduction]
    use super::traits::{Size as Sized2, Size};

    pub struct List(pub Option<Box<(u8, List)>>);
    pub struct Other(pub Option<Box<(u8, Other)>>);

    impl Sized2 for List
    where
        (u8, List): Sized2,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size())
        }
    }

    impl super::traits::Size for Other
    where
        (u8, Other): super::traits::Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size())
        }
    }
}

use traits::{Count, Size, Weight};

#[test]
fn aliased_user() {
    use aliased_user::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(wrappers::Boxed(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.count(), 4);
}

#[test]
fn both_aliased() {
    use both_aliased::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(aliased_wrappers::Boxed(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.count(), 5);
}

#[test]
fn rules() {
    let list = rules::List(Some(Box::new((1, rules::List(None)))));
    assert_eq!(list.size(), 1);
    let other = rules::Other(Some(Box::new((1, rules::Other(None)))));
    assert_eq!(other.size(), 1);
}

#[test]
fn renamed_trampoline() {
    use weighted::{Leaf, Node};
    let leaf = Leaf(Some(Box::new(Node(weights::Boxed(Some(Box::new(Leaf(None))))))));
    assert_eq!(leaf.weight(), 4);
}
