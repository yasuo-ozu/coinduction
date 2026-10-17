// `marker` was removed: the impls of announced types are announced as written.
use coinduction::coinduction;

pub struct M;

#[coinduction(marker = super::M)]
mod m {}

fn main() {}
