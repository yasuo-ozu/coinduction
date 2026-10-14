// The traits of a glob import are unknown, so it cannot be marked.
use coinduction::coinduction;

#[coinduction]
mod m {
    #[coinduction]
    use std::fmt::*;
}

fn main() {}
