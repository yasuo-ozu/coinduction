// A trait in a module is announced, not marked with `#[coinduction]`.
use coinduction::coinduction;

#[coinduction]
mod m {
    #[coinduction]
    pub trait Count {
        fn count(&self) -> usize;
    }
}

fn main() {}
