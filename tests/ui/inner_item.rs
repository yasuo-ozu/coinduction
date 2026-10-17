// In a module, `#[coinduction]` marks only `use` items.
use coinduction::coinduction;

#[coinduction]
mod m {
    #[coinduction]
    fn f() {}
}

fn main() {}
