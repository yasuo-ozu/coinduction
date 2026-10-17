//! Traits and types announced with `announce`, as `#[coinduction]` modules see them.

use coinduction::coinduction;
use traitdef::Size;

// `(A, u8): Size` resolves through the rules of `Size` in another crate, back to `A: Size`.
#[coinduction]
pub mod rule_cycle {
    #[coinduction]
    use traitdef::Size;

    pub struct A(pub Option<Box<(A, u8)>>);

    impl traitdef::Countable for A {}

    impl Size for A
    where
        (A, u8): Size,
    {
        fn size(&self) -> usize {
            1 + self.0.as_ref().map_or(0, |pair| pair.size())
        }
    }
}

// A type announced in a `#[coinduction]` module.
#[coinduction]
pub mod wrapper {
    #[coinduction]
    use traitdef::Size;

    #[announce::announce]
    pub struct Boxed<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Size for Boxed<T>
    where
        T: Size,
    {
        fn size(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.size())
        }
    }
}

// `Boxed<Node>: Size` resolves through the impls of `Boxed` in the other module, back to
// `Node: Size`.
#[coinduction]
pub mod wrapper_user {
    use super::wrapper::Boxed;
    #[coinduction]
    use traitdef::Size;

    pub struct Node(pub Boxed<Node>);

    impl Size for Node
    where
        Boxed<Node>: Size,
    {
        fn size(&self) -> usize {
            1 + self.0.size()
        }
    }
}

#[test]
fn test_rule_cycle() {
    use rule_cycle::A;

    let a = A(Some(Box::new((A(None), 7))));
    // A + (A + u8)
    assert_eq!(a.size(), 3);
}

#[test]
fn test_announced_type() {
    use wrapper::Boxed;
    use wrapper_user::Node;

    let node = Node(Boxed(Some(Box::new(Node(Boxed(None))))));
    assert_eq!(node.size(), 2);
}
