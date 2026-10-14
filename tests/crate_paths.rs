//! Traits named with `crate::` paths. announce sends them as `$crate::`, which must still match
//! the `crate::` paths of the module that reads them.

pub mod traits {
    #[announce::announce]
    pub trait Count {
        fn count(&self) -> usize;
    }
}

#[coinduction::coinduction]
pub mod defs {
    #[coinduction]
    use crate::traits::Count;

    #[announce::announce]
    pub struct W<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> crate::traits::Count for W<T>
    where
        T: crate::traits::Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }
}

pub mod deeper {
    #[coinduction::coinduction]
    pub mod user {
        use crate::defs::W;
        #[coinduction]
        use crate::traits::Count;

        pub struct N(pub W<N>);

        // N -> W<N> -> N
        impl crate::traits::Count for N
        where
            W<N>: crate::traits::Count,
        {
            fn count(&self) -> usize {
                self.0.count() + 1
            }
        }
    }
}

#[test]
fn test_crate_paths() {
    use crate::traits::Count;
    use deeper::user::N;
    use defs::W;

    let n = N(W(Some(Box::new(N(W(None))))));
    assert_eq!(n.count(), 4);
}
