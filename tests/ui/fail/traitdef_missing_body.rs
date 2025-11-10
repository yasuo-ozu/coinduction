use coinduction::*;

// This should fail: #[traitdef] requires a trait body
#[traitdef]
trait MissingBody;

fn main() {}