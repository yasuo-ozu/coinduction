//! The `coinduction = <path>` module argument: the crate reached through another path.

mod reexport {
    pub use ::coinduction::*;
}

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[::coinduction::coinduction(coinduction = crate::reexport)]
pub mod defs {
    #[coinduction]
    use super::Count;

    #[announce::announce]
    pub struct W<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for W<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }
}

#[::coinduction::coinduction(coinduction = crate::reexport)]
pub mod user {
    use super::defs::W;
    #[coinduction]
    use super::Count;

    pub struct N(pub W<N>);

    // N -> W<N> -> N, through `crate::reexport::__next_step`
    impl Count for N
    where
        W<N>: Count,
    {
        fn count(&self) -> usize {
            self.0.count() + 1
        }
    }
}

#[test]
fn test_coinduction_path() {
    use defs::W;
    use user::N;

    let n = N(W(Some(Box::new(N(W(None))))));
    assert_eq!(n.count(), 4);
}
