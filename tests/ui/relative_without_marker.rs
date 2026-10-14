// Exporting a type whose impls use relative paths needs `marker`.
use coinduction::coinduction;

#[coinduction]
pub trait Conv<T> {
    fn conv(&self);
}

pub struct Local;

#[coinduction]
mod m {
    #[coinduction]
    use super::Conv;
    use super::Local;

    #[coinduction(macro_name = SMacro)]
    pub struct S;

    impl Conv<Local> for S {
        fn conv(&self) {}
    }
}

fn main() {}
