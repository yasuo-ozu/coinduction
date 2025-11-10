use coinduction::*;

// This should fail: #[traitdef] cannot be applied to structs
#[traitdef]
struct NotATrait {
    field: i32,
}

fn main() {}