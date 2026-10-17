//! The crates that the integration tests use across a crate boundary, compiled as subcrates:
//! `traitdef` announces traits (with rules), `typedef` announces types of a `#[coinduction]`
//! module.
use subcrate::subcrate;

#[subcrate(::announce)]
pub mod traitdef {
    use announce::announce;
    use std::fmt::Debug;
    use std::hash::Hash;

    // Local trait that exists only in this crate
    #[announce(meta = coinduction::rules(
        ([$T:ty; $N:expr]) => { $T: ::core::clone::Clone + ::core::default::Default },
        ([$T:ty]) => { $T: ::core::cmp::PartialEq + ::core::clone::Clone },
        (($T:ty, $U:ty)) => { $T: ::core::clone::Clone + ::core::marker::Send, $U: ::core::marker::Sync + ::core::default::Default },
        (($T:ty, $U:ty, $V:ty)) => { $T: ::core::clone::Clone + ::core::marker::Send, $U: ::core::marker::Sync, $V: ::core::default::Default + ::core::marker::Send }
    ))]
    pub trait LocalTrait {
        fn local_method(&self) -> usize;
    }

    #[announce(meta = coinduction::rules(
        (($T:ty, $U:ty)) => { $T: ::core::clone::Clone + ::core::marker::Send, $U: ::core::fmt::Debug + ::core::default::Default },
        (($T:ty, $U:ty, $V:ty)) => { $T: ::core::marker::Send + ::core::marker::Sync, $U: ::core::clone::Clone, $V: ::core::fmt::Debug + ::core::hash::Hash }
    ))]
    pub trait TestTrait {
        fn test_method(&self) -> ::std::string::String;
    }

    #[announce(meta = coinduction::rules(
        ([$T:ty; $N:expr]) => { $T: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync },
        (($T:ty, $U:ty)) => { $T: ::core::default::Default + ::core::clone::Clone, $U: ::core::marker::Send + ::core::marker::Sync }
    ))]
    pub trait CircularTrait {
        fn circular_method(&self) -> ::std::boxed::Box<dyn crate::CircularTrait>;
    }

    #[announce(meta = coinduction::rules(
        ([$T:ty]) => { $T: ::core::cmp::PartialEq + ::core::clone::Clone },
        (($T:ty, $U:ty, $V:ty)) => { $T: ::core::hash::Hash + ::core::cmp::Eq, $U: ::core::fmt::Debug + ::core::clone::Clone, $V: ::core::marker::Send + ::core::marker::Sync }
    ))]
    pub trait ExtendedTrait {
        fn extended_method(&self) -> bool;
    }

    // Implementations for arrays in LocalTrait
    impl<T, const N: usize> LocalTrait for [T; N]
    where
        T: Clone + Default,
    {
        fn local_method(&self) -> usize {
            let _ = T::default();
            self.len()
        }
    }

    // Implementations for slices in LocalTrait
    impl<T> LocalTrait for [T]
    where
        T: PartialEq + Clone,
    {
        fn local_method(&self) -> usize {
            if !self.is_empty() {
                let _ = self[0].clone();
            }
            self.len()
        }
    }

    // Implementations for tuples in LocalTrait
    impl<T, U> LocalTrait for (T, U)
    where
        T: Clone + Send,
        U: Sync + Default,
    {
        fn local_method(&self) -> usize {
            let _ = self.0.clone();
            let _ = U::default();
            2
        }
    }

    impl<T, U, V> LocalTrait for (T, U, V)
    where
        T: Clone + Send,
        U: Sync,
        V: Default + Send,
    {
        fn local_method(&self) -> usize {
            let _ = self.0.clone();
            let _ = V::default();
            3
        }
    }

    // Implementations for tuples in TestTrait
    impl<T, U> TestTrait for (T, U)
    where
        T: Clone + Send,
        U: Debug + Default,
    {
        fn test_method(&self) -> String {
            let _cloned = self.0.clone();
            let default_u = U::default();
            format!("tuple(2): {:?}", default_u)
        }
    }

    impl<T, U, V> TestTrait for (T, U, V)
    where
        T: Send + Sync,
        U: Clone,
        V: Debug + Hash,
    {
        fn test_method(&self) -> String {
            let _ = self.1.clone();
            format!("tuple(3): {:?}", self.2)
        }
    }

    // Implementations for arrays in CircularTrait
    impl<T, const N: usize> CircularTrait for [T; N]
    where
        T: Clone + Send + Sync + Default + 'static,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait> {
            Box::new((T::default(), 42u32))
        }
    }

    // Implementations for tuples in CircularTrait
    impl<T, U> CircularTrait for (T, U)
    where
        T: Default + Clone + Send + Sync + 'static,
        U: Send + Sync,
    {
        fn circular_method(&self) -> Box<dyn CircularTrait> {
            let default_t = T::default();
            Box::new((default_t.clone(), default_t))
        }
    }

    // Implementations for slices in ExtendedTrait
    impl<T> ExtendedTrait for [T]
    where
        T: PartialEq + Clone,
    {
        fn extended_method(&self) -> bool {
            if self.len() >= 2 {
                self[0] == self[1]
            } else {
                !self.is_empty()
            }
        }
    }

    // Implementations for tuples in ExtendedTrait
    impl<T, U, V> ExtendedTrait for (T, U, V)
    where
        T: Hash + Eq,
        U: Debug + Clone,
        V: Send + Sync,
    {
        fn extended_method(&self) -> bool {
            let _ = self.1.clone();
            use std::collections::hash_map::DefaultHasher;
            use std::hash::Hasher;
            let mut hasher = DefaultHasher::new();
            self.0.hash(&mut hasher);
            hasher.finish() % 2 == 0
        }
    }

    /// A plain trait, named with `crate::` in the rules of `Size`: other crates read it as
    /// `$crate::Countable`.
    pub trait Countable {}

    #[announce(meta = coinduction::rules(
        (u8) => {},
        (($T:ty, $U:ty)) => { $T: Size + crate::Countable, $U: Size }
    ))]
    pub trait Size {
        fn size(&self) -> usize;
    }

    impl Size for u8 {
        fn size(&self) -> usize {
            1
        }
    }

    impl<T, U> Size for (T, U)
    where
        T: Size + Countable,
        U: Size,
    {
        fn size(&self) -> usize {
            self.0.size() + self.1.size()
        }
    }
}

