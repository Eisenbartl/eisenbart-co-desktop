//! Explicit handoff of an unchanged exported PDF to the Windows association.
//! This never invokes a command interpreter, prints, or creates temporary files.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MAX_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) const EXPIRED: &str = "The saved-PDF opening request is unavailable or expired. Export a new PDF, or open the existing export manually in your PDF viewer.";
pub(crate) struct Pending {
    pub token: String,
    pub client_id: String,
    pub assessment_id: String,
    pub receipt: String,
    pub path: PathBuf,
    pub digest: [u8; 32],
    pub created: Instant,
}
impl Pending {
    pub fn expired(&self) -> bool {
        self.created.elapsed() >= Duration::from_secs(10 * 60)
    }
    pub fn verified_file(&self) -> Result<File, String> {
        let changed = "The exported PDF is missing, changed or unavailable. Nothing was opened. Export a new PDF or inspect the existing file manually.";
        // Keep the file open through dispatch. On Windows deny concurrent writes
        // and deletion during verification/dispatch; the external viewer owns
        // its subsequent reads and may retain readable data after workspace lock.
        let mut options = File::options();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
        }
        if std::fs::symlink_metadata(&self.path)
            .map_err(|_| changed)?
            .file_type()
            .is_symlink()
        {
            return Err(changed.into());
        }
        let mut file = options.open(&self.path).map_err(|_| changed)?;
        let meta = file.metadata().map_err(|_| changed)?;
        if !meta.is_file() || meta.len() > MAX_BYTES {
            return Err(changed.into());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| changed)?;
        if bytes.len() as u64 > MAX_BYTES
            || !bytes.starts_with(b"%PDF-")
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != self.digest
        {
            return Err(changed.into());
        }
        Ok(file)
    }
}
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Requested,
    #[cfg_attr(not(windows), allow(dead_code))] // Returned by the Windows adapter.
    Cancelled,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Handoff {
    pub status: Status,
    pub path: String,
}

