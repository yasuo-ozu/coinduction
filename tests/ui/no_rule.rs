// A tuple has no macro, so a rule of the trait must match it.
use coinduction::coinduction;

#[coinduction]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
mod m {
    #[coinduction]
    use super::Count;

    pub struct S;

    impl Count for S
    where
        (u8, u8): Count,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

fn main() {}
