use coinduction::coinduction;

#[coinduction]
pub trait Count {
    fn count(&self) -> usize;
}

#[coinduction]
mod m {
    #[coinduction]
    #[coinduction]
    use super::Count;
}

fn main() {}
