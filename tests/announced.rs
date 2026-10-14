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

// A type announced with `#[derive(Announce)]` instead of `#[announce]`.
#[coinduction]
pub mod derived {
    #[coinduction]
    use traitdef::Size;

    #[derive(announce::Announce)]
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
pub mod derived_user {
    use super::derived::Boxed;
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
fn test_derived() {
    use derived::Boxed;
    use derived_user::Node;

    let node = Node(Boxed(Some(Box::new(Node(Boxed(None))))));
    assert_eq!(node.size(), 2);
}
