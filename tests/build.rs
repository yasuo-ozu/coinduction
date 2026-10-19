fn main() {
    // Compiles the `#[subcrate]` modules of the tests
    subcrate::build::compile().unwrap();
}
