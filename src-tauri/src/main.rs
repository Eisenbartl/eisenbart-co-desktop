#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|v| v == "--private-pdf-worker-v1")
    {
        eisenbart_counseling_suite_lib::pdf_worker_entry();
        return;
    }
    eisenbart_counseling_suite_lib::run();
}
