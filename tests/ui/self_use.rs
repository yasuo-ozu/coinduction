// `self` imports a module, not a trait.
use coinduction::coinduction;

#[coinduction]
mod m {
    #[coinduction]
    use std::fmt::{self};
}

fn main() {}
