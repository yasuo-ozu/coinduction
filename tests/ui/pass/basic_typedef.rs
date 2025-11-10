use coinduction::*;

#[allow(unused)]
#[traitdef]
trait PassingTrait {
    fn method(&self);
}

// This should pass: basic typedef usage
#[typedef(super::PassingTrait)]
mod passing_types {
    pub struct PassingType;
    
    impl super::PassingTrait for PassingType {
        fn method(&self) {}
    }
}

fn main() {
    let test = passing_types::PassingType;
    test.method();
}