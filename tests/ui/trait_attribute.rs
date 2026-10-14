// `#[coinduction]` is no longer applied to traits: they are announced.
use coinduction::coinduction;

#[coinduction]
pub trait Count {
    fn count(&self) -> usize;
}

fn main() {}
