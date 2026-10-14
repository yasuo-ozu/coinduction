use coinduction::coinduction;

#[announce::announce]
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
