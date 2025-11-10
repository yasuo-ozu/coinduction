use coinduction::*;

#[coinduction::traitdef]
pub trait Display {
    fn display(&self) -> String;
}

#[coinduction]
mod circular {

    use super::Display;
    #[allow(unused)]
    pub struct TypeA;
    #[allow(unused)]
    pub struct TypeB;

    impl Display for TypeA
    where
        TypeB: Display, // This circular dependency is resolved by coinduction
    {
        fn display(&self) -> String {
            "TypeA".to_string()
        }
    }

    impl Display for TypeB
    where
        TypeA: Display, // This circular dependency is resolved by coinduction
    {
        fn display(&self) -> String {
            "TypeB".to_string()
        }
    }
}
