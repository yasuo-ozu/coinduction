use coinduction::*;

#[allow(unused)]
#[traitdef]
trait CoinductiveTrait {
    fn coinductive_method(&self);
}

// This should pass: basic coinduction usage resolving circular dependencies
#[coinduction(super::CoinductiveTrait)]
mod passing_coinduction {
    pub struct TypeA;
    pub struct TypeB;

    // Circular dependencies that are resolved by coinduction
    impl super::CoinductiveTrait for TypeA
    where
        TypeB: super::CoinductiveTrait,
    {
        fn coinductive_method(&self) {}
    }

    impl super::CoinductiveTrait for TypeB
    where
        TypeA: super::CoinductiveTrait,
    {
        fn coinductive_method(&self) {}
    }
}

fn main() {
    let a = passing_coinduction::TypeA;
    let b = passing_coinduction::TypeB;
    a.coinductive_method();
    b.coinductive_method();
}