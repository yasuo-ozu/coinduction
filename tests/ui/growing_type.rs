// A type of the module that comes back grown each time never closes a loop.
use coinduction::coinduction;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
mod m {
    #[coinduction]
    use super::Count;

    pub struct G<T>(pub T);
    pub struct W<T>(pub T);

    // G<T> -> G<W<T>> -> G<W<W<T>>> -> ...
    impl<T> Count for G<T>
    where
        G<W<T>>: Count,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

fn main() {}
