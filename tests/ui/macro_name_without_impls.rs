// `macro_name` on a type for which no macro is generated.
use coinduction::coinduction;

#[coinduction]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
mod m {
    #[coinduction]
    use super::Count;

    #[coinduction(macro_name = SMacro)]
    pub struct S;
}

fn main() {}
