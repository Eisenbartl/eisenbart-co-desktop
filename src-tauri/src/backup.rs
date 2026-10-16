//! Authenticated containers of SQLCipher snapshots and copy-before-switch recovery.
use super::*;
use fs2::FileExt;
use hmac::{Hmac, Mac};
use sha2::Sha256;

const MAGIC: &[u8; 8] = b"EBCBACK1";
const MAX_DATABASE: u64 = 256 * 1024 * 1024;
const POINTER: &str = "workspace-active.json";
const VERIFY_ERROR: &str = "The backup could not be verified. The password may be incorrect, or the file may be damaged or unsupported. Your current workspace has not been changed.";
const RECOVERY_ERROR: &str = "Recovery could not finish. Your previous workspace files have been preserved. Review the backup and try again.";
type Authentication = Hmac<Sha256>;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Header {
    version: u32,
    metadata: Metadata,
    created_at: String,
    backup_id: String,
    schema_version: i32,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    version: u32,
    generation: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub path: String,
    pub created_at: String,
    pub audit_recorded: bool,
    pub durability_warning: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReview {
    pub token: String,
    pub organization_name: String,
    pub counselor_name: String,
    pub client_count: u32,
    pub created_at: String,
    pub source_schema: i32,
    pub current_workspace_present: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub previous_workspace_preserved: bool,
    pub durability_warning: bool,
}

pub(super) struct PrivateDirectory {
    pub(super) path: PathBuf,
    pub(super) keep: bool,
}
impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
pub(super) struct Candidate {
    directory: PrivateDirectory,
    connection: Option<Connection>,
    review: RestoreReview,
    parent_pointer: Option<Vec<u8>>,
    parent_files: Vec<Option<(u64, Option<std::time::SystemTime>)>>,
    source_directory: PathBuf,
    backup_id: String,
    created: Instant,
}
impl Candidate {
    pub(super) fn expired(&self) -> bool {
        self.created.elapsed() >= AUTO_LOCK_AFTER
    }
}
impl Drop for Candidate {
    fn drop(&mut self) {
        self.connection.take();
    }
}

fn private_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_dir() => Ok(()),
        Ok(_) => Err(RECOVERY_ERROR.into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => create_private_directory(path),
        Err(_) => Err(RECOVERY_ERROR.into()),
    }
}
pub(super) fn staging(root: &Path) -> Result<PrivateDirectory, String> {
    private_directory(root)?;
    let parent = root.join("recovery-generations");
    private_directory(&parent)?;
    let path = parent.join(Uuid::new_v4().to_string());
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&path)
        .map_err(|_| RECOVERY_ERROR.to_owned())?;
    Ok(PrivateDirectory { path, keep: false })
}
fn file_stamps(directory: &Path) -> Vec<Option<(u64, Option<std::time::SystemTime>)>> {
    [
        DATABASE_NAME,
        METADATA_NAME,
        "workspace.db-journal",
        "workspace.db-wal",
        "workspace.db-shm",
    ]
    .iter()
    .map(|name| {
        fs::symlink_metadata(directory.join(name))
            .ok()
            .map(|m| (m.len(), m.modified().ok()))
    })
    .collect()
}
fn pointer_bytes(root: &Path) -> Result<Option<Vec<u8>>, String> {
    let path = root.join(POINTER);
    if !regular_file_exists(&path)? {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| RECOVERY_ERROR.to_owned())?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| RECOVERY_ERROR.to_owned())?;
    if bytes.len() > 4096 {
        return Err(RECOVERY_ERROR.into());
    }
    Ok(Some(bytes))
}
pub(super) fn resolve_directory(root: &Path) -> Result<PathBuf, String> {
    let Some(bytes) = pointer_bytes(root)? else {
        return Ok(root.to_path_buf());
    };
    let p: Pointer = serde_json::from_slice(&bytes).map_err(|_| RECOVERY_ERROR.to_owned())?;
    if p.version != 1
        || Uuid::parse_str(&p.generation)
            .map(|id| id.to_string() != p.generation)
            .unwrap_or(true)
    {
        return Err(RECOVERY_ERROR.into());
    }
    let parent = root.join("recovery-generations");
    if !fs::symlink_metadata(&parent)
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
    {
        return Err(RECOVERY_ERROR.into());
    }
    let directory = parent.join(p.generation);
    if !fs::symlink_metadata(&directory)
        .map(|m| m.file_type().is_dir())
        .unwrap_or(false)
    {
        return Err(RECOVERY_ERROR.into());
    }
    Ok(directory)
}
pub(super) fn activation_lock(root: &Path) -> Result<File, String> {
    private_directory(root)?;
    let path = root.join("workspace-operation.lock");
    let file = match create_private_file(&path) {
        Ok(file) => file,
        Err(_) if regular_file_exists(&path)? => OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| RECOVERY_ERROR.to_owned())?,
        Err(_) => return Err(RECOVERY_ERROR.into()),
    };
    file.try_lock_exclusive().map_err(|_|"Another backup or recovery operation is running. Close other application instances and try again.".to_owned())?;
    Ok(file)
}
pub(super) fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|f| f.sync_all())
            .map_err(|_| RECOVERY_ERROR.to_owned())?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
fn authentication(key: &[u8; 32]) -> Authentication {
    let mut derive = Authentication::new_from_slice(key).expect("fixed HMAC key");
    derive.update(b"Eisenbart Counseling Suite backup authentication key v1");
    let derived = Zeroizing::new(<[u8; 32]>::from(derive.finalize().into_bytes()));
    Authentication::new_from_slice(&*derived).expect("fixed HMAC key")
}
pub(super) fn info(connection: &Connection) -> Result<WorkspaceInfo, String> {
    let result=connection.query_row("SELECT o.id,o.name,c.id,c.name FROM workspace_settings w JOIN organizations o ON o.id=w.organization_id JOIN counselors c ON c.id=w.counselor_id AND c.organization_id=o.id WHERE w.singleton=1",[],|r|Ok(WorkspaceInfo{organization_id:r.get(0)?,organization_name:r.get(1)?,counselor_id:r.get(2)?,counselor_name:r.get(3)?})).map_err(|_|VERIFY_ERROR.to_owned())?;
    required_text(&result.organization_name, "Organization name", 120)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    required_text(&result.counselor_name, "Counselor name", 120)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    Ok(result)
}
pub(super) fn verify_database(connection: &Connection) -> Result<(), String> {
    let id: i32 = connection
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    if id != APPLICATION_ID {
        return Err(VERIFY_ERROR.into());
    }
    integrity_check(connection).map_err(|_| VERIFY_ERROR.to_owned())?;
    if connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|_| VERIFY_ERROR.to_owned())?
        .query([])
        .map_err(|_| VERIFY_ERROR.to_owned())?
        .next()
        .map_err(|_| VERIFY_ERROR.to_owned())?
        .is_some()
    {
        return Err(VERIFY_ERROR.into());
    }
    Ok(())
}
pub(super) fn export_snapshot(
    source: &Connection,
    path: &Path,
    key: &[u8; 32],
) -> Result<(), String> {
    drop(create_private_file(path)?);
    let mut encoded = Zeroizing::new(String::from("x'"));
    for b in key {
        write!(&mut *encoded, "{b:02x}").map_err(|_| STORAGE_ERROR.to_owned())?;
    }
    encoded.push('\'');
    source
        .execute(
            "ATTACH DATABASE ?1 AS snapshot KEY ?2",
            params![path.to_str().ok_or(STORAGE_ERROR)?, encoded.as_str()],
        )
        .map_err(storage_error)?;
    let result: Result<(), String> = (|| {
        source.execute_batch("BEGIN;").map_err(storage_error)?;
        let version: i32 = source
            .query_row("PRAGMA main.user_version", [], |r| r.get(0))
            .map_err(storage_error)?;
        source
            .query_row("SELECT sqlcipher_export('snapshot')", [], |_| Ok(()))
            .map_err(storage_error)?;
        source.execute_batch(&format!("PRAGMA snapshot.user_version={version}; PRAGMA snapshot.application_id={APPLICATION_ID}; COMMIT;")).map_err(storage_error)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = source.execute_batch("ROLLBACK;");
    }
    let detached = source
        .execute_batch("DETACH DATABASE snapshot;")
        .map_err(storage_error);
    result?;
    detached?;
    let verified = open_encrypted(path, key)?;
    verify_database(&verified)?;
    drop(verified);
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| STORAGE_ERROR.to_owned())?;
    Ok(())
}
fn write_container(
    path: &Path,
    header: &Header,
    database: &Path,
    key: &[u8; 32],
) -> Result<(), String> {
    let manifest = serde_json::to_vec(header).map_err(|_| STORAGE_ERROR.to_owned())?;
    let length = fs::metadata(database)
        .map_err(|_| STORAGE_ERROR.to_owned())?
        .len();
    if manifest.len() > 4096 || !(4096..=MAX_DATABASE).contains(&length) {
        return Err(
            "This workspace exceeds the supported 256 MiB backup size, or cannot be backed up."
                .into(),
        );
    }
    // Exclusive creation prevents replacement on local and removable filesystems.
    let mut output = create_private_file(path)?;
    let result = (|| {
        let mut mac = authentication(key);
        let mut write = |bytes: &[u8]| -> Result<(), String> {
            mac.update(bytes);
            output
                .write_all(bytes)
                .map_err(|_| STORAGE_ERROR.to_owned())
        };
        write(MAGIC)?;
        write(&(manifest.len() as u32).to_le_bytes())?;
        write(&length.to_le_bytes())?;
        write(&manifest)?;
        let mut source = File::open(database).map_err(|_| STORAGE_ERROR.to_owned())?;
        let mut buffer = [0_u8; 65536];
        let mut copied = 0_u64;
        loop {
            let count = source
                .read(&mut buffer)
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            if count == 0 {
                break;
            }
            copied += count as u64;
            if copied > length {
                return Err(STORAGE_ERROR.into());
            }
            write(&buffer[..count])?;
        }
        if copied != length {
            return Err(STORAGE_ERROR.into());
        }
        output
            .write_all(&mac.finalize().into_bytes())
            .map_err(|_| STORAGE_ERROR.to_owned())?;
        output.sync_all().map_err(|_| STORAGE_ERROR.to_owned())?;
        Ok(())
    })();
    drop(output);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
fn read_container(
    path: &Path,
    directory: &Path,
    password: &str,
) -> Result<(Header, Connection), String> {
    if !regular_file_exists(path).map_err(|_| VERIFY_ERROR.to_owned())? {
        return Err(VERIFY_ERROR.into());
    }
    let mut source = File::open(path).map_err(|_| VERIFY_ERROR.to_owned())?;
    let mut prefix = [0_u8; 20];
    source
        .read_exact(&mut prefix)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    if &prefix[..8] != MAGIC {
        return Err(VERIFY_ERROR.into());
    }
    let manifest_len = u32::from_le_bytes(prefix[8..12].try_into().expect("fixed prefix")) as usize;
    let database_len = u64::from_le_bytes(prefix[12..20].try_into().expect("fixed prefix"));
    if manifest_len > 4096
        || !(4096..=MAX_DATABASE).contains(&database_len)
        || source
            .metadata()
            .map_err(|_| VERIFY_ERROR.to_owned())?
            .len()
            != 20 + manifest_len as u64 + database_len + 32
    {
        return Err(VERIFY_ERROR.into());
    }
    let mut manifest = vec![0_u8; manifest_len];
    source
        .read_exact(&mut manifest)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    let header: Header = serde_json::from_slice(&manifest).map_err(|_| VERIFY_ERROR.to_owned())?;
    if header.version != 1
        || header.metadata.version != METADATA_VERSION
        || !(1..=SCHEMA_VERSION).contains(&header.schema_version)
        || Uuid::parse_str(&header.backup_id).is_err()
        || chrono::DateTime::parse_from_rfc3339(&header.created_at).is_err()
    {
        return Err(VERIFY_ERROR.into());
    }
    let key = derive_key(password, &header.metadata.salt)?;
    let mut mac = authentication(&key);
    mac.update(&prefix);
    mac.update(&manifest);
    let mut output = create_private_file(&directory.join(DATABASE_NAME))?;
    let mut remaining = database_len;
    let mut buffer = [0_u8; 65536];
    while remaining > 0 {
        let count = (remaining.min(buffer.len() as u64)) as usize;
        source
            .read_exact(&mut buffer[..count])
            .map_err(|_| VERIFY_ERROR.to_owned())?;
        mac.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .map_err(|_| RECOVERY_ERROR.to_owned())?;
        remaining -= count as u64;
    }
    let mut tag = [0_u8; 32];
    source
        .read_exact(&mut tag)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    mac.verify_slice(&tag)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    output.sync_all().map_err(|_| RECOVERY_ERROR.to_owned())?;
    drop(output);
    let mut metadata = create_private_file(&directory.join(METADATA_NAME))?;
    serde_json::to_writer(&mut metadata, &header.metadata)
        .map_err(|_| RECOVERY_ERROR.to_owned())?;
    metadata.sync_all().map_err(|_| RECOVERY_ERROR.to_owned())?;
    let connection = open_encrypted(&directory.join(DATABASE_NAME), &key)
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    let schema: i32 = connection
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| VERIFY_ERROR.to_owned())?;
    if schema != header.schema_version {
        return Err(VERIFY_ERROR.into());
    }
    verify_database(&connection)?;
    Ok((header, connection))
}
#[cfg(unix)]
fn replace_pointer(from: &Path, to: &Path) -> Result<(), String> {
    fs::rename(from, to).map_err(|_| RECOVERY_ERROR.to_owned())
}
#[cfg(windows)]
fn replace_pointer(from: &Path, to: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(RECOVERY_ERROR.into());
    }
    Ok(())
}

