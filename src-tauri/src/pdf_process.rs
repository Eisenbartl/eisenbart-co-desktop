//! Supervised PDF child process: no workspace initialization, key, or path input.
use crate::pdf::{self, Page, PDF_ERROR};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;
const MEMORY: usize = 512 * 1024 * 1024;
const MAX_OUTPUT: u64 = 2 * 1024 * 1024;

#[cfg(unix)]
fn configure(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // Dynamic native libraries are already mapped at startup. Limit heap/data
    // allocations rather than the address space needed by the desktop loader.
    unsafe {
        command.pre_exec(|| {
            for (resource, cap) in [
                (libc::RLIMIT_DATA, MEMORY as libc::rlim_t),
                (libc::RLIMIT_CPU, 10),
                (libc::RLIMIT_CORE, 0),
            ] {
                let limit = libc::rlimit {
                    rlim_cur: cap,
                    rlim_max: cap,
                };
                if libc::setrlimit(resource, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
}
#[cfg(windows)]
fn configure(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
}
#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
#[cfg(windows)]
fn constrain(child: &Child) -> Result<Job, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::*;
    unsafe {
        let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if handle.is_null() {
            return Err(PDF_ERROR.into());
        }
        let job = Job(handle);
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        limits.ProcessMemoryLimit = MEMORY;
        if SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32,
        ) == 0
            || AssignProcessToJobObject(handle, child.as_raw_handle() as _) == 0
        {
            return Err(PDF_ERROR.into());
        }
        Ok(job)
    }
}
#[cfg(unix)]
fn constrain(_: &Child) -> Result<(), String> {
    Ok(())
}

struct Guard(Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub fn extract(bytes: &[u8]) -> Result<Vec<Page>, String> {
    extract_with(
        &std::env::current_exe().map_err(|_| PDF_ERROR)?,
        bytes,
        Duration::from_secs(15),
    )
}
pub fn extract_with(
    exe: &std::path::Path,
    bytes: &[u8],
    timeout: Duration,
) -> Result<Vec<Page>, String> {
    if bytes.len() > crate::imports::MAX_BYTES || !bytes.starts_with(b"%PDF-") {
        return Err(PDF_ERROR.into());
    }
    let mut command = Command::new(exe);
    command
        .arg(pdf::WORKER_ARG)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Preserve loader/OS selectors only. No HOME, authentication, proxy, or
    // workspace variables are inherited by the parser process.
    for name in ["LD_LIBRARY_PATH", "SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    configure(&mut command);
    let mut child = Guard(command.spawn().map_err(|_| PDF_ERROR)?);
    #[cfg(windows)]
    let _constraint = constrain(&child.0)?;
    #[cfg(unix)]
    constrain(&child.0)?;
    let mut stdin = child.0.stdin.take().ok_or(PDF_ERROR)?;
    let stdout = child.0.stdout.take().ok_or(PDF_ERROR)?;
    let source = Zeroizing::new(bytes.to_vec());
    let writer = std::thread::spawn(move || stdin.write_all(&source));
    let reader = std::thread::spawn(move || {
        let mut bytes = Zeroizing::new(Vec::new());
        stdout.take(MAX_OUTPUT + 1).read_to_end(&mut bytes)?;
        Ok::<_, std::io::Error>(bytes)
    });
    let started = Instant::now();
    let successful = loop {
        match child.0.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.0.kill();
                let _ = child.0.wait();
                break false;
            }
        }
    };
    let written = writer.join().map_err(|_| PDF_ERROR)?.is_ok();
    let output = reader
        .join()
        .map_err(|_| PDF_ERROR)?
        .map_err(|_| PDF_ERROR)?;
    if !successful || !written || output.len() as u64 > MAX_OUTPUT {
        return Err(PDF_ERROR.into());
    }
    let pages: Vec<Page> = serde_json::from_slice(&output).map_err(|_| PDF_ERROR)?;
    pdf::validate_pages(&pages)?;
    Ok(pages)
}
