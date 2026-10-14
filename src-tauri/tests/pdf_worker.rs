#[path = "../src/pdf_fixture.rs"]
mod fixture;
use eisenbart_counseling_suite_lib::pdf_process::extract_with;
use std::{path::Path, time::Duration};
fn exe() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_eisenbart-counseling-suite"))
}
#[test]
fn shipped_executable_extracts_in_supervised_process_without_workspace_initialization() {
    let pages = extract_with(exe(), &fixture::statement(), Duration::from_secs(15)).unwrap();
    assert_eq!(pages.len(), 2);
    assert!(pages[0].text.contains("Fictional payroll 1000.01"));
    assert!(pages[1].text.contains("Pending merchant"));
}
#[test]
fn protected_invalid_empty_and_deadline_expired_workers_are_rejected() {
    for bytes in [
        fixture::protected(),
        fixture::protected_with_password(""),
        fixture::bytes(fixture::document(&[&[]])),
        b"%PDF-1.4\ndamaged".to_vec(),
    ] {
        assert!(extract_with(exe(), &bytes, Duration::from_secs(15)).is_err());
    }
    assert!(extract_with(exe(), &fixture::statement(), Duration::ZERO).is_err());
}