impl Workspace {
    pub fn create_backup(
        &mut self,
        directory: String,
        password: String,
    ) -> Result<BackupResult, String> {
        self.session()?;
        let _activation = self.activation_lock()?;
        let password = Zeroizing::new(password);
        validate_password(&password)?;
        let metadata = self.read_metadata()?;
        let key = derive_key(&password, &metadata.salt)?;
        let source =
            open_encrypted(&self.database_path(), &key).map_err(|_| VERIFY_ERROR.to_owned())?;
        verify_database(&source)?;
        let schema: i32 = source
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(storage_error)?;
        let clients: i64 = source
            .query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0))
            .map_err(storage_error)?;
        if schema != SCHEMA_VERSION || clients > 1000 {
            return Err("Reopen this workspace with the current app before backup; this milestone supports up to 1,000 clients.".into());
        }
        let source_info = info(&source)?;
        if source_info != self.session()?.info {
            return Err(VERIFY_ERROR.into());
        }
        let target = fs::canonicalize(directory).map_err(|_| {
            "Choose an existing backup folder. Your workspace has not been changed.".to_owned()
        })?;
        let root = fs::canonicalize(&self.data_dir).map_err(|_| STORAGE_ERROR.to_owned())?;
        if !target.is_dir() || target.starts_with(root) {
            return Err("Choose a backup folder outside the application's data folder, preferably on a separate drive.".into());
        }
        let stage = staging(&self.data_dir)?;
        let snapshot = stage.path.join(DATABASE_NAME);
        export_snapshot(&source, &snapshot, &key)?;
        drop(source);
        let id = Uuid::new_v4().to_string();
        let created_at = timestamp();
        let path = target.join(format!(
            "Eisenbart-{}-{id}.ebcbackup",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let header = Header {
            version: 1,
            metadata,
            created_at: created_at.clone(),
            backup_id: id.clone(),
            schema_version: SCHEMA_VERSION,
        };
        write_container(&path, &header, &snapshot, &key)?;
        let durability_warning = sync_directory(&target).is_err();
        // A durable valid backup is retained even if the later audit write fails.
        let audit_recorded = (|| {
            let session = self.session()?;
            let transaction = session.connection.transaction().map_err(storage_error)?;
            audit(&transaction, &session.info, "backup_created", Some(&id))?;
            transaction.commit().map_err(storage_error)
        })()
        .is_ok();
        Ok(BackupResult {
            path: path.to_string_lossy().into_owned(),
            created_at,
            audit_recorded,
            durability_warning,
        })
    }
    pub fn prepare_restore(
        &mut self,
        path: String,
        password: String,
    ) -> Result<RestoreReview, String> {
        self.expire_if_idle();
        if self.session.is_some() {
            return Err("Lock the workspace before reviewing a restore.".into());
        }
        self.pending_restore = None;
        let password = Zeroizing::new(password);
        validate_password(&password)?;
        let _activation = self.activation_lock()?;
        let parent_pointer = pointer_bytes(&self.data_dir)?;
        let source_directory =
            resolve_directory(&self.data_dir).unwrap_or_else(|_| self.data_dir.clone());
        let parent_files = file_stamps(&source_directory);
        let directory = staging(&self.data_dir)?;
        let (header, mut connection) =
            read_container(Path::new(&path), &directory.path, &password)?;
        migrate(&mut connection, false).map_err(|_| VERIFY_ERROR.to_owned())?;
        let workspace_info = info(&connection)?;
        let ids: Vec<String> = connection
            .prepare(
                "SELECT id FROM clients WHERE organization_id=?1 AND counselor_id=?2 LIMIT 1001",
            )
            .map_err(|_| VERIFY_ERROR.to_owned())?
            .query_map(
                params![workspace_info.organization_id, workspace_info.counselor_id],
                |r| r.get(0),
            )
            .map_err(|_| VERIFY_ERROR.to_owned())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| VERIFY_ERROR.to_owned())?;
        if ids.len() > 1000 {
            return Err("This backup exceeds the current 1,000-client recovery limit. Your current workspace has not been changed.".into());
        }
        security_policy::read(&connection, &workspace_info)?;
        branding_store::read(&connection, &workspace_info)?;
        for id in &ids {
            scoped_client(&connection, &workspace_info, id)?;
            client_history_store::threads(&connection, &workspace_info, id)?;
            client_care_store::validate_records(&connection, &workspace_info, id)?;
            read_budget(&connection, &workspace_info, id)?;
            read_debt_plan(&connection, &workspace_info, id)?;
            read_savings_plan(&connection, &workspace_info, id)?;
            read_cashflow_plan(&connection, &workspace_info, id)?;
            imports_store::read_imports(&connection, &workspace_info, id)?;
            cards_store::read(&connection, &workspace_info, id)?;
            card_review_store::history(&connection, &workspace_info, id)?;
            card_facts_store::versions(&connection, &workspace_info, id)?;
            coverage_review_store::validate_records(&connection, &workspace_info, id)?;
            document_collection_store::validate_records(&connection, &workspace_info, id)?;
            statement_accounts_store::read(&connection, &workspace_info, id)?;
            category_rules_store::read(&connection, &workspace_info, id)?;
            scenarios_store::validate_records(&connection, &workspace_info, id)?;
            assessments_store::validate_records(&connection, &workspace_info, id)?;
            final_reports_store::validate_records(&connection, &workspace_info, id)?;
            action_plans_store::validate_records(&connection, &workspace_info, id)?;
        }
        let review = RestoreReview {
            token: Uuid::new_v4().to_string(),
            organization_name: workspace_info.organization_name,
            counselor_name: workspace_info.counselor_name,
            client_count: ids.len() as u32,
            created_at: header.created_at,
            source_schema: header.schema_version,
            current_workspace_present: self.database_path().exists()
                || self.metadata_path().exists()
                || parent_pointer.is_some(),
        };
        self.pending_restore = Some(Candidate {
            directory,
            connection: Some(connection),
            review: review.clone(),
            parent_pointer,
            parent_files,
            source_directory,
            backup_id: header.backup_id,
            created: Instant::now(),
        });
        Ok(review)
    }
    pub fn cancel_restore(&mut self, token: &str) {
        if self
            .pending_restore
            .as_ref()
            .is_some_and(|p| p.review.token == token)
        {
            self.pending_restore = None;
        }
    }
    pub fn confirm_restore(
        &mut self,
        token: String,
        confirmation: String,
    ) -> Result<RestoreResult, String> {
        self.expire_if_idle();
        if self.session.is_some() {
            return Err("Lock the workspace before restoring.".into());
        }
        let mut candidate = self
            .pending_restore
            .take()
            .ok_or("Review a verified backup before restoring. The review may have expired.")?;
        if candidate.review.token != token || confirmation != "RESTORE" || candidate.expired() {
            return Err(
                "Restore confirmation did not match the verified review. Review the backup again."
                    .into(),
            );
        }
        let _activation = self.activation_lock()?;
        if pointer_bytes(&self.data_dir)? != candidate.parent_pointer
            || file_stamps(&candidate.source_directory) != candidate.parent_files
        {
            return Err("The active workspace changed during this review. Review the backup again before restoring.".into());
        }
        let connection = candidate
            .connection
            .as_mut()
            .expect("verified candidate connection");
        verify_database(connection)?;
        let workspace_info = info(connection)?;
        let transaction = connection.transaction().map_err(storage_error)?;
        audit(
            &transaction,
            &workspace_info,
            "workspace_restored",
            Some(&candidate.backup_id),
        )?;
        transaction.commit().map_err(storage_error)?;
        integrity_check(connection)?;
        candidate.connection.take();
        let durability_warning =
            activate_generation(&self.data_dir, &mut candidate.directory, &token)?;
        self.active_dir = candidate.directory.path.clone();
        Ok(RestoreResult {
            previous_workspace_preserved: candidate.review.current_workspace_present,
            durability_warning,
        })
    }
}

