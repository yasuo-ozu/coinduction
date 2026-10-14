// `#[coinduction(macro_name = ..)]` on a type is replaced by `#[announce(macro_name = ..)]`.
use coinduction::coinduction;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
mod m {
    #[coinduction]
    use super::Count;

    #[coinduction(macro_name = SMacro)]
    pub struct S;

    impl Count for S {
        fn count(&self) -> usize {
            0
        }
    }
}

fn main() {}
