// In a module, `#[coinduction]` applies only to `use` items, not to traits.
use coinduction::coinduction;

#[coinduction]
mod m {
    #[coinduction]
    pub trait Count {
        fn count(&self) -> usize;
    }
}

fn main() {}
