// A type of the module that comes back grown from another module's answers never closes a loop.
use coinduction::coinduction;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
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
            0
        }
    }
}

#[coinduction]
mod m {
    use super::defs::Wrap;
    #[coinduction]
    use super::Count;

    pub struct G<T>(pub T);

    // G<T> -> Wrap<G<Wrap<T>>> -> G<Wrap<T>> -> Wrap<G<Wrap<Wrap<T>>>> -> ...
    impl<T> Count for G<T>
    where
        Wrap<G<Wrap<T>>>: Count,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

fn main() {}
