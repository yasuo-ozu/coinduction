use coinduction::*;

#[allow(unused)]
#[traitdef]
trait TraitA {
    fn method_a(&self);
}

#[allow(unused)]
#[traitdef]
trait TraitB {
    fn method_b(&self);
}

// This should pass: multiple traits in coinduction
#[coinduction(super::TraitA, super::TraitB)]
mod multi_trait_coinduction {
    pub struct ComplexType;

    impl super::TraitA for ComplexType {
        fn method_a(&self) {}
    }

    impl super::TraitB for ComplexType {
        fn method_b(&self) {}
    }
}

fn main() {
    let complex = multi_trait_coinduction::ComplexType;
    complex.method_a();
    complex.method_b();
}