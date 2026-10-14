// A trait that modules work on must be announced: its trampoline is called.
use coinduction::coinduction;

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
        u8: Count,
    {
        fn count(&self) -> usize {
            0
        }
    }
}

fn main() {}
