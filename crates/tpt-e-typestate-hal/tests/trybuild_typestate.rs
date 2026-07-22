//! Compile-fail tests proving invalid typestate transitions are rejected.

#[test]
fn reject_start_without_configure() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail_cases/start_without_configure.rs");
}

#[test]
fn reject_wait_without_start() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail_cases/wait_without_start.rs");
}

#[test]
fn reject_configure_complete_channel() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail_cases/configure_complete.rs");
}

#[test]
fn reject_start_complete_channel() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail_cases/start_complete.rs");
}
