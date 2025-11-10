use coinduction::*;

// This should succeed in compilation because auto-detection mode
// doesn't validate traits - it only validates explicitly specified ones
#[typedef]
mod auto_detect_module {
    // Regular trait without #[traitdef] - but this is okay in auto-detection mode
    trait LocalUndefinedTrait {
        fn undefined_method(&self);
    }
    
    struct TestStruct;
    
    impl LocalUndefinedTrait for TestStruct {
        fn undefined_method(&self) {}
    }
}

fn main() {}