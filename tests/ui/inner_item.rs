// In a module, `#[coinduction]` marks only `use`, `trait`, `struct`, `enum` and `union` items.
use coinduction::coinduction;

#[coinduction]
mod m {
    #[coinduction]
    fn f() {}
}

fn main() {}