pub(super) fn activate_generation(
    root: &Path,
    directory: &mut PrivateDirectory,
    token: &str,
) -> Result<bool, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.path.join(DATABASE_NAME))
        .and_then(|f| f.sync_all())
        .map_err(|_| RECOVERY_ERROR.to_owned())?;
    sync_directory(&directory.path)?;
    sync_directory(directory.path.parent().expect("generation parent"))?;
    let temporary = root.join(format!(".workspace-active-{token}.tmp"));
    let p = Pointer {
        version: 1,
        generation: directory
            .path
            .file_name()
            .expect("generation name")
            .to_string_lossy()
            .into_owned(),
    };
    let mut owns_temporary = false;
    let result = (|| {
        let mut file = create_private_file(&temporary)?;
        owns_temporary = true;
        serde_json::to_writer(&mut file, &p).map_err(|_| RECOVERY_ERROR.to_owned())?;
        file.sync_all().map_err(|_| RECOVERY_ERROR.to_owned())?;
        drop(file);
        sync_directory(root)?;
        replace_pointer(&temporary, &root.join(POINTER))
    })();
    if let Err(error) = result {
        if owns_temporary {
            let _ = fs::remove_file(&temporary);
        }
        return Err(error);
    }
    directory.keep = true;
    Ok(sync_directory(root).is_err())
}

