use coinduction::typedef;
use traitdef::{CircularTrait, ExtendedTrait, LocalTrait, TestTrait};

pub struct TypedefMarker;
pub struct LocalTypeMarker;

#[typedef(TestTrait, LocalTrait, CircularTrait, ExtendedTrait, marker = TypedefMarker)]
pub mod generic_types {
    use super::*;
    use std::fmt::Debug;
    use std::hash::Hash;

    pub struct Container<T, U> {
        pub first: T,
        pub second: U,
    }

    pub struct Wrapper<T>
    where
        T: Clone + Debug,
    {
        pub value: T,
        pub count: usize,
    }

    pub struct MultiGeneric<T, U, V>
    where
        T: Clone,
        U: Send + Sync,
        V: Debug + Hash,
    {
        pub primary: T,
        pub secondary: U,
        pub metadata: V,
    }

    pub struct ConstrainedStruct<T>
    where
        T: Iterator + Clone,
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

#[typedef(LocalTrait, marker = LocalTypeMarker)]
pub mod local_types {
    use super::*;

    pub struct LocalType(pub String);

    impl LocalTrait for LocalType {
        fn local_method(&self) -> usize {
            self.0.len()
        }
    }
}
