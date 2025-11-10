use coinduction::*;

#[allow(unused)]
#[traitdef]
trait CircularTrait {
    fn circular_method(&self);
}

// This should fail: circular dependencies without coinduction macro
mod circular_fail {
    pub struct TypeA;
    pub struct TypeB;

    // TypeA depends on TypeB implementing CircularTrait
    impl super::CircularTrait for TypeA
    where
        TypeB: super::CircularTrait,
    {
        fn circular_method(&self) {}
    }

    // TypeB depends on TypeA implementing CircularTrait (circular dependency)
    impl super::CircularTrait for TypeB
    where
        TypeA: super::CircularTrait,
    {
        fn circular_method(&self) {}
    }
}

fn main() {
    let a = circular_fail::TypeA;
    let b = circular_fail::TypeB;
    a.circular_method(); // Should fail due to unresolved circular dependency
    b.circular_method(); // Should fail due to unresolved circular dependency
}