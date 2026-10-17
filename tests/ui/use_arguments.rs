// `#[coinduction] use` takes no arguments.
use coinduction::coinduction;

#[announce::announce]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
mod m {
    #[coinduction(macro_name = X)]
    use super::Count;
}

fn main() {}
