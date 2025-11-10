use coinduction::*;

// Regular trait without #[traitdef] - this should cause error when explicitly specified
trait UndefinedTrait {
    fn undefined_method(&self);
}

// This should fail because UndefinedTrait is not defined with #[traitdef]
// but is explicitly specified as an argument
#[coinduction(super::UndefinedTrait)]
mod explicit_trait_module {
    struct TestStruct;

    impl super::UndefinedTrait for TestStruct {
        fn undefined_method(&self) {}
    }
}

fn main() {}
