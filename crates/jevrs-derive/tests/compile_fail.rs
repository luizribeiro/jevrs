#![allow(missing_docs)]

#[test]
fn options_errors_are_actionable() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/options/*.rs");
}

#[test]
fn levels_errors_are_actionable() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/levels/*.rs");
}
