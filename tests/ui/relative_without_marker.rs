// Announcing a type whose impls use relative paths needs `marker`.
use coinduction::coinduction;

#[announce::announce]
pub trait Conv<T> {
    fn conv(&self);
}

pub struct Local;

#[coinduction]
mod m {
    #[coinduction]
    use super::Conv;
    use super::Local;

    #[announce::announce]
    pub struct S;

    impl Conv<Local> for S {
        fn conv(&self) {}
    }
}

fn main() {}
