use coinduction::*;

// This should pass: basic traitdef usage
#[allow(unused)]
#[traitdef]
trait SimplePassingTrait {
    fn method(&self);
}

struct TestImpl;

impl SimplePassingTrait for TestImpl {
    fn method(&self) {}
}

fn main() {
    let test = TestImpl;
    test.method();
}