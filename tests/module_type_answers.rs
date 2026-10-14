//! Cycles that leave the module more than once: the answer for a type announced by another module,
//! or of the rules of a trait, is a constraint on another type of the module, which is followed
//! through the impls of the module.

use coinduction::coinduction;

#[announce::announce(meta = coinduction::rules(
    (u8) => {},
    (($T:ty, $U:ty)) => { $T: Count, $U: Count }
))]
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

    #[announce::announce]
    pub struct Wrap<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for Wrap<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }
}

#[coinduction]
pub mod user {
    use super::defs::Wrap;
    #[coinduction]
    use super::Count;

    // A -> Wrap<B> -> B -> Wrap<A> -> A
    pub struct A(pub Wrap<B>);
    pub struct B(pub Wrap<A>);

    impl Count for A
    where
        Wrap<B>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }

    impl Count for B
    where
        Wrap<A>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 2
        }
    }

    // Generic: GenA<T> -> Wrap<GenB<T>> -> GenB<T> -> Wrap<GenA<T>> -> GenA<T>
    pub struct GenA<T>(pub Wrap<GenB<T>>, pub T);
    pub struct GenB<T>(pub Wrap<GenA<T>>, pub T);

    impl<T> Count for GenA<T>
    where
        T: Count,
        Wrap<GenB<T>>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + self.1.count()
        }
    }

    impl<T> Count for GenB<T>
    where
        T: Count,
        Wrap<GenA<T>>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + self.1.count()
        }
    }

    // Through the rules: RulesA -> (u8, RulesB) -> RulesB -> (u8, RulesA) -> RulesA
    pub struct RulesA(pub Option<Box<(u8, RulesB)>>);
    pub struct RulesB(pub Option<Box<(u8, RulesA)>>);

    impl Count for RulesA
    where
        (u8, RulesB): Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.count())
        }
    }

    impl Count for RulesB
    where
        (u8, RulesA): Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |pair| pair.count())
        }
    }
}

// A type that comes back grown once: P<u8> -> Wrap<P<Wrap<u8>>> -> P<Wrap<u8>> -> Wrap<P<u8>> -> P<u8>
#[coinduction]
pub mod grown_once {
    use super::defs::Wrap;
    #[coinduction]
    use super::Count;

    pub struct P<T>(pub T);

    impl Count for P<u8>
    where
        Wrap<P<Wrap<u8>>>: Count,
    {
        fn count(&self) -> usize {
            1
        }
    }

    impl Count for P<Wrap<u8>>
    where
        Wrap<P<u8>>: Count,
    {
        fn count(&self) -> usize {
            2
        }
    }
}

use defs::Wrap;
use user::*;

#[test]
fn test_through_type() {
    let a = A(Wrap(Some(Box::new(B(Wrap(None))))));
    assert_eq!(a.count(), 5);
}

#[test]
fn test_through_type_generic() {
    let a = GenA(Wrap(Some(Box::new(GenB(Wrap(None), 1u8)))), 1u8);
    assert_eq!(a.count(), 4);
}

#[test]
fn test_through_rules() {
    let a = RulesA(Some(Box::new((
        1,
        RulesB(Some(Box::new((1, RulesA(None))))),
    ))));
    assert_eq!(a.count(), 2);
}

#[test]
fn test_grown_once() {
    use grown_once::P;
    assert_eq!(P(1u8).count(), 1);
    assert_eq!(P(Wrap::<u8>(None)).count(), 2);
}
