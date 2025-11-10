use coinduction::*;

// This should pass: generic traitdef with associated types
#[allow(unused)]
#[traitdef]
trait GenericPassingTrait<T> {
    type Output;
    fn process(&self, input: T) -> Self::Output;
}

struct GenericImpl;

impl GenericPassingTrait<i32> for GenericImpl {
    type Output = String;
    
    fn process(&self, input: i32) -> Self::Output {
        format!("Processed: {}", input)
    }
}

fn main() {
    let processor = GenericImpl;
    let result = processor.process(42);
    assert_eq!(result, "Processed: 42");
}