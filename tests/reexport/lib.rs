//! A crate without a dependency on announce: it uses announce through `coinduction::announce`. Its
//! types are used in a cycle of another crate in `tests/reexport.rs`.
#![allow(dead_code)]
use coinduction::coinduction;

// Outside a `#[coinduction]` module, the path to announce is given
#[coinduction::announce::announce(
    crate = ::coinduction::announce,
    meta = coinduction::rules((($T:ty, $U:ty)) => { $T: Count, $U: Count }, (u8) => {})
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

// In a `#[coinduction]` module, `crate = ::coinduction::announce` is added
#[coinduction]
pub mod wrappers {
    #[coinduction]
    use super::Count;
    use coinduction::announce as ann;
    use coinduction::announce::Announce as Derived;

    #[coinduction::announce::announce]
    pub struct ByPath<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for ByPath<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 1
        }
    }

    #[derive(Derived)]
    pub struct DeriveImported<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for DeriveImported<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 2
        }
    }

    #[ann::announce(macro_name = CrateAliasMacro)]
    pub struct CrateAlias<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

    impl<T> Count for CrateAlias<T>
    where
        T: Count,
    {
        fn count(&self) -> usize {
            self.0.as_ref().map_or(0, |t| t.count()) + 3
        }
    }
}
