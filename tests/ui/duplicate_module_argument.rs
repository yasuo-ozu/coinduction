// `coinduction = <path>` is given once.
use coinduction::coinduction;

#[coinduction(coinduction = ::coinduction, coinduction = ::coinduction)]
mod m {}

fn main() {}
