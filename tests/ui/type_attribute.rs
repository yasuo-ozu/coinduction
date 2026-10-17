// In a module, `#[coinduction]` applies only to `use` items, not to types.
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
