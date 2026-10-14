//! The impls that `#[coinduction]` modules export for each kind of type, and the rules of a trait.

use coinduction::coinduction;

#[coinduction(
    (u8) => {},
    (($T:ty, $U:ty)) => { $T: Count, $U: Count }
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

#[coinduction]
pub mod defs {
    #[coinduction]
    use super::Count;

    pub enum E<T> {
        Leaf,
        Node(Box<T>),
    }

    impl<T> Count for E<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            match self {
                E::Leaf => 1,
                E::Node(t) => t.count() + 1,
            }
        }
    }

    pub union U<T: Copy> {
        pub value: T,
    }

    impl<T: Copy> Count for U<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            unsafe { self.value.count() }
        }
    }
}

#[coinduction]
pub mod user {
    use super::defs::{E, U};
    #[coinduction]
    use super::Count;

    pub struct ViaEnum(pub E<ViaEnum>);
    pub struct ViaUnion(pub U<u8>, pub Option<Box<ViaUnion>>);
    pub struct ViaRules(pub Option<Box<(u8, ViaRules)>>);

    // ViaEnum -> E<ViaEnum> -> ViaEnum
    impl Count for ViaEnum
    where
        E<ViaEnum>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for ViaUnion
    where
        U<u8>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + self.1.as_ref().map_or(0, |u| u.count())
        }
    }

    // ViaRules -> (u8, ViaRules) -> ViaRules
    impl Count for ViaRules
    where
        (u8, ViaRules): Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.count()) + 1
        }
    }
}

#[test]
fn test_enum() {
    use defs::E;
    use user::ViaEnum;

    let v = ViaEnum(E::Node(Box::new(ViaEnum(E::Leaf))));
    assert_eq!(v.count(), 4);
}

#[test]
fn test_union() {
    use defs::U;
    use user::ViaUnion;

    let v = ViaUnion(U { value: 0 }, Some(Box::new(ViaUnion(U { value: 0 }, None))));
    assert_eq!(v.count(), 2);
}

#[test]
fn test_rules() {
    use user::ViaRules;

    let v = ViaRules(Some(Box::new((1, ViaRules(None)))));
    assert_eq!(v.count(), 3);
}