#[cfg(test)]
mod tests {
    use super::super::tests::{
        budget_input, cashflow_input, client_input, configured, debt_input, savings_input, PASSWORD,
    };
    use super::*;
    use tempfile::tempdir;
    fn all_records(w: &mut Workspace) -> Client {
        let client = w.save_client(client_input()).unwrap();
        w.add_client_note(&client.id, "Synthetic historical note")
            .unwrap();
        w.save_client_budget(&client.id, budget_input(0)).unwrap();
        w.save_client_debt_plan(&client.id, debt_input(0)).unwrap();
        w.save_client_savings_plan(&client.id, savings_input(0))
            .unwrap();
        w.save_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap();
        client
    }
    #[test]
    fn encrypted_backup_roundtrip_preserves_all_plans_audit_and_prior_files() {
        let (root, mut source, _) = configured();
        let target = tempdir().unwrap();
        let client = all_records(&mut source);
        let budget = source.get_client_budget(&client.id).unwrap();
        let savings = source.get_client_savings_plan(&client.id).unwrap();
        let cash = source.get_client_cashflow_plan(&client.id).unwrap();
        let notes = source.list_client_notes(&client.id).unwrap();
        let result = source
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        assert!(result.audit_recorded);
        let bytes = fs::read(&result.path).unwrap();
        assert!(bytes.starts_with(MAGIC));
        for plain in [
            "Synthetic historical note",
            client.first_name.as_str(),
            "Synthetic checking",
            PASSWORD,
        ] {
            assert!(!bytes.windows(plain.len()).any(|v| v == plain.as_bytes()));
        }
        let old_db = fs::read(source.database_path()).unwrap();
        let old_meta = fs::read(source.metadata_path()).unwrap();
        source.lock_workspace();
        let review = source
            .prepare_restore(result.path.clone(), PASSWORD.into())
            .unwrap();
        assert_eq!(review.client_count, 1);
        assert!(review.current_workspace_present);
        assert_eq!(fs::read(root.path().join(DATABASE_NAME)).unwrap(), old_db);
        source
            .confirm_restore(review.token, "RESTORE".into())
            .unwrap();
        assert!(source.session.is_none());
        assert_eq!(fs::read(root.path().join(DATABASE_NAME)).unwrap(), old_db);
        assert_eq!(fs::read(root.path().join(METADATA_NAME)).unwrap(), old_meta);
        let mut restarted = Workspace::new(root.path().to_path_buf());
        restarted.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(restarted.get_client(&client.id).unwrap(), client);
        assert_eq!(restarted.list_client_notes(&client.id).unwrap(), notes);
        assert_eq!(restarted.get_client_budget(&client.id).unwrap(), budget);
        assert_eq!(
            restarted.get_client_savings_plan(&client.id).unwrap(),
            savings
        );
        assert_eq!(
            restarted.get_client_cashflow_plan(&client.id).unwrap(),
            cash
        );
        assert_eq!(
            restarted.get_client_debt_plan(&client.id).unwrap().data,
            Some(crate::debt::calculate(debt_input(0).data).unwrap().0)
        );
        let c = &restarted.session.as_ref().unwrap().connection;
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event='workspace_restored'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert!(c.execute("DELETE FROM audit_events", []).is_err());
        assert!(c
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none());
        // Restoring on a new computer is independent of the original installation.
        let fresh = tempdir().unwrap();
        let mut new = Workspace::new(fresh.path().to_path_buf());
        let r = new.prepare_restore(result.path, PASSWORD.into()).unwrap();
        assert!(!r.current_workspace_present);
        new.confirm_restore(r.token, "RESTORE".into()).unwrap();
        new.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(new.get_client(&client.id).unwrap(), client);
    }
    #[test]
    fn incorrect_password_tampering_truncation_and_oversized_headers_preserve_source() {
        let (root, mut w, _) = configured();
        all_records(&mut w);
        let target = tempdir().unwrap();
        let b = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let original = fs::read(&b.path).unwrap();
        let db = fs::read(w.database_path()).unwrap();
        let metadata = fs::read(w.metadata_path()).unwrap();
        w.lock_workspace();
        assert!(w
            .prepare_restore(b.path.clone(), "Incorrect-fictional-password".into())
            .is_err());
        for position in [0, 8, 12, 22, original.len() / 2, original.len() - 1] {
            let path = target.path().join(format!("mutated-{position}.ebcbackup"));
            let mut changed = original.clone();
            changed[position] ^= 1;
            fs::write(&path, changed).unwrap();
            assert!(w
                .prepare_restore(path.to_string_lossy().into(), PASSWORD.into())
                .is_err());
            assert_eq!(fs::read(root.path().join(DATABASE_NAME)).unwrap(), db);
            assert_eq!(fs::read(root.path().join(METADATA_NAME)).unwrap(), metadata);
            assert!(w.pending_restore.is_none());
        }
        for bytes in [&original[..19], &original[..original.len() - 1]] {
            let path = target.path().join("truncated.ebcbackup");
            fs::write(&path, bytes).unwrap();
            assert!(w
                .prepare_restore(path.to_string_lossy().into(), PASSWORD.into())
                .is_err());
        }
        let mut header = original[..20].to_vec();
        header[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        let p = target.path().join("huge.ebcbackup");
        fs::write(&p, header).unwrap();
        assert!(w
            .prepare_restore(p.to_string_lossy().into(), PASSWORD.into())
            .is_err());
        assert_eq!(fs::read(w.database_path()).unwrap(), db);
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.list_clients().unwrap().len(), 1);
    }
    #[test]
    fn missing_and_corrupted_live_files_are_recovered_without_modifying_them() {
        let (root, mut w, _) = configured();
        let client = all_records(&mut w);
        let target = tempdir().unwrap();
        let b = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        fs::write(
            root.path().join(DATABASE_NAME),
            b"Synthetic corrupted database",
        )
        .unwrap();
        fs::remove_file(root.path().join(METADATA_NAME)).unwrap();
        assert!(w.setup_status().is_err());
        let r = w.prepare_restore(b.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        assert_eq!(
            fs::read(root.path().join(DATABASE_NAME)).unwrap(),
            b"Synthetic corrupted database"
        );
        assert!(!root.path().join(METADATA_NAME).exists());
        let mut restarted = Workspace::new(root.path().to_path_buf());
        restarted.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(restarted.get_client(&client.id).unwrap(), client);
    }
    #[test]
    fn cancel_expiry_wrong_confirmation_and_activation_failure_do_not_replace_data() {
        let (root, mut w, _) = configured();
        all_records(&mut w);
        let target = tempdir().unwrap();
        let b = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let db = fs::read(w.database_path()).unwrap();
        let r = w.prepare_restore(b.path.clone(), PASSWORD.into()).unwrap();
        let stage = w.pending_restore.as_ref().unwrap().directory.path.clone();
        w.cancel_restore("old-unrelated-token");
        assert!(stage.exists());
        w.cancel_restore(&r.token);
        assert!(!stage.exists());
        assert!(w.confirm_restore(r.token, "RESTORE".into()).is_err());
        let r = w.prepare_restore(b.path.clone(), PASSWORD.into()).unwrap();
        assert!(w.confirm_restore(r.token, "restore".into()).is_err());
        let r = w.prepare_restore(b.path.clone(), PASSWORD.into()).unwrap();
        w.pending_restore.as_mut().unwrap().created = Instant::now() - AUTO_LOCK_AFTER;
        assert!(w.confirm_restore(r.token, "RESTORE".into()).is_err());
        let r = w.prepare_restore(b.path.clone(), PASSWORD.into()).unwrap();
        // A temporary-file collision makes publication fail after candidate verification.
        let blocked = root
            .path()
            .join(format!(".workspace-active-{}.tmp", r.token));
        fs::write(&blocked, b"Synthetic existing internal file").unwrap();
        assert!(w.confirm_restore(r.token, "RESTORE".into()).is_err());
        assert_eq!(
            fs::read(&blocked).unwrap(),
            b"Synthetic existing internal file"
        );
        fs::remove_file(blocked).unwrap();
        assert_eq!(fs::read(w.database_path()).unwrap(), db);
        let r = w.prepare_restore(b.path, PASSWORD.into()).unwrap();
        let c = w
            .pending_restore
            .as_ref()
            .unwrap()
            .connection
            .as_ref()
            .unwrap();
        c.execute_batch("CREATE TRIGGER reject_restore_audit BEFORE INSERT ON audit_events WHEN NEW.event='workspace_restored' BEGIN SELECT RAISE(ABORT,'Synthetic disk failure'); END;").unwrap();
        assert!(w.confirm_restore(r.token, "RESTORE".into()).is_err());
        assert_eq!(fs::read(w.database_path()).unwrap(), db);
        w.unlock_workspace(PASSWORD.into()).unwrap();
    }
    #[test]
    fn backup_never_overwrites_and_keeps_valid_file_if_audit_fails() {
        let (_root, mut w, _) = configured();
        let target = tempdir().unwrap();
        all_records(&mut w);
        assert!(w
            .create_backup(
                target.path().to_string_lossy().into(),
                "Wrong-fictional-passphrase".into()
            )
            .is_err());
        assert_eq!(fs::read_dir(target.path()).unwrap().count(), 0);
        let first = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let original = fs::read(&first.path).unwrap();
        let meta = w.read_metadata().unwrap();
        let key = derive_key(PASSWORD, &meta.salt).unwrap();
        let header = Header {
            version: 1,
            metadata: meta,
            created_at: timestamp(),
            backup_id: Uuid::new_v4().to_string(),
            schema_version: SCHEMA_VERSION,
        };
        assert!(
            write_container(Path::new(&first.path), &header, &w.database_path(), &key).is_err()
        );
        assert_eq!(fs::read(&first.path).unwrap(), original);
        w.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER reject_backup_audit BEFORE INSERT ON audit_events WHEN NEW.event='backup_created' BEGIN SELECT RAISE(ABORT,'Synthetic disk failure'); END;").unwrap();
        let second = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        assert!(!second.audit_recorded);
        assert_ne!(first.path, second.path);
        w.lock_workspace();
        let r = w.prepare_restore(second.path, PASSWORD.into()).unwrap();
        w.cancel_restore(&r.token);
        assert!(w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .err()
            .unwrap()
            .contains("locked"));
    }
    #[test]
    fn concurrent_restore_or_live_edits_invalidate_review_and_old_sessions_lock() {
        let (root, mut first, _) = configured();
        let target = tempdir().unwrap();
        let client = all_records(&mut first);
        let backup = first
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let mut old = Workspace::new(root.path().to_path_buf());
        old.unlock_workspace(PASSWORD.into()).unwrap();
        first.lock_workspace();
        let r = first
            .prepare_restore(backup.path.clone(), PASSWORD.into())
            .unwrap();
        old.add_client_note(&client.id, "Synthetic edit during review")
            .unwrap();
        assert!(first
            .confirm_restore(r.token, "RESTORE".into())
            .err()
            .unwrap()
            .contains("changed"));
        let r = first
            .prepare_restore(backup.path.clone(), PASSWORD.into())
            .unwrap();
        let mut competing = Workspace::new(root.path().to_path_buf());
        let other = competing
            .prepare_restore(backup.path, PASSWORD.into())
            .unwrap();
        first.confirm_restore(r.token, "RESTORE".into()).unwrap();
        assert!(competing
            .confirm_restore(other.token, "RESTORE".into())
            .err()
            .unwrap()
            .contains("changed"));
        assert_eq!(old.get_client(&client.id).err().unwrap(), LOCKED);
        assert!(old.session.is_none());
        first.unlock_workspace(PASSWORD.into()).unwrap();
        // Exercise replacing an existing pointer, including the Windows MoveFileExW path.
        let second_backup = first
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        let previous = first.active_dir.clone();
        first.lock_workspace();
        let r = first
            .prepare_restore(second_backup.path, PASSWORD.into())
            .unwrap();
        first.confirm_restore(r.token, "RESTORE".into()).unwrap();
        assert!(previous.join(DATABASE_NAME).exists());
        first.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(first.get_client(&client.id).unwrap(), client);
    }
    #[test]
    fn unsupported_schema_and_authenticated_corrupt_database_are_rejected() {
        let (_root, mut w, _) = configured();
        let target = tempdir().unwrap();
        all_records(&mut w);
        let meta = w.read_metadata().unwrap();
        let key = derive_key(PASSWORD, &meta.salt).unwrap();
        let snapshot = target.path().join("snapshot.db");
        export_snapshot(&w.session.as_ref().unwrap().connection, &snapshot, &key).unwrap();
        let header = Header {
            version: 1,
            metadata: meta,
            created_at: timestamp(),
            backup_id: Uuid::new_v4().to_string(),
            schema_version: SCHEMA_VERSION + 1,
        };
        let future = target.path().join("future.ebcbackup");
        write_container(&future, &header, &snapshot, &key).unwrap();
        w.lock_workspace();
        assert!(w
            .prepare_restore(future.to_string_lossy().into(), PASSWORD.into())
            .is_err());
        let mut corrupted = fs::read(&snapshot).unwrap();
        corrupted[200] ^= 1;
        fs::write(&snapshot, corrupted).unwrap();
        let mut header = header;
        header.schema_version = SCHEMA_VERSION;
        let damaged = target.path().join("authenticated-damage.ebcbackup");
        write_container(&damaged, &header, &snapshot, &key).unwrap();
        assert!(w
            .prepare_restore(damaged.to_string_lossy().into(), PASSWORD.into())
            .is_err());
    }
    #[test]
    fn authenticated_container_with_corrupt_security_history_cannot_replace_workspace() {
        let (root, mut w, _) = configured();
        let target = tempdir().unwrap();
        w.save_security_policy(security_policy::Input {
            expected_revision: 0,
            idle_minutes: "5".into(),
        })
        .unwrap();
        let meta = w.read_metadata().unwrap();
        let key = derive_key(PASSWORD, &meta.salt).unwrap();
        let snapshot = target.path().join("security-corrupt.db");
        export_snapshot(&w.session.as_ref().unwrap().connection, &snapshot, &key).unwrap();
        let connection = open_encrypted(&snapshot, &key).unwrap();
        connection.execute_batch("DROP TRIGGER workspace_security_no_update; UPDATE workspace_security_history SET saved_at='invalid';").unwrap();
        drop(connection);
        let header = Header {
            version: 1,
            metadata: meta,
            created_at: timestamp(),
            backup_id: Uuid::new_v4().to_string(),
            schema_version: SCHEMA_VERSION,
        };
        let backup = target.path().join("security-corrupt.ebcbackup");
        write_container(&backup, &header, &snapshot, &key).unwrap();
        w.lock_workspace();
        let original = fs::read(root.path().join(DATABASE_NAME)).unwrap();
        assert!(w
            .prepare_restore(backup.display().to_string(), PASSWORD.into())
            .is_err());
        assert!(w.pending_restore.is_none());
        assert_eq!(fs::read(root.path().join(DATABASE_NAME)).unwrap(), original);
        assert!(!root.path().join(POINTER).exists());
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_security_policy().unwrap().idle_minutes, 5);
    }
    #[test]
    fn older_schema_snapshot_migrates_in_staging_and_failure_preserves_active_files() {
        let (root, mut w, _) = configured();
        let client = all_records(&mut w);
        let target = tempdir().unwrap();
        let connection = &w.session.as_ref().unwrap().connection;
        connection.execute_batch("DROP TABLE IF EXISTS workspace_security_history; DROP TABLE IF EXISTS note_amendments; DROP INDEX IF EXISTS note_amendment_scope; DROP TABLE IF EXISTS category_rule_applications; DROP TABLE IF EXISTS category_rule_versions; DROP INDEX IF EXISTS review_history_rule_scope; DROP TABLE IF EXISTS card_facts_history; DROP TABLE IF EXISTS card_review_history; DROP TABLE IF EXISTS card_decisions; DROP TABLE IF EXISTS card_statements; DROP TABLE IF EXISTS final_report_approvals; DROP TABLE IF EXISTS coverage_review_history; DROP TABLE IF EXISTS firm_branding_history; DROP TABLE IF EXISTS statement_account_history; DROP TRIGGER IF EXISTS statement_accounts_no_update; DROP TRIGGER IF EXISTS statement_accounts_no_delete; DROP TABLE IF EXISTS document_collection_snapshots; DROP TABLE IF EXISTS client_care_snapshots; DROP TABLE IF EXISTS assessment_scenario_links; DROP TABLE IF EXISTS client_scenarios; DROP TABLE IF EXISTS client_action_plans; DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices; DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state; CREATE TABLE audit_events_v6 (
            id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
            event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved')),
            entity_id TEXT,created_at TEXT NOT NULL,
            FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
            INSERT INTO audit_events_v6 SELECT * FROM audit_events; DROP TABLE audit_events; ALTER TABLE audit_events_v6 RENAME TO audit_events;
            CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
            CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Immutable'); END;
            CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Immutable'); END;
            PRAGMA user_version=6;").unwrap();
        let metadata = w.read_metadata().unwrap();
        let key = derive_key(PASSWORD, &metadata.salt).unwrap();
        let snapshot = target.path().join("old.db");
        export_snapshot(connection, &snapshot, &key).unwrap();
        let header = Header {
            version: 1,
            metadata,
            created_at: timestamp(),
            backup_id: Uuid::new_v4().to_string(),
            schema_version: 6,
        };
        let backup = target.path().join("old.ebcbackup");
        write_container(&backup, &header, &snapshot, &key).unwrap();
        w.lock_workspace();
        let original = fs::read(root.path().join(DATABASE_NAME)).unwrap();
        let r = w
            .prepare_restore(backup.to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        assert_eq!(r.source_schema, 6);
        assert_eq!(fs::read(root.path().join(DATABASE_NAME)).unwrap(), original);
        assert_eq!(
            w.pending_restore
                .as_ref()
                .unwrap()
                .connection
                .as_ref()
                .unwrap()
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(w.get_client(&client.id).unwrap(), client);
        // A migration name collision rejects the candidate, never altering the active source.
        let source = open_encrypted(&snapshot, &key).unwrap();
        source
            .execute_batch("CREATE TABLE audit_events_v7(collision TEXT);")
            .unwrap();
        drop(source);
        let broken = target.path().join("migration-failure.ebcbackup");
        write_container(&broken, &header, &snapshot, &key).unwrap();
        w.lock_workspace();
        let active = w.database_path();
        let bytes = fs::read(&active).unwrap();
        assert!(w
            .prepare_restore(broken.to_string_lossy().into(), PASSWORD.into())
            .is_err());
        assert_eq!(fs::read(active).unwrap(), bytes);
    }
    #[test]
    fn backup_includes_committed_wal_records_and_rejects_unsafe_destinations() {
        let (root, mut w, _) = configured();
        let target = tempdir().unwrap();
        let client = all_records(&mut w);
        w.session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .unwrap();
        w.add_client_note(&client.id, "Synthetic committed WAL note")
            .unwrap();
        assert!(w
            .create_backup(root.path().to_string_lossy().into(), PASSWORD.into())
            .is_err());
        assert!(w
            .create_backup(
                target.path().join("missing").to_string_lossy().into(),
                PASSWORD.into()
            )
            .is_err());
        let result = w
            .create_backup(target.path().to_string_lossy().into(), PASSWORD.into())
            .unwrap();
        w.lock_workspace();
        let r = w.prepare_restore(result.path, PASSWORD.into()).unwrap();
        w.confirm_restore(r.token, "RESTORE".into()).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
        assert!(w
            .list_client_notes(&client.id)
            .unwrap()
            .iter()
            .any(|n| n.body == "Synthetic committed WAL note"));
    }

    #[test]
    fn operation_lock_and_invalid_pointer_never_expose_or_replace_workspace() {
        let (root, mut w, _) = configured();
        let guard = activation_lock(root.path()).unwrap();
        assert!(activation_lock(root.path()).is_err());
        drop(guard);
        fs::write(
            root.path().join(POINTER),
            b"{\"version\":1,\"generation\":\"../outside\"}",
        )
        .unwrap();
        assert!(w.setup_status().is_err());
        assert_eq!(w.list_clients().err().unwrap(), LOCKED);
        assert!(resolve_directory(root.path()).is_err());
        fs::remove_file(root.path().join(POINTER)).unwrap();
        w.unlock_workspace(PASSWORD.into()).unwrap();
    }
}
