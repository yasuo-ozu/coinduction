#[test]
fn ui_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/circular_without_coinduction.rs");
    t.compile_fail("tests/ui/fail/traitdef_missing_body.rs"); 
    t.compile_fail("tests/ui/fail/traitdef_on_struct.rs");
    t.compile_fail("tests/ui/fail/typedef_on_function.rs");
    t.compile_fail("tests/ui/fail/coinduction_on_struct.rs");
    // Tests for trait validation mechanism
    t.compile_fail("tests/ui/fail/coinduction_undefined_trait.rs");
    t.compile_fail("tests/ui/fail/typedef_undefined_trait.rs");
    t.compile_fail("tests/ui/fail/typedef_undefined_trait_multipath.rs");
    t.compile_fail("tests/ui/fail/version_mismatch_test.rs");
}

#[test]
fn ui_pass_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}