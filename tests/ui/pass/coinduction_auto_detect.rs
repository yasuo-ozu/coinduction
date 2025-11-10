use coinduction::*;

// This should pass because when no traits are explicitly specified,
// auto-detected traits are not validated
#[coinduction]
mod auto_detect_module {
    // Local trait defined with #[traitdef] - should work fine
    #[coinduction::traitdef]
    trait LocalValidTrait {
        fn valid_method(&self);
    }

    // Regular trait without #[traitdef] - should still work because it's auto-detected
    trait LocalUndefinedTrait {
        fn undefined_method(&self);
    }

    struct TestStruct;

    impl LocalValidTrait for TestStruct {
        fn valid_method(&self) {}
    }

    impl LocalUndefinedTrait for TestStruct {
        fn undefined_method(&self) {}
    }
}

fn main() {}