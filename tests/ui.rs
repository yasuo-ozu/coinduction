// proc-macro-error reports `help` notes differently on nightly, so the expected output is that of
// stable.
#[rustversion::attr(not(stable), ignore)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("ui/*.rs");
}
