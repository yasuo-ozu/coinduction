// `#[coinduction]` applies only to modules.
use coinduction::coinduction;

#[coinduction]
pub trait Count {
    fn count(&self) -> usize;
}

fn main() {}