#[subcrate(::announce, ::coinduction, traitdef)]
pub mod typedef {
    use coinduction::coinduction;
    use traitdef::{CircularTrait, ExtendedTrait, LocalTrait, TestTrait};

    pub trait Never {}

    #[coinduction]
    pub mod generic_types {
        #[coinduction]
        use super::{CircularTrait, ExtendedTrait, LocalTrait, TestTrait};
        use announce::announce;
        use std::fmt::Debug;
        use std::hash::Hash;

        #[announce]
        pub struct Container<T, U> {
            pub first: T,
            pub second: U,
        }

        #[announce]
        pub struct Wrapper<T>
        where
            T: ::core::clone::Clone + ::core::fmt::Debug,
        {
            pub value: T,
            pub count: usize,
        }

        #[announce]
        pub struct MultiGeneric<T, U, V>
        where
            T: ::core::clone::Clone,
            U: ::core::marker::Send + ::core::marker::Sync,
            V: ::core::fmt::Debug + ::core::hash::Hash,
        {
            pub primary: T,
            pub secondary: U,
            pub metadata: V,
        }

        #[announce]
        pub struct ConstrainedStruct<T>
        where
            T: ::core::iter::Iterator + ::core::clone::Clone,
        {
            pub iterator: T,
        }

        impl<T, U> TestTrait for Container<T, U>
        where
            T: Clone + ::std::fmt::Debug + Send,
            U: ::std::fmt::Debug + Default + Sync,
        {
            fn test_method(&self) -> String {
                format!("{:?}", self.first)
            }
        }

        impl<T> TestTrait for Wrapper<T>
        where
            T: Clone + Debug + ToString,
        {
            fn test_method(&self) -> String {
                format!("{}: {}", self.value.to_string(), self.count)
            }
        }

        impl<T, U> LocalTrait for Container<T, U>
        where
            T: Clone + Send + Sync,
            U: ::std::fmt::Debug + Hash,
        {
            fn local_method(&self) -> usize {
                let _ = self.first.clone();
                42
            }
        }

        impl<T> LocalTrait for Wrapper<T>
        where
            T: Clone + ::std::fmt::Debug + Default,
        {
            fn local_method(&self) -> usize {
                let _ = T::default();
                self.count
            }
        }

        // A disabled impl: its predicates must not be announced (tests/cfg_impls.rs)
        #[announce]
        pub struct Chain<T>(pub ::core::option::Option<::std::boxed::Box<T>>);

        impl<T> LocalTrait for Chain<T>
        where
            T: LocalTrait,
        {
            fn local_method(&self) -> usize {
                self.0.as_ref().map_or(0, |t| t.local_method()) + 1
            }
        }

        #[cfg(any())]
        impl<T> LocalTrait for Chain<T>
        where
            T: LocalTrait + crate::Never,
        {
            fn local_method(&self) -> usize {
                0
            }
        }

        impl<T, U, V> CircularTrait for MultiGeneric<T, U, V>
        where
            T: Clone + ::std::fmt::Debug + Send + 'static,
            U: Send + Sync + Default,
            V: ::std::fmt::Debug + Hash + Clone,
        {
            fn circular_method(&self) -> Box<dyn CircularTrait> {
                Box::new(ConstrainedStruct {
                    iterator: std::iter::once(self.primary.clone()),
                })
            }
        }

        impl<T> CircularTrait for ConstrainedStruct<T>
        where
            T: Iterator + Clone + Send,
            T::Item: ::std::fmt::Debug,
        {
            fn circular_method(&self) -> Box<dyn CircularTrait> {
                Box::new(MultiGeneric {
                    primary: "circular".to_string(),
                    secondary: 42u32,
                    metadata: 123usize,
                })
            }
        }

        impl<T, U> ExtendedTrait for Container<T, U>
        where
            T: PartialEq + Clone,
            U: Default + Send,
        {
            fn extended_method(&self) -> bool {
                let _default_u = U::default();
                true
            }
        }

        impl<T, U, V> ExtendedTrait for MultiGeneric<T, U, V>
        where
            T: Clone + PartialOrd,
            U: Send + Sync + Clone,
            V: ::std::fmt::Debug + Hash + Default,
        {
            fn extended_method(&self) -> bool {
                let _ = V::default();
                let _ = self.secondary.clone();
                true
            }
        }
    }

    #[coinduction]
    pub mod local_types {
        #[coinduction]
        use super::LocalTrait;
        use announce::announce;

        #[announce(allow_std_prelude = true)]
        pub struct LocalType(pub String);

        impl LocalTrait for LocalType {
            fn local_method(&self) -> usize {
                self.0.len()
            }
        }
    }
}