// Test the same UTF-16 preparation on both hosts; Windows paths are deliberately
// handled as text here so Linux tests do not reinterpret their components.
#[cfg(any(windows, test))]
fn windows_pdf_path(value: &[u16]) -> Result<Vec<u16>, String> {
    let prefix: Vec<u16> = "\\\\?\\".encode_utf16().collect();
    let value = value.strip_prefix(prefix.as_slice()).unwrap_or(value);
    let slash = |v| v == b'\\' as u16 || v == b'/' as u16;
    if value.len() < 7
        || value.contains(&0)
        || !((b'A' as u16..=b'Z' as u16).contains(&value[0])
            || (b'a' as u16..=b'z' as u16).contains(&value[0]))
        || value[1] != b':' as u16
        || !slash(value[2])
        || value[3..].contains(&(b':' as u16))
        || !String::from_utf16_lossy(&value[value.len() - 4..]).eq_ignore_ascii_case(".pdf")
    {
        return Err("Open only an exported PDF on a Windows drive. Network, device and URL paths cannot be opened by this action; use your PDF viewer manually.".into());
    }
    Ok(value.iter().copied().chain(std::iter::once(0)).collect())
}
#[cfg(any(windows, test))]
fn launch_failure(code: u32) -> Result<Status, String> {
    match code {
        1223 => Ok(Status::Cancelled),
        1155 => Err("Windows has no PDF viewer associated with .pdf files. Choose a PDF viewer in Windows Settings → Apps → Default apps, then try again. Your exported PDF is unchanged.".into()),
        2 | 3 => Err("Windows could not find the exported PDF or its viewer. Check the file and your PDF viewer. Your export was not changed.".into()),
        5 | 32 => Err("Windows denied access to the PDF or viewer, or the file is in use. Check access and try again. Your export was not changed.".into()),
        _ => Err("Windows did not accept the PDF opening request. Check your PDF viewer and try again, or open the saved file manually. No printing was confirmed.".into()),
    }
}
#[cfg(not(windows))]
pub(crate) fn open_pdf(_path: &Path) -> Result<Status, String> {
    Err("Opening a saved PDF from this app is supported on Windows only. Open the exported PDF manually in a local viewer and choose Print.".into())
}
#[cfg(windows)]
pub(crate) fn open_pdf(path: &Path) -> Result<Status, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::GetLastError,
        System::Com::{
            CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        },
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    let file = windows_pdf_path(&path.as_os_str().encode_wide().collect::<Vec<_>>())?;
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    // SAFETY: COM is initialized/uninitialized on this blocking thread. All
    // pointers reference live NUL-terminated buffers for the synchronous call.
    // lpParameters stays null: no user string is a command or argument list.
    unsafe {
        if CoInitializeEx(
            std::ptr::null(),
            COINIT_APARTMENTTHREADED as u32 | COINIT_DISABLE_OLE1DDE as u32,
        ) < 0
        {
            return Err("Windows could not initialize the PDF opening request. Open the saved PDF manually, or try again.".into());
        }
        struct Com;
        impl Drop for Com {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _com = Com;
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            nShow: SW_SHOWNORMAL,
            ..Default::default()
        };
        if ShellExecuteExW(&mut info) != 0 {
            Ok(Status::Requested)
        } else {
            launch_failure(GetLastError())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifier_bounds_files_rejects_replacements_and_keeps_export_after_drop() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("Fictional private.pdf");
        let bytes = b"%PDF-Synthetic verification fixture";
        std::fs::write(&path, bytes).unwrap();
        let p = Pending {
            token: String::new(),
            client_id: String::new(),
            assessment_id: String::new(),
            receipt: String::new(),
            path: path.clone(),
            digest: Sha256::digest(bytes).into(),
            created: Instant::now(),
        };
        let verified = p.verified_file().unwrap();
        #[cfg(windows)]
        assert!(File::options().write(true).open(&path).is_err());
        drop(verified);
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert!(p.verified_file().is_err());
        std::fs::remove_file(&path).unwrap();
        #[cfg(unix)]
        {
            let target = root.path().join("other.pdf");
            std::fs::write(&target, bytes).unwrap();
            std::os::unix::fs::symlink(&target, &path).unwrap();
            assert!(p.verified_file().is_err());
            std::fs::remove_file(&path).unwrap();
        }
        std::fs::write(&path, bytes).unwrap();
        drop(p);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    #[test]
    fn windows_path_is_one_literal_unicode_pdf_without_shell_arguments() {
        for p in [
            "C:\\Private reports\\Jamie & Co; $(echo) résumé.PDF",
            "\\\\?\\C:\\Private reports\\Jamie.pdf",
            "D:/fictional/a'b.pdf",
        ] {
            let prepared = windows_pdf_path(&p.encode_utf16().collect::<Vec<_>>()).unwrap();
            assert_eq!(prepared.last(), Some(&0));
            assert_eq!(
                String::from_utf16(&prepared[..prepared.len() - 1]).unwrap(),
                p.strip_prefix("\\\\?\\").unwrap_or(p)
            );
        }
        for p in [
            "https://example.invalid/a.pdf",
            "\\\\server\\share\\a.pdf",
            "\\\\?\\UNC\\server\\a.pdf",
            "\\\\.\\a.pdf",
            "C:relative.pdf",
            "a.pdf",
            "C:\\a.pdf:stream.pdf",
            "C:\\a.exe",
            "C:\\a.pdf\0.exe",
        ] {
            assert!(
                windows_pdf_path(&p.encode_utf16().collect::<Vec<_>>()).is_err(),
                "{p}"
            );
        }
    }
    #[test]
    fn cancellation_association_and_other_failures_never_claim_print_success() {
        assert_eq!(launch_failure(1223).unwrap(), Status::Cancelled);
        assert!(launch_failure(1155).unwrap_err().contains("no PDF viewer"));
        assert!(launch_failure(5).unwrap_err().contains("denied access"));
        assert!(launch_failure(2).unwrap_err().contains("could not find"));
        assert!(launch_failure(29)
            .unwrap_err()
            .contains("No printing was confirmed"));
    }
}
