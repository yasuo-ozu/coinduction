use coinduction::*;

#[traitdef]
trait TestTrait {
    fn test(&self);
}

// This should trigger a version mismatch error
// by manually calling the version check with wrong version
fn test_version_mismatch() {
    TestTrait! { @version_check "0.2.0" };
}

fn main() {}