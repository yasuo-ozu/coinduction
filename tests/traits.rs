//! Which traits a `#[coinduction]` module works on.

use coinduction::coinduction;
use traits::Size;

// A trait with rules, announced in a module
#[coinduction]
pub mod traits {
    #[announce::announce(meta = coinduction::rules(
        (($T:ty, $U:ty)) => { $T: Size, $U: Size },
        (u8) => {}
    ))]
    pub trait Size {
        fn size(&self) -> usize;
    }
}

impl Size for u8 {
    fn size(&self) -> usize {
        1
    }
}

impl<T: Size, U: Size> Size for (T, U) {
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
}

#[coinduction]
pub mod user {
    #[coinduction]
    use super::traits::Size;

    pub struct List(pub Option<Box<(u8, List)>>);

    // List -> (u8, List) -> List, through the rules of a trait defined in another module
    impl Size for List
    where
        (u8, List): Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.size()) + 1
        }
    }
}

// No trait is marked: every trait implemented in the module is used
#[coinduction]
pub mod unmarked {
    use super::traits::Size;

    pub struct A(pub Option<Box<B>>);
    pub struct B(pub Option<Box<A>>);

    impl Size for A
    where
        B: Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |b| b.size()) + 1
        }
    }

    impl Size for B
    where
        A: Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |a| a.size()) + 1
        }
    }
}

#[test]
fn test_trait_in_module() {
    use user::List;

    let list = List(Some(Box::new((1, List(None)))));
    assert_eq!(list.size(), 3);
}

#[test]
fn test_unmarked_traits() {
    use unmarked::{A, B};

    assert_eq!(A(Some(Box::new(B(None)))).size(), 2);
}
