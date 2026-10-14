// The impls of a type are known only when it is announced in a `#[coinduction]` module.
use coinduction::coinduction;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[announce::announce]
pub struct Outside;

#[coinduction]
mod m {
    #[coinduction]
    use super::Count;

    pub struct S;

    impl Count for S
    where
        super::Outside: Count,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

fn main() {}
