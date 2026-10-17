//! The impls that `#[coinduction]` modules export for each kind of announced type, announce
//! options, and the rules of a trait, split over several `META` entries.

use coinduction::coinduction;

#[announce::announce(
    meta = coinduction::rules((u8) => {}),
    meta = coinduction::rules((($T:ty, $U:ty)) => { $T: Count, $U: Count })
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

    // An enum, with options to which the module appends its own (trailing comma included)
    #[announce::announce(allow_std_prelude = true,)]
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

    // A union
    #[announce::announce]
    pub union U<T: ::core::marker::Copy> {
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

    // A renamed trampoline
    #[announce::announce(macro_name = DMacro)]
    pub struct D<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for D<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }

    // Generic over a lifetime
    #[announce::announce]
    pub struct R<'a, T>(pub ::core::option::Option<&'a T>);

    impl<'a, T> Count for R<'a, T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.map_or(0, |t| t.count()) + 1
        }
    }

    // Announced, but its impl of `Count` is outside the module, so it exports none
    #[announce::announce]
    pub struct Plain;
}

impl Count for defs::Plain {
    fn count(&self) -> usize {
        5
    }
}

#[coinduction]
pub mod user {
    use super::defs::{DMacro as D, Plain, D, E, R, U};
    #[coinduction]
    use super::Count;

    pub struct ViaEnum(pub E<ViaEnum>);
    pub struct ViaUnion(pub U<u8>, pub Option<Box<ViaUnion>>);
    pub struct ViaRenamed(pub D<ViaRenamed>);
    pub struct ViaRules(pub Option<Box<(u8, ViaRules)>>);
    pub struct WithPlain;
    pub struct ViaRef(pub R<'static, ViaRef>);

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

    // ViaRenamed -> D<ViaRenamed> -> ViaRenamed
    impl Count for ViaRenamed
    where
        D<ViaRenamed>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    // ViaRules -> (u8, ViaRules) -> ViaRules, with rules from two META entries
    impl Count for ViaRules
    where
        (u8, ViaRules): Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.count()) + 1
        }
    }

    // ViaRef -> R<'static, ViaRef> -> ViaRef
    impl Count for ViaRef
    where
        R<'static, ViaRef>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    // `Plain` exports no impls of `Count`: the bound is kept as it is
    impl Count for WithPlain
    where
        Plain: Count,
    {
        fn count(&self) -> usize {
            Plain.count()
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
fn test_derive_macro_name() {
    use defs::D;
    use user::ViaRenamed;

    let v = ViaRenamed(D(Some(Box::new(ViaRenamed(D(None))))));
    assert_eq!(v.count(), 4);
}

#[test]
fn test_rules() {
    use user::ViaRules;

    let v = ViaRules(Some(Box::new((1, ViaRules(None)))));
    assert_eq!(v.count(), 3);
}

#[test]
fn test_lifetime_argument() {
    use defs::R;
    use user::ViaRef;

    static LEAF: ViaRef = ViaRef(R(None));
    let v = ViaRef(R(Some(&LEAF)));
    assert_eq!(v.count(), 4);
}

#[test]
fn test_type_without_impls() {
    assert_eq!(user::WithPlain.count(), 5);
}
