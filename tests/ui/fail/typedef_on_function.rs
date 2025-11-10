use coinduction::*;

#[allow(unused)]
#[traitdef]
trait TestTrait {
    fn test(&self);
}

// This should fail: #[typedef] can only be applied to modules
#[typedef(TestTrait)]
fn not_a_module() {
    println!("I'm a function, not a module!");
}

fn main() {}