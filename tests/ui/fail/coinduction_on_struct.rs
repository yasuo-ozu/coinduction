use coinduction::*;

#[allow(unused)]
#[traitdef]
trait TestTrait {
    fn test(&self);
}

// This should fail: #[coinduction] can only be applied to modules
#[coinduction(TestTrait)]
struct NotAModule {
    field: i32,
}

fn main() {}