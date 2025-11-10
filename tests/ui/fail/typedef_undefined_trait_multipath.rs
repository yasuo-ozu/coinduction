use coinduction::*;

// Regular trait without #[traitdef] - this should cause error when explicitly specified
trait UndefinedTrait {
    fn undefined_method(&self);
}

struct TestStruct;

// This should fail because UndefinedTrait is not defined with #[traitdef]
// but is explicitly specified as an argument with multi-segment path
#[typedef(super::UndefinedTrait)]
mod explicit_trait_module {
    impl super::UndefinedTrait for super::TestStruct {
        fn undefined_method(&self) {}
    }
}

fn main() {}