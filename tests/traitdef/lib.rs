use coinduction::traitdef;
use std::fmt::Debug;
use std::hash::Hash;

// Local trait that exists only in this crate
#[traitdef(
    ([$T:ty; $N:expr]) => { $T: ::core::clone::Clone + ::core::default::Default },
    ([$T:ty]) => { $T: ::core::cmp::PartialEq + ::core::clone::Clone },
    (($T:ty, $U:ty)) => { $T: ::core::clone::Clone + ::core::marker::Send, $U: ::core::marker::Sync + ::core::default::Default },
    (($T:ty, $U:ty, $V:ty)) => { $T: ::core::clone::Clone + ::core::marker::Send, $U: ::core::marker::Sync, $V: ::core::default::Default + ::core::marker::Send }
)]
pub trait LocalTrait {
    fn local_method(&self) -> usize;
}

#[traitdef(
    (($T:ty, $U:ty)) => { $T: ::core::clone::Clone + ::core::marker::Send, $U: ::core::fmt::Debug + ::core::default::Default },
    (($T:ty, $U:ty, $V:ty)) => { $T: ::core::marker::Send + ::core::marker::Sync, $U: ::core::clone::Clone, $V: ::core::fmt::Debug + ::core::hash::Hash }
)]
pub trait TestTrait {
    fn test_method(&self) -> String;
}

#[traitdef(
    ([$T:ty; $N:expr]) => { $T: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync },
    (($T:ty, $U:ty)) => { $T: ::core::default::Default + ::core::clone::Clone, $U: ::core::marker::Send + ::core::marker::Sync }
)]
pub trait CircularTrait {
    fn circular_method(&self) -> Box<dyn CircularTrait>;
}

#[traitdef(
    ([$T:ty]) => { $T: ::core::cmp::PartialEq + ::core::clone::Clone },
    (($T:ty, $U:ty, $V:ty)) => { $T: ::core::hash::Hash + ::core::cmp::Eq, $U: ::core::fmt::Debug + ::core::clone::Clone, $V: ::core::marker::Send + ::core::marker::Sync }
)]
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
