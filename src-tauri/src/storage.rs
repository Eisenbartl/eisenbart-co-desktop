//! The encrypted local workspace. All authorization decisions happen here,
//! using the authenticated organization and counselor, never IDs from the UI.
#[path = "assessments_store.rs"]
pub mod assessments_store;
#[path = "backup.rs"]
pub mod backup;
#[path = "imports_store.rs"]
pub mod imports_store;

use crate::budget::{self, BudgetInput, BudgetLine, ClientBudget};
use crate::cashflow::{self, ClientCashflowPlan};
use crate::debt::{self, ClientDebtPlan, PlanInput};
use crate::models::{
    Client, ClientInput, ClientNote, SetupStatus, SetupWorkspaceInput, WorkspaceInfo,
};
use crate::savings::{self, ClientSavingsPlan};
use argon2::{Algorithm, Argon2, Block, Params, Version};
use chrono::{SecondsFormat, Utc};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::Zeroizing;

const METADATA_VERSION: u32 = 1;
const SCHEMA_VERSION: i32 = 14;
const APPLICATION_ID: i32 = 0x45424346;
const DATABASE_NAME: &str = "workspace.db";
const METADATA_NAME: &str = "workspace.json";
pub const AUTO_LOCK_AFTER: Duration = Duration::from_secs(15 * 60);
const LOCKED: &str = "The workspace is locked. Unlock it to continue.";
const STORAGE_ERROR: &str =
    "The local workspace could not be read or saved. Your existing files have been preserved.";
const OPEN_ERROR: &str = "The password is incorrect, or the workspace cannot be opened.";
const UNAVAILABLE_CLIENT: &str = "The client is unavailable in this workspace.";

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    version: u32,
    salt: [u8; 32],
}

struct Session {
    connection: Connection,
    info: WorkspaceInfo,
    last_activity: Instant,
}

pub struct Workspace {
    data_dir: PathBuf,
    active_dir: PathBuf,
    session: Option<Session>,
    pending_restore: Option<backup::Candidate>,
    pending_csv: Option<imports_store::PendingCsv>,
}

impl Workspace {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            active_dir: data_dir.clone(),
            data_dir,
            session: None,
            pending_restore: None,
            pending_csv: None,
        }
    }

    /// Called by the application timer; it must not extend the session.
    pub fn expire_if_idle(&mut self) -> bool {
        if self.pending_csv.as_ref().is_some_and(|p| p.expired()) {
            self.pending_csv = None;
        }
        if self.pending_restore.as_ref().is_some_and(|p| p.expired()) {
            self.pending_restore = None;
        }
        let changed = self.session.is_some()
            && backup::resolve_directory(&self.data_dir)
                .map(|p| p != self.active_dir)
                .unwrap_or(true);
        let expired = changed
            || self
                .session
                .as_ref()
                .is_some_and(|session| session.last_activity.elapsed() >= AUTO_LOCK_AFTER);
        if expired {
            self.session = None;
            self.pending_csv = None;
        }
        expired
    }

    pub fn setup_status(&mut self) -> Result<SetupStatus, String> {
        self.expire_if_idle();
        self.active_dir = backup::resolve_directory(&self.data_dir)?;
        let database_exists = regular_file_exists(&self.database_path())?;
        let metadata_exists = regular_file_exists(&self.metadata_path())?;
        match (database_exists, metadata_exists) {
            (false, false) => {
                if self.active_dir!=self.data_dir { return Err("The restored workspace files are incomplete. Preserve them and restore a verified backup.".to_owned()); }
                // SQLite recovery files can be the last surviving part of an
                // interrupted workspace. Do not let fresh setup overwrite them.
                for suffix in ["-journal", "-wal", "-shm"] {
                    if regular_file_exists(&self.active_dir.join(format!("{DATABASE_NAME}{suffix}")))? {
                        return Err("The workspace setup is incomplete. Preserve the existing files and contact support before trying setup again.".to_owned());
                    }
                }
                Ok(SetupStatus { configured: false })
            }
            (true, true) => {
                self.read_metadata()?;
                Ok(SetupStatus { configured: true })
            }
            _ => Err("The workspace setup is incomplete. Preserve the existing files and contact support before trying setup again.".to_owned()),
        }
    }

    pub fn setup_workspace(&mut self, input: SetupWorkspaceInput) -> Result<WorkspaceInfo, String> {
        self.pending_restore = None;
        let _activation = backup::activation_lock(&self.data_dir)?;
        let password = Zeroizing::new(input.password);
        validate_password(&password)?;
        let organization_name = required_text(&input.organization_name, "Organization name", 120)?;
        let counselor_name = required_text(&input.counselor_name, "Counselor name", 120)?;
        if self.setup_status()?.configured {
            return Err("A workspace already exists. Unlock it to continue.".to_owned());
        }
        create_private_directory(&self.data_dir)?;
        let database_path = self.database_path();
        let metadata_path = self.metadata_path();

        // Claim the filename exclusively before SQLite opens it. No setup path
        // ever truncates or replaces an existing database or metadata file.
        drop(create_private_file(&database_path)?);
        let mut owns_metadata = false;
        let result = (|| {
            let mut salt = [0_u8; 32];
            OsRng
                .try_fill_bytes(&mut salt)
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            let metadata = Metadata {
                version: METADATA_VERSION,
                salt,
            };
            let key = derive_key(&password, &metadata.salt)?;
            let mut connection = open_encrypted(&database_path, &key)?;
            migrate(&mut connection, true)?;
            let info = WorkspaceInfo {
                organization_id: Uuid::new_v4().to_string(),
                organization_name,
                counselor_id: Uuid::new_v4().to_string(),
                counselor_name,
            };
            let transaction = connection.transaction().map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO organizations (id, name) VALUES (?1, ?2)",
                    params![info.organization_id, info.organization_name],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO counselors (id, organization_id, name) VALUES (?1, ?2, ?3)",
                    params![info.counselor_id, info.organization_id, info.counselor_name],
                )
                .map_err(storage_error)?;
            transaction.execute(
                "INSERT INTO workspace_settings (singleton, organization_id, counselor_id) VALUES (1, ?1, ?2)",
                params![info.organization_id, info.counselor_id],
            ).map_err(storage_error)?;
            audit(&transaction, &info, "workspace_created", None)?;
            transaction.commit().map_err(storage_error)?;
            integrity_check(&connection)?;
            let mut metadata_file = create_private_file(&metadata_path)?;
            owns_metadata = true;
            serde_json::to_writer(&mut metadata_file, &metadata)
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            metadata_file
                .flush()
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            metadata_file
                .sync_all()
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            Ok((connection, info))
        })();
        match result {
            Ok((connection, info)) => {
                self.session = Some(Session {
                    connection,
                    info: info.clone(),
                    last_activity: Instant::now(),
                });
                Ok(info)
            }
            Err(error) => {
                // Only files exclusively created by this attempt are removed.
                // Existing or partial workspaces are never silently repaired.
                let _ = fs::remove_file(database_path);
                if owns_metadata {
                    let _ = fs::remove_file(metadata_path);
                }
                Err(error)
            }
        }
    }

    pub fn unlock_workspace(&mut self, password: String) -> Result<WorkspaceInfo, String> {
        self.lock_workspace();
        let password = Zeroizing::new(password);
        validate_password(&password)?;
        if !self.setup_status()?.configured {
            return Err("Create a workspace before unlocking it.".to_owned());
        }
        let metadata = self.read_metadata()?;
        let key = derive_key(&password, &metadata.salt)?;
        let mut connection =
            open_encrypted(&self.database_path(), &key).map_err(|_| OPEN_ERROR.to_owned())?;
        // The key is verified with an actual encrypted-page read, before any
        // migration or metadata change is allowed.
        let app_id: i32 = connection
            .query_row("PRAGMA application_id", [], |row| row.get(0))
            .map_err(|_| OPEN_ERROR.to_owned())?;
        if app_id != APPLICATION_ID {
            return Err(OPEN_ERROR.to_owned());
        }
        integrity_check(&connection).map_err(|_| OPEN_ERROR.to_owned())?;
        migrate(&mut connection, false)?;
        let info = connection
            .query_row(
                "SELECT o.id, o.name, c.id, c.name FROM workspace_settings w
             JOIN organizations o ON o.id = w.organization_id
             JOIN counselors c ON c.id = w.counselor_id AND c.organization_id = o.id
             WHERE w.singleton = 1",
                [],
                |row| {
                    Ok(WorkspaceInfo {
                        organization_id: row.get(0)?,
                        organization_name: row.get(1)?,
                        counselor_id: row.get(2)?,
                        counselor_name: row.get(3)?,
                    })
                },
            )
            .map_err(|_| OPEN_ERROR.to_owned())?;
        self.session = Some(Session {
            connection,
            info: info.clone(),
            last_activity: Instant::now(),
        });
        Ok(info)
    }

    pub fn lock_workspace(&mut self) {
        // Dropping the only connection releases SQLCipher's in-memory key.
        self.session = None;
        self.pending_restore = None;
        self.pending_csv = None;
    }

    pub fn workspace_info(&mut self) -> Result<WorkspaceInfo, String> {
        Ok(self.session()?.info.clone())
    }

    pub fn list_clients(&mut self) -> Result<Vec<Client>, String> {
        let session = self.session()?;
        let mut statement = session
            .connection
            .prepare(
                "SELECT id, organization_id, first_name, last_name, email, phone,
                    household_size, status, goals, created_at, updated_at
             FROM clients WHERE organization_id = ?1 AND counselor_id = ?2
             ORDER BY last_name COLLATE NOCASE, first_name COLLATE NOCASE, id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![session.info.organization_id, session.info.counselor_id],
                client_from_row,
            )
            .map_err(storage_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage_error)
    }

    pub fn get_client(&mut self, id: &str) -> Result<Client, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, id)
    }

    pub fn save_client(&mut self, input: ClientInput) -> Result<Client, String> {
        // Authenticate before validation so locked commands never inspect or
        // modify the database, including invalid requests.
        let session = self.session()?;
        let input = validate_client(input)?;
        let now = timestamp();
        let (id, created_at, event) = if let Some(ref id) = input.id {
            let current = scoped_client(&session.connection, &session.info, id)?;
            (current.id, current.created_at, "client_updated")
        } else {
            (Uuid::new_v4().to_string(), now.clone(), "client_created")
        };
        let transaction = session.connection.transaction().map_err(storage_error)?;
        if input.id.is_some() {
            let changed = transaction
                .execute(
                    "UPDATE clients SET first_name = ?1, last_name = ?2, email = ?3, phone = ?4,
                     household_size = ?5, status = ?6, goals = ?7, updated_at = ?8
                 WHERE id = ?9 AND organization_id = ?10 AND counselor_id = ?11",
                    params![
                        input.first_name,
                        input.last_name,
                        input.email,
                        input.phone,
                        input.household_size,
                        input.status,
                        input.goals,
                        now,
                        id,
                        session.info.organization_id,
                        session.info.counselor_id
                    ],
                )
                .map_err(storage_error)?;
            if changed != 1 {
                return Err(UNAVAILABLE_CLIENT.to_owned());
            }
        } else {
            transaction
                .execute(
                    "INSERT INTO clients (id, organization_id, counselor_id, first_name, last_name,
                    email, phone, household_size, status, goals, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        id,
                        session.info.organization_id,
                        session.info.counselor_id,
                        input.first_name,
                        input.last_name,
                        input.email,
                        input.phone,
                        input.household_size,
                        input.status,
                        input.goals,
                        created_at,
                        now
                    ],
                )
                .map_err(storage_error)?;
        }
        audit(&transaction, &session.info, event, Some(&id))?;
        transaction.commit().map_err(storage_error)?;
        scoped_client(&session.connection, &session.info, &id)
    }

    pub fn add_client_note(&mut self, client_id: &str, body: &str) -> Result<ClientNote, String> {
        let session = self.session()?;
        let body = required_multiline_text(body, "Note", 8_000)?;
        scoped_client(&session.connection, &session.info, client_id)?;
        let note = ClientNote {
            id: Uuid::new_v4().to_string(),
            client_id: client_id.to_owned(),
            counselor_id: session.info.counselor_id.clone(),
            counselor_name: session.info.counselor_name.clone(),
            body,
            created_at: timestamp(),
        };
        let transaction = session.connection.transaction().map_err(storage_error)?;
        transaction.execute(
            "INSERT INTO client_notes (id, client_id, organization_id, counselor_id, counselor_name, body, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![note.id, note.client_id, session.info.organization_id, note.counselor_id,
                note.counselor_name, note.body, note.created_at],
        ).map_err(storage_error)?;
        audit(&transaction, &session.info, "note_added", Some(client_id))?;
        transaction.commit().map_err(storage_error)?;
        Ok(note)
    }

    pub fn list_client_notes(&mut self, client_id: &str) -> Result<Vec<ClientNote>, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        let mut statement = session.connection.prepare(
            "SELECT n.id, n.client_id, n.counselor_id, n.counselor_name, n.body, n.created_at
             FROM client_notes n JOIN clients c ON c.id = n.client_id AND c.organization_id = n.organization_id
             WHERE n.client_id = ?1 AND n.organization_id = ?2 AND n.counselor_id = ?3
                 AND c.counselor_id = ?3
             ORDER BY n.created_at DESC, n.id DESC",
        ).map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![
                    client_id,
                    session.info.organization_id,
                    session.info.counselor_id
                ],
                |row| {
                    Ok(ClientNote {
                        id: row.get(0)?,
                        client_id: row.get(1)?,
                        counselor_id: row.get(2)?,
                        counselor_name: row.get(3)?,
                        body: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .map_err(storage_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage_error)
    }

    pub fn get_client_dashboard(
        &mut self,
        client_id: &str,
        input: crate::dashboard::Input,
    ) -> Result<crate::dashboard::Assessment, String> {
        let snapshot = self.get_client_imports(client_id)?;
        crate::dashboard::assess(&snapshot, input, chrono::Utc::now().date_naive())
    }

    pub fn get_client_budget(&mut self, client_id: &str) -> Result<ClientBudget, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        read_budget(&session.connection, &session.info, client_id)
    }

    pub fn save_client_budget(
        &mut self,
        client_id: &str,
        input: BudgetInput,
    ) -> Result<ClientBudget, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        if !(0..i64::from(i32::MAX)).contains(&input.expected_revision) {
            return Err(
                "Invalid budget revision. Close and reopen the client before saving.".to_owned(),
            );
        }
        let (lines, summary) = budget::calculate(input.lines)?;
        let json = serde_json::to_string(&lines).map_err(|_| STORAGE_ERROR.to_owned())?;
        let transaction = session
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let current = read_budget(&transaction, &session.info, client_id)?;
        if current.revision != input.expected_revision {
            return Err("This budget changed in another window. Close and reopen the client to load it before saving again.".to_owned());
        }
        let revision = current.revision + 1;
        let now = timestamp();
        let changed = transaction.execute(
            "INSERT INTO client_budgets (client_id, organization_id, counselor_id, revision, lines_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(client_id) DO UPDATE SET revision = excluded.revision, lines_json = excluded.lines_json, updated_at = excluded.updated_at
             WHERE client_budgets.organization_id = excluded.organization_id AND client_budgets.counselor_id = excluded.counselor_id",
            params![client_id, session.info.organization_id, session.info.counselor_id, revision, json, now],
        ).map_err(storage_error)?;
        if changed != 1 {
            return Err(STORAGE_ERROR.to_owned());
        }
        audit(&transaction, &session.info, "budget_saved", Some(client_id))?;
        transaction.commit().map_err(storage_error)?;
        Ok(ClientBudget {
            client_id: client_id.to_owned(),
            revision,
            updated_at: Some(now),
            lines,
            summary,
        })
    }

    pub fn get_client_debt_plan(&mut self, client_id: &str) -> Result<ClientDebtPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        read_debt_plan(&session.connection, &session.info, client_id)
    }

    pub fn save_client_debt_plan(
        &mut self,
        client_id: &str,
        input: PlanInput,
    ) -> Result<ClientDebtPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        if !(0..i64::from(i32::MAX)).contains(&input.expected_revision) {
            return Err("Invalid debt plan revision. Close and reopen the client.".to_owned());
        }
        let (data, _) = debt::calculate(input.data)?;
        let json = serde_json::to_string(&data).map_err(|_| STORAGE_ERROR.to_owned())?;
        if json.len() > 131072 {
            return Err("The debt plan is too large.".to_owned());
        }
        let transaction = session
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&transaction, &session.info, client_id)?;
        let current = read_debt_plan(&transaction, &session.info, client_id)?;
        if current.revision != input.expected_revision {
            return Err("This debt plan changed in another window. Close and reopen the client before saving again.".to_owned());
        }
        let changed = transaction.execute(
            "INSERT INTO client_debt_plans (client_id, organization_id, counselor_id, revision, plan_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(client_id) DO UPDATE SET revision = excluded.revision, plan_json = excluded.plan_json, updated_at = excluded.updated_at
             WHERE client_debt_plans.organization_id = excluded.organization_id AND client_debt_plans.counselor_id = excluded.counselor_id",
            params![client_id, session.info.organization_id, session.info.counselor_id, current.revision + 1, json, timestamp()],
        ).map_err(storage_error)?;
        if changed != 1 {
            return Err(STORAGE_ERROR.to_owned());
        }
        audit(
            &transaction,
            &session.info,
            "debt_plan_saved",
            Some(client_id),
        )?;
        let result = read_debt_plan(&transaction, &session.info, client_id)?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }

    pub fn get_client_savings_plan(
        &mut self,
        client_id: &str,
    ) -> Result<ClientSavingsPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        read_savings_plan(&session.connection, &session.info, client_id)
    }

    pub fn save_client_savings_plan(
        &mut self,
        client_id: &str,
        input: savings::PlanInput,
    ) -> Result<ClientSavingsPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        if !(0..i64::from(i32::MAX)).contains(&input.expected_revision) {
            return Err("Invalid savings plan revision. Close and reopen the client.".to_owned());
        }
        let transaction = session
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&transaction, &session.info, client_id)?;
        let current = read_savings_plan(&transaction, &session.info, client_id)?;
        if current.revision != input.expected_revision {
            return Err("This savings plan changed in another window. Close and reopen the client before saving again.".to_owned());
        }
        let (data, _) = savings::calculate(input.data, &current.context)?;
        let json = serde_json::to_string(&data).map_err(|_| STORAGE_ERROR.to_owned())?;
        if json.len() > 65536 {
            return Err("The savings plan is too large.".to_owned());
        }
        let changed=transaction.execute(
            "INSERT INTO client_savings_plans (client_id,organization_id,counselor_id,revision,plan_json,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(client_id) DO UPDATE SET revision=excluded.revision,plan_json=excluded.plan_json,updated_at=excluded.updated_at
             WHERE client_savings_plans.organization_id=excluded.organization_id AND client_savings_plans.counselor_id=excluded.counselor_id",
            params![client_id,session.info.organization_id,session.info.counselor_id,current.revision+1,json,timestamp()],
        ).map_err(storage_error)?;
        if changed != 1 {
            return Err(STORAGE_ERROR.to_owned());
        }
        audit(
            &transaction,
            &session.info,
            "savings_plan_saved",
            Some(client_id),
        )?;
        let result = read_savings_plan(&transaction, &session.info, client_id)?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }

    pub fn get_client_cashflow_plan(
        &mut self,
        client_id: &str,
    ) -> Result<ClientCashflowPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        read_cashflow_plan(&session.connection, &session.info, client_id)
    }

    pub fn preview_client_cashflow_plan(
        &mut self,
        client_id: &str,
        input: cashflow::PlanInput,
    ) -> Result<ClientCashflowPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        let current = read_cashflow_plan(&session.connection, &session.info, client_id)?;
        if input.expected_revision != current.revision {
            return Err("This cash-flow plan changed in another window. Reopen the client before previewing or saving.".to_owned());
        }
        let (data, summary) = cashflow::calculate(input.data)?;
        Ok(ClientCashflowPlan {
            data: Some(data),
            summary: Some(summary),
            ..current
        })
    }

    pub fn save_client_cashflow_plan(
        &mut self,
        client_id: &str,
        input: cashflow::PlanInput,
    ) -> Result<ClientCashflowPlan, String> {
        let session = self.session()?;
        scoped_client(&session.connection, &session.info, client_id)?;
        if !(0..i64::from(i32::MAX)).contains(&input.expected_revision) {
            return Err("Invalid cash-flow revision. Reopen the client.".to_owned());
        }
        let transaction = session
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        scoped_client(&transaction, &session.info, client_id)?;
        let current = read_cashflow_plan(&transaction, &session.info, client_id)?;
        if input.expected_revision != current.revision {
            return Err(
                "This cash-flow plan changed in another window. Reopen the client before saving."
                    .to_owned(),
            );
        }
        let (data, _) = cashflow::calculate(input.data)?;
        let json = serde_json::to_string(&data).map_err(|_| STORAGE_ERROR.to_owned())?;
        if json.len() > 65536 {
            return Err("The cash-flow plan is too large.".to_owned());
        }
        let changed=transaction.execute(
            "INSERT INTO client_cashflow_plans (client_id,organization_id,counselor_id,revision,plan_json,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(client_id) DO UPDATE SET revision=excluded.revision,plan_json=excluded.plan_json,updated_at=excluded.updated_at
             WHERE client_cashflow_plans.organization_id=excluded.organization_id AND client_cashflow_plans.counselor_id=excluded.counselor_id",
            params![client_id,session.info.organization_id,session.info.counselor_id,current.revision+1,json,timestamp()],
        ).map_err(storage_error)?;
        if changed != 1 {
            return Err(STORAGE_ERROR.to_owned());
        }
        audit(
            &transaction,
            &session.info,
            "cashflow_plan_saved",
            Some(client_id),
        )?;
        let result = read_cashflow_plan(&transaction, &session.info, client_id)?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }

    fn session(&mut self) -> Result<&mut Session, String> {
        self.expire_if_idle();
        let session = self.session.as_mut().ok_or_else(|| LOCKED.to_owned())?;
        session.last_activity = Instant::now();
        Ok(session)
    }

    fn database_path(&self) -> PathBuf {
        self.active_dir.join(DATABASE_NAME)
    }
    fn metadata_path(&self) -> PathBuf {
        self.active_dir.join(METADATA_NAME)
    }

    fn read_metadata(&self) -> Result<Metadata, String> {
        let mut bytes = Vec::new();
        // Salt/version metadata is deliberately tiny. Bound malformed files
        // instead of allocating arbitrarily large input from disk.
        File::open(self.metadata_path())
            .map_err(|_| STORAGE_ERROR.to_owned())?
            .take(4_097)
            .read_to_end(&mut bytes)
            .map_err(|_| STORAGE_ERROR.to_owned())?;
        if bytes.len() > 4_096 {
            return Err(STORAGE_ERROR.to_owned());
        }
        let metadata: Metadata =
            serde_json::from_slice(&bytes).map_err(|_| STORAGE_ERROR.to_owned())?;
        if metadata.version != METADATA_VERSION {
            return Err("This workspace uses an unsupported format. Open it with the application version that created it.".to_owned());
        }
        Ok(metadata)
    }
}

fn regular_file_exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(STORAGE_ERROR.to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(STORAGE_ERROR.to_owned()),
    }
}

fn create_private_directory(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(|_| STORAGE_ERROR.to_owned())
}

fn create_private_file(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| STORAGE_ERROR.to_owned())
}

fn derive_key(password: &str, salt: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, String> {
    let parameters = Params::new(65_536, 3, 1, Some(32)).map_err(|_| STORAGE_ERROR.to_owned())?;
    let mut memory = Zeroizing::new(vec![Block::default(); parameters.block_count()]);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters);
    let mut key = Zeroizing::new([0_u8; 32]);
    argon2
        .hash_password_into_with_memory(password.as_bytes(), salt, &mut *key, &mut *memory)
        .map_err(|_| STORAGE_ERROR.to_owned())?;
    Ok(key)
}

fn open_encrypted(path: &Path, key: &[u8; 32]) -> Result<Connection, String> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(storage_error)?;
    #[cfg(windows)]
    configure_windows_sqlcipher_logging(&connection)?;
    let cipher_version: String = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .map_err(storage_error)?;
    if cipher_version.trim().is_empty() {
        return Err(STORAGE_ERROR.to_owned());
    }
    let mut key_statement = Zeroizing::new(String::from("PRAGMA key = \"x'"));
    for byte in key {
        write!(&mut *key_statement, "{byte:02x}").map_err(|_| STORAGE_ERROR.to_owned())?;
    }
    key_statement.push_str("'\"; PRAGMA cipher_memory_security = ON;");
    connection
        .execute_batch(&key_statement)
        .map_err(storage_error)?;
    // First real page read detects wrong keys without modifying the file.
    connection
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(storage_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(storage_error)?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;
         PRAGMA temp_store = MEMORY; PRAGMA synchronous = FULL;",
        )
        .map_err(storage_error)?;
    Ok(connection)
}

#[cfg(windows)]
fn configure_windows_sqlcipher_logging(connection: &Connection) -> Result<(), String> {
    // SQLCipher's log settings and hardened allocator are process-wide. Configure
    // them once, before any connection enables cipher_memory_security. Its
    // Windows stderr formatter allocates through SQLite; a VirtualLock quota
    // failure can recursively log another MEMORY warning during that allocation.
    // Keep core/provider/mutex diagnostics, memory wiping, and every lock attempt.
    // Only native MEMORY diagnostics are omitted; query errors and integrity
    // failures still propagate through the application's normal error handling.
    static LOGGING: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    LOGGING
        .get_or_init(|| {
            connection
                .execute_batch(
                    "PRAGMA cipher_log_source = NONE;
                     PRAGMA cipher_log_source = CORE;
                     PRAGMA cipher_log_source = PROVIDER;
                     PRAGMA cipher_log_source = MUTEX;",
                )
                .map_err(storage_error)
        })
        .clone()
}

fn integrity_check(connection: &Connection) -> Result<(), String> {
    let mut cipher_check = connection
        .prepare("PRAGMA cipher_integrity_check")
        .map_err(storage_error)?;
    let mut rows = cipher_check.query([]).map_err(storage_error)?;
    // SQLCipher returns no rows when every encrypted page authenticates.
    if rows.next().map_err(storage_error)?.is_some() {
        return Err(STORAGE_ERROR.to_owned());
    }
    let sqlite_check: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(storage_error)?;
    if sqlite_check != "ok" {
        return Err(STORAGE_ERROR.to_owned());
    }
    Ok(())
}

fn migrate(connection: &mut Connection, allow_new: bool) -> Result<(), String> {
    let mut version: i32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(storage_error)?;
    if version > SCHEMA_VERSION {
        return Err("This workspace was created by a newer application. Update the application before opening it.".to_owned());
    }
    if version < 0 || (version == 0 && !allow_new) {
        return Err(STORAGE_ERROR.to_owned());
    }
    let transaction = connection.transaction().map_err(storage_error)?;
    if version == 0 {
        transaction.execute_batch(
            "CREATE TABLE organizations (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL);
             CREATE TABLE counselors (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, name TEXT NOT NULL,
                UNIQUE (id, organization_id), FOREIGN KEY (organization_id) REFERENCES organizations(id)
             );
             CREATE TABLE workspace_settings (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             CREATE TABLE clients (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                first_name TEXT NOT NULL, last_name TEXT NOT NULL, email TEXT NOT NULL DEFAULT '',
                phone TEXT NOT NULL DEFAULT '', household_size INTEGER NOT NULL CHECK (household_size BETWEEN 1 AND 30),
                status TEXT NOT NULL CHECK (status IN ('New Intake', 'Information Collection', 'Financial Review', 'Planning', 'Action Plan', 'Follow-Up', 'Archived')),
                goals TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
                UNIQUE (id, organization_id),
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             CREATE INDEX clients_scope ON clients (organization_id, counselor_id, last_name, first_name);
             PRAGMA user_version = 1;",
        ).map_err(storage_error)?;
        transaction
            .pragma_update(None, "application_id", APPLICATION_ID)
            .map_err(storage_error)?;
        version = 1;
    }
    if version == 1 {
        transaction.execute_batch(
            "CREATE TABLE client_notes (
                id TEXT PRIMARY KEY NOT NULL, client_id TEXT NOT NULL, organization_id TEXT NOT NULL,
                counselor_id TEXT NOT NULL, counselor_name TEXT NOT NULL, body TEXT NOT NULL, created_at TEXT NOT NULL,
                FOREIGN KEY (client_id, organization_id) REFERENCES clients(id, organization_id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             CREATE INDEX client_notes_scope ON client_notes (organization_id, counselor_id, client_id, created_at);
             CREATE TABLE audit_events (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK (event IN ('workspace_created', 'client_created', 'client_updated', 'note_added')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             CREATE INDEX audit_events_scope ON audit_events (organization_id, counselor_id, created_at);
             CREATE TRIGGER client_notes_no_update BEFORE UPDATE ON client_notes
                BEGIN SELECT RAISE(ABORT, 'Notes are append-only'); END;
             CREATE TRIGGER client_notes_no_delete BEFORE DELETE ON client_notes
                BEGIN SELECT RAISE(ABORT, 'Notes are append-only'); END;
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             PRAGMA user_version = 2;",
        ).map_err(storage_error)?;
        version = 2;
    }
    if version == 2 {
        transaction.execute_batch(
            "CREATE TABLE client_budgets (
                client_id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 2147483647),
                lines_json TEXT NOT NULL CHECK (length(lines_json) <= 65536), updated_at TEXT NOT NULL,
                FOREIGN KEY (client_id, organization_id) REFERENCES clients(id, organization_id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             CREATE TABLE audit_events_v3 (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK (event IN ('workspace_created', 'client_created', 'client_updated', 'note_added', 'budget_saved')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             INSERT INTO audit_events_v3 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v3 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events (organization_id, counselor_id, created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             PRAGMA user_version = 3;",
        ).map_err(storage_error)?;
        version = 3;
    }
    if version == 3 {
        transaction.execute_batch(
            "CREATE TABLE client_debt_plans (
                client_id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 2147483647),
                plan_json TEXT NOT NULL CHECK (length(plan_json) <= 131072), updated_at TEXT NOT NULL,
                FOREIGN KEY (client_id, organization_id) REFERENCES clients(id, organization_id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             CREATE TABLE audit_events_v4 (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK (event IN ('workspace_created', 'client_created', 'client_updated', 'note_added', 'budget_saved', 'debt_plan_saved')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             INSERT INTO audit_events_v4 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v4 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events (organization_id, counselor_id, created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             PRAGMA user_version = 4;",
        ).map_err(storage_error)?;
        version = 4;
    }
    if version == 4 {
        transaction.execute_batch(
            "CREATE TABLE client_savings_plans (
                client_id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 2147483647),
                plan_json TEXT NOT NULL CHECK(length(plan_json)<=65536), updated_at TEXT NOT NULL,
                FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),
                FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
             );
             CREATE TABLE audit_events_v5 (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY(organization_id) REFERENCES organizations(id),
                FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
             );
             INSERT INTO audit_events_v5 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v5 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
             PRAGMA user_version=5;",
        ).map_err(storage_error)?;
        version = 5;
    }
    if version == 5 {
        transaction.execute_batch(
            "CREATE TABLE client_cashflow_plans (
                client_id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK(revision BETWEEN 1 AND 2147483647),
                plan_json TEXT NOT NULL CHECK(length(plan_json)<=65536), updated_at TEXT NOT NULL,
                FOREIGN KEY(client_id,organization_id) REFERENCES clients(id,organization_id),
                FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
             );
             CREATE TABLE audit_events_v6 (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY(organization_id) REFERENCES organizations(id),
                FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
             );
             INSERT INTO audit_events_v6 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v6 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
             PRAGMA user_version=6;",
        ).map_err(storage_error)?;
        version = 6;
    }
    if version == 6 {
        transaction.execute_batch(
            "CREATE TABLE audit_events_v7 (
                id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored')),
                entity_id TEXT,created_at TEXT NOT NULL,
                FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
            INSERT INTO audit_events_v7 SELECT * FROM audit_events; DROP TABLE audit_events;
            ALTER TABLE audit_events_v7 RENAME TO audit_events;
            CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
            CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
            CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
            PRAGMA user_version=7;"
        ).map_err(storage_error)?;
        version = 7;
    }
    if version == 7 {
        transaction
            .execute_batch(include_str!("imports-schema.sql"))
            .map_err(storage_error)?;
        version = 8;
    }
    if version == 8 {
        transaction
            .execute_batch(include_str!("pdf-schema.sql"))
            .map_err(storage_error)?;
        version = 9;
    }
    if version == 9 {
        transaction
            .execute_batch(include_str!("recurring-schema.sql"))
            .map_err(storage_error)?;
        version = 10;
    }
    if version == 10 {
        transaction
            .execute_batch(include_str!("recurring-invalidation-schema.sql"))
            .map_err(storage_error)?;
        version = 11;
    }
    if version == 11 {
        transaction
            .execute_batch(include_str!("reconciliation-schema.sql"))
            .map_err(storage_error)?;
        version = 12;
    }
    if version == 12 {
        transaction
            .execute_batch(include_str!("financial-facts-schema.sql"))
            .map_err(storage_error)?;
        version = 13;
    }
    if version == 13 {
        transaction
            .execute_batch(include_str!("assessments-schema.sql"))
            .map_err(storage_error)?;
    }
    transaction.commit().map_err(storage_error)?;
    integrity_check(connection)
}

fn read_cashflow_plan(
    connection: &Connection,
    info: &WorkspaceInfo,
    client_id: &str,
) -> Result<ClientCashflowPlan, String> {
    let savings = read_savings_plan(connection, info, client_id)?;
    let context = crate::planning::cashflow_context(&savings);
    let saved:Option<(i64,String,String)>=connection.query_row(
        "SELECT revision,plan_json,updated_at FROM client_cashflow_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",
        params![client_id,info.organization_id,info.counselor_id], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).optional().map_err(storage_error)?;
    match saved {
        Some((revision, json, updated_at)) => {
            let data: cashflow::PlanData =
                serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
            let (data, summary) =
                cashflow::calculate(data).map_err(|_| STORAGE_ERROR.to_owned())?;
            Ok(ClientCashflowPlan {
                client_id: client_id.to_owned(),
                revision,
                updated_at: Some(updated_at),
                data: Some(data),
                summary: Some(summary),
                context,
            })
        }
        None => Ok(ClientCashflowPlan {
            client_id: client_id.to_owned(),
            revision: 0,
            updated_at: None,
            data: None,
            summary: None,
            context,
        }),
    }
}

fn read_savings_plan(
    connection: &Connection,
    info: &WorkspaceInfo,
    client_id: &str,
) -> Result<ClientSavingsPlan, String> {
    let budget = read_budget(connection, info, client_id)?;
    let debt = read_debt_plan(connection, info, client_id)?;
    let context = crate::planning::savings_context(&budget, &debt);
    let saved: Option<(i64,String,String)>=connection.query_row(
        "SELECT revision,plan_json,updated_at FROM client_savings_plans WHERE client_id=?1 AND organization_id=?2 AND counselor_id=?3",
        params![client_id,info.organization_id,info.counselor_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).optional().map_err(storage_error)?;
    let (revision, data, summary, updated_at) = match saved {
        Some((revision, json, updated_at)) => {
            if json.len() > 65536 {
                return Err(STORAGE_ERROR.to_owned());
            }
            let data: savings::PlanData =
                serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
            let (data, summary) =
                savings::calculate(data, &context).map_err(|_| STORAGE_ERROR.to_owned())?;
            (revision, Some(data), Some(summary), Some(updated_at))
        }
        None => (0, None, None, None),
    };
    Ok(ClientSavingsPlan {
        client_id: client_id.to_owned(),
        revision,
        updated_at,
        data,
        summary,
        context,
    })
}

fn read_debt_plan(
    connection: &Connection,
    info: &WorkspaceInfo,
    client_id: &str,
) -> Result<ClientDebtPlan, String> {
    let saved: Option<(i64, String, String)> = connection.query_row(
        "SELECT revision, plan_json, updated_at FROM client_debt_plans WHERE client_id = ?1 AND organization_id = ?2 AND counselor_id = ?3",
        params![client_id, info.organization_id, info.counselor_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(storage_error)?;
    let budget = read_budget(connection, info, client_id)?;
    let context = crate::planning::debt_context(&budget);
    let (revision, data, comparison, updated_at) = match saved {
        Some((revision, json, updated_at)) => {
            if json.len() > 131072 {
                return Err(STORAGE_ERROR.to_owned());
            }
            let data: debt::PlanData =
                serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
            let (data, mut comparison) =
                debt::calculate(data).map_err(|_| STORAGE_ERROR.to_owned())?;
            debt::apply_budget_context(&data, &mut comparison, &context)
                .map_err(|_| STORAGE_ERROR.to_owned())?;
            (revision, Some(data), Some(comparison), Some(updated_at))
        }
        None => (0, None, None, None),
    };
    Ok(ClientDebtPlan {
        client_id: client_id.to_owned(),
        revision,
        updated_at,
        data,
        comparison,
        budget_context: context,
    })
}

fn read_budget(
    connection: &Connection,
    info: &WorkspaceInfo,
    client_id: &str,
) -> Result<ClientBudget, String> {
    let saved: Option<(i64, String, String)> = connection.query_row(
        "SELECT revision, lines_json, updated_at FROM client_budgets WHERE client_id = ?1 AND organization_id = ?2 AND counselor_id = ?3",
        params![client_id, info.organization_id, info.counselor_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(storage_error)?;
    let (revision, lines, updated_at) = match saved {
        Some((revision, json, updated_at)) => {
            if json.len() > 65536 {
                return Err(STORAGE_ERROR.to_owned());
            }
            let lines: Vec<BudgetLine> =
                serde_json::from_str(&json).map_err(|_| STORAGE_ERROR.to_owned())?;
            (revision, lines, Some(updated_at))
        }
        None => (0, Vec::new(), None),
    };
    let (lines, summary) = budget::calculate(lines).map_err(|_| STORAGE_ERROR.to_owned())?;
    Ok(ClientBudget {
        client_id: client_id.to_owned(),
        revision,
        updated_at,
        lines,
        summary,
    })
}

fn audit(
    transaction: &Transaction<'_>,
    info: &WorkspaceInfo,
    event: &str,
    entity_id: Option<&str>,
) -> Result<(), String> {
    transaction.execute(
        "INSERT INTO audit_events (id, organization_id, counselor_id, event, entity_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![Uuid::new_v4().to_string(), info.organization_id, info.counselor_id, event, entity_id, timestamp()],
    ).map_err(storage_error)?;
    Ok(())
}

fn scoped_client(
    connection: &Connection,
    info: &WorkspaceInfo,
    id: &str,
) -> Result<Client, String> {
    if Uuid::parse_str(id).is_err() {
        return Err(UNAVAILABLE_CLIENT.to_owned());
    }
    connection
        .query_row(
            "SELECT id, organization_id, first_name, last_name, email, phone,
                household_size, status, goals, created_at, updated_at
         FROM clients WHERE id = ?1 AND organization_id = ?2 AND counselor_id = ?3",
            params![id, info.organization_id, info.counselor_id],
            client_from_row,
        )
        .optional()
        .map_err(storage_error)?
        .ok_or_else(|| UNAVAILABLE_CLIENT.to_owned())
}

fn client_from_row(row: &Row<'_>) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get(0)?,
        organization_id: row.get(1)?,
        first_name: row.get(2)?,
        last_name: row.get(3)?,
        email: row.get(4)?,
        phone: row.get(5)?,
        household_size: row.get(6)?,
        status: row.get(7)?,
        goals: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn validate_password(password: &str) -> Result<(), String> {
    if !(12..=1_024).contains(&password.chars().count()) {
        return Err("Use a password with 12 to 1,024 characters.".to_owned());
    }
    Ok(())
}

fn required_text(value: &str, field: &str, limit: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > limit
        || trimmed.chars().any(char::is_control)
    {
        return Err(format!(
            "{field} must contain 1 to {limit} characters without control characters."
        ));
    }
    Ok(trimmed.to_owned())
}

fn optional_text(
    value: &str,
    field: &str,
    limit: usize,
    multiline: bool,
) -> Result<String, String> {
    let trimmed = value.trim();
    let invalid_control = trimmed.chars().any(|character| {
        character.is_control() && !(multiline && matches!(character, '\n' | '\r' | '\t'))
    });
    if trimmed.chars().count() > limit || invalid_control {
        return Err(format!(
            "{field} must contain no more than {limit} characters."
        ));
    }
    Ok(trimmed.to_owned())
}

fn required_multiline_text(value: &str, field: &str, limit: usize) -> Result<String, String> {
    let value = optional_text(value, field, limit, true)?;
    if value.is_empty() {
        return Err(format!("{field} cannot be empty."));
    }
    Ok(value)
}

fn validate_client(mut input: ClientInput) -> Result<ClientInput, String> {
    input.first_name = required_text(&input.first_name, "First name", 100)?;
    input.last_name = required_text(&input.last_name, "Last name", 100)?;
    input.email = optional_text(&input.email, "Email", 254, false)?;
    if !input.email.is_empty() {
        let parts: Vec<_> = input.email.split('@').collect();
        let valid = parts.len() == 2
            && !parts[0].is_empty()
            && parts[1].contains('.')
            && parts[1].split('.').all(|part| !part.is_empty())
            && !input.email.chars().any(char::is_whitespace);
        if !valid {
            return Err("Enter a valid email address, or leave it blank.".to_owned());
        }
    }
    input.phone = optional_text(&input.phone, "Phone", 40, false)?;
    input.goals = optional_text(&input.goals, "Goals", 4_000, true)?;
    if !(1..=30).contains(&input.household_size) {
        return Err("Household size must be a whole number from 1 to 30.".to_owned());
    }
    if !matches!(
        input.status.as_str(),
        "New Intake"
            | "Information Collection"
            | "Financial Review"
            | "Planning"
            | "Action Plan"
            | "Follow-Up"
            | "Archived"
    ) {
        return Err("Choose a valid client status.".to_owned());
    }
    Ok(input)
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn storage_error(_: rusqlite::Error) -> String {
    STORAGE_ERROR.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    pub(super) const PASSWORD: &str = "Synthetic-test-passphrase-42";

    pub(super) fn configured() -> (TempDir, Workspace, WorkspaceInfo) {
        let directory = tempdir().unwrap();
        let mut workspace = Workspace::new(directory.path().to_path_buf());
        let info = workspace
            .setup_workspace(SetupWorkspaceInput {
                organization_name: "Synthetic Test Organization".to_owned(),
                counselor_name: "Synthetic Test Counselor".to_owned(),
                password: PASSWORD.to_owned(),
            })
            .unwrap();
        (directory, workspace, info)
    }

    pub(super) fn client_input() -> ClientInput {
        ClientInput {
            id: None,
            first_name: " Synthetic ".to_owned(),
            last_name: " Person ".to_owned(),
            email: "synthetic@example.invalid".to_owned(),
            phone: "555-0100".to_owned(),
            household_size: 2,
            status: "New Intake".to_owned(),
            goals: "SYNTHETIC_PRIVATE_GOAL_812767".to_owned(),
        }
    }

    #[test]
    fn dashboard_requires_authenticated_client_scope_and_reopens_from_encrypted_sources() {
        let (directory, mut w, _) = configured();
        let client = w.save_client(client_input()).unwrap();
        let period = || crate::dashboard::Input {
            from_month: "2026-09".into(),
            through_month: "2026-10".into(),
        };
        assert!(w.get_client_dashboard("other-client", period()).is_err());
        assert_eq!(
            w.get_client_dashboard(&client.id, period())
                .unwrap()
                .client_id,
            client.id
        );
        w.lock_workspace();
        assert!(w
            .get_client_dashboard(&client.id, period())
            .err()
            .unwrap()
            .contains("locked"));
        drop(w);
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(
            reopened
                .get_client_dashboard(&client.id, period())
                .unwrap()
                .totals
                .income,
            "0.00"
        );
    }

    #[test]
    fn persists_clients_notes_and_identity_across_reopen() {
        let (directory, mut workspace, info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        assert_eq!(client.first_name, "Synthetic");
        let note = workspace
            .add_client_note(&client.id, "Synthetic follow-up note")
            .unwrap();
        drop(workspace);
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        assert!(reopened.setup_status().unwrap().configured);
        assert_eq!(
            reopened.unlock_workspace(PASSWORD.to_owned()).unwrap(),
            info
        );
        assert_eq!(reopened.get_client(&client.id).unwrap(), client);
        assert_eq!(reopened.list_clients().unwrap(), vec![client.clone()]);
        assert_eq!(reopened.list_client_notes(&client.id).unwrap(), vec![note]);
        let mut updated = client_input();
        updated.id = Some(client.id.clone());
        updated.status = "Follow-Up".to_owned();
        updated.household_size = 3;
        let updated = reopened.save_client(updated).unwrap();
        assert_eq!(updated.created_at, client.created_at);
        assert_eq!(updated.status, "Follow-Up");
        assert_eq!(updated.household_size, 3);
        drop(reopened);
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(reopened.get_client(&client.id).unwrap(), updated);
    }

    #[test]
    fn database_is_sqlcipher_encrypted_and_metadata_contains_no_identity_or_password() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        workspace
            .add_client_note(&client.id, "SYNTHETIC_PRIVATE_NOTE_157892")
            .unwrap();
        let version: String = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row("PRAGMA cipher_version", [], |row| row.get(0))
            .unwrap();
        assert!(!version.is_empty());
        workspace.lock_workspace();
        let bytes = fs::read(workspace.database_path()).unwrap();
        assert!(!bytes.starts_with(b"SQLite format 3"));
        for private_text in [
            PASSWORD,
            "Synthetic Test Organization",
            "synthetic@example.invalid",
            "SYNTHETIC_PRIVATE_GOAL_812767",
            "SYNTHETIC_PRIVATE_NOTE_157892",
        ] {
            assert!(!bytes
                .windows(private_text.len())
                .any(|window| window == private_text.as_bytes()));
        }
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(workspace.metadata_path()).unwrap()).unwrap();
        let object = metadata.as_object().unwrap();
        assert_eq!(object.len(), 2);
        assert!(object.contains_key("version"));
        assert!(object.contains_key("salt"));
        let unkeyed = Connection::open(workspace.database_path()).unwrap();
        assert!(unkeyed
            .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| row
                .get::<_, i64>(0))
            .is_err());
    }

    #[test]
    fn incorrect_password_keeps_workspace_locked_and_files_unchanged() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        workspace.lock_workspace();
        let original = fs::read(workspace.database_path()).unwrap();
        let error = workspace
            .unlock_workspace("Wrong-test-passphrase-99".to_owned())
            .unwrap_err();
        assert_eq!(error, OPEN_ERROR);
        assert_eq!(workspace.list_clients().unwrap_err(), LOCKED);
        assert_eq!(original, fs::read(workspace.database_path()).unwrap());
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
    }

    #[cfg(windows)]
    #[test]
    fn windows_memory_lock_quota_preserves_encryption_and_password_rejection() {
        // A separate process prevents this deliberately small working-set quota
        // from affecting parallel tests or the desktop application. Run the real
        // allocator/authentication path; checking a PRAGMA value alone would not
        // detect the native recursive-logging crash.
        const CHILD: &str = "EISENBART_SQLCIPHER_LOW_LOCK_QUOTA_TEST";
        const COMPLETE: &str = "Quota-constrained encrypted workspace regression passed";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "storage::tests::windows_memory_lock_quota_preserves_encryption_and_password_rejection",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "Quota-constrained storage process failed: {}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains(COMPLETE),
                "The storage regression body did not complete in the child process",
            );
            return;
        }

        use std::ffi::c_void;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
            fn SetProcessWorkingSetSize(process: *mut c_void, min: usize, max: usize) -> i32;
            fn VirtualAlloc(
                address: *mut c_void,
                size: usize,
                kind: u32,
                protect: u32,
            ) -> *mut c_void;
            fn VirtualLock(address: *mut c_void, size: usize) -> i32;
            fn VirtualUnlock(address: *mut c_void, size: usize) -> i32;
            fn VirtualFree(address: *mut c_void, size: usize, kind: u32) -> i32;
            fn GetLastError() -> u32;
        }
        const ALLOCATION: usize = 2 * 1024 * 1024;
        const MEM_COMMIT: u32 = 0x1000;
        const MEM_RESERVE: u32 = 0x2000;
        const MEM_RELEASE: u32 = 0x8000;
        const PAGE_READWRITE: u32 = 0x04;
        const ERROR_WORKING_SET_QUOTA: u32 = 1453;
        // SAFETY: these calls affect only this disposable child process. The
        // region is owned here and freed once after reading the lock result.
        // GetCurrentProcess supplies a pseudo-handle that must not be closed.
        unsafe {
            assert_ne!(
                SetProcessWorkingSetSize(GetCurrentProcess(), 64 * 1024, ALLOCATION),
                0,
                "Could not set the regression process working-set quota: {}",
                GetLastError(),
            );
            let region = VirtualAlloc(
                std::ptr::null_mut(),
                ALLOCATION,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            assert!(!region.is_null(), "Could not allocate the quota probe");
            let locked = VirtualLock(region, ALLOCATION);
            let lock_error = GetLastError();
            if locked != 0 {
                VirtualUnlock(region, ALLOCATION);
            }
            let freed = VirtualFree(region, 0, MEM_RELEASE);
            assert_ne!(freed, 0, "Could not release the quota probe");
            assert_eq!(
                locked, 0,
                "The regression must exercise a real lock failure"
            );
            assert_eq!(
                lock_error, ERROR_WORKING_SET_QUOTA,
                "Expected ERROR_WORKING_SET_QUOTA"
            );
        }

        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let connection = &workspace.session.as_ref().unwrap().connection;
        let blob: Vec<u8> = connection
            .query_row("SELECT zeroblob(?1)", [ALLOCATION as i64], |row| row.get(0))
            .unwrap();
        assert_eq!(blob.len(), ALLOCATION);
        assert!(blob.iter().all(|byte| *byte == 0));
        let memory_security: String = connection
            .query_row("PRAGMA cipher_memory_security", [], |row| row.get(0))
            .unwrap();
        assert_eq!(memory_security, "1");
        integrity_check(connection).unwrap();
        workspace.lock_workspace();
        let original = fs::read(workspace.database_path()).unwrap();
        assert!(!original.starts_with(b"SQLite format 3"));
        assert_eq!(
            workspace
                .unlock_workspace("Wrong-test-passphrase-99".to_owned())
                .unwrap_err(),
            OPEN_ERROR,
        );
        assert_eq!(workspace.list_clients().unwrap_err(), LOCKED);
        assert_eq!(original, fs::read(workspace.database_path()).unwrap());
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        println!("{COMPLETE}");
    }

    #[test]
    fn locked_data_apis_and_idle_expiration_reject_access() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        workspace.lock_workspace();
        assert_eq!(workspace.workspace_info().unwrap_err(), LOCKED);
        assert_eq!(workspace.list_clients().unwrap_err(), LOCKED);
        assert_eq!(workspace.get_client(&client.id).unwrap_err(), LOCKED);
        assert_eq!(workspace.save_client(client_input()).unwrap_err(), LOCKED);
        assert_eq!(
            workspace
                .add_client_note(&client.id, "Synthetic note")
                .unwrap_err(),
            LOCKED
        );
        assert_eq!(workspace.list_client_notes(&client.id).unwrap_err(), LOCKED);
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        workspace.session.as_mut().unwrap().last_activity = Instant::now() - AUTO_LOCK_AFTER;
        assert!(workspace.expire_if_idle());
        assert!(workspace.session.is_none());
        assert!(!workspace.expire_if_idle());
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        workspace.session.as_mut().unwrap().last_activity = Instant::now() - AUTO_LOCK_AFTER;
        assert_eq!(workspace.get_client(&client.id).unwrap_err(), LOCKED);
        assert!(workspace.session.is_none());
    }

    #[test]
    fn validates_inputs_before_writing_and_uses_parameterized_values() {
        let (_directory, mut workspace, _info) = configured();
        for input in [
            ClientInput {
                first_name: " ".to_owned(),
                ..client_input()
            },
            ClientInput {
                last_name: "x".repeat(101),
                ..client_input()
            },
            ClientInput {
                email: "invalid@".to_owned(),
                ..client_input()
            },
            ClientInput {
                email: "person@invalid..test".to_owned(),
                ..client_input()
            },
            ClientInput {
                phone: "x".repeat(41),
                ..client_input()
            },
            ClientInput {
                household_size: 0,
                ..client_input()
            },
            ClientInput {
                household_size: 31,
                ..client_input()
            },
            ClientInput {
                status: "Unknown".to_owned(),
                ..client_input()
            },
            ClientInput {
                goals: "x".repeat(4_001),
                ..client_input()
            },
            ClientInput {
                first_name: "Invalid\0Name".to_owned(),
                ..client_input()
            },
        ] {
            assert!(workspace.save_client(input).is_err());
        }
        assert!(workspace.list_clients().unwrap().is_empty());
        let mut input = client_input();
        input.last_name = "O'Example; DROP TABLE clients; --".to_owned();
        let client = workspace.save_client(input).unwrap();
        assert!(client.last_name.starts_with("O'Example"));
        assert!(workspace.add_client_note(&client.id, " ").is_err());
        assert!(workspace
            .add_client_note(&client.id, &"x".repeat(8_001))
            .is_err());
        assert!(workspace.list_client_notes(&client.id).unwrap().is_empty());
        assert!(validate_password("short").is_err());
        assert!(validate_password(&"x".repeat(1_025)).is_err());
        assert!(validate_password(&"é".repeat(12)).is_ok());
        assert!(serde_json::from_value::<ClientInput>(serde_json::json!({
            "firstName": "Synthetic", "lastName": "Person", "email": "", "phone": "",
            "householdSize": 1.5, "status": "New Intake", "goals": ""
        }))
        .is_err());
    }

    #[test]
    fn organization_and_counselor_scopes_block_foreign_clients_and_notes() {
        let (_directory, mut workspace, info) = configured();
        let local = workspace.save_client(client_input()).unwrap();
        let other_organization = Uuid::new_v4().to_string();
        let other_counselor = Uuid::new_v4().to_string();
        let same_org_other_counselor = Uuid::new_v4().to_string();
        let other_org_client = Uuid::new_v4().to_string();
        let other_counselor_client = Uuid::new_v4().to_string();
        {
            let connection = &workspace.session.as_ref().unwrap().connection;
            connection.execute("INSERT INTO organizations (id, name) VALUES (?1, 'Synthetic Other Organization')", [&other_organization]).unwrap();
            connection.execute("INSERT INTO counselors (id, organization_id, name) VALUES (?1, ?2, 'Synthetic Other Counselor')", params![other_counselor, other_organization]).unwrap();
            connection.execute("INSERT INTO counselors (id, organization_id, name) VALUES (?1, ?2, 'Synthetic Same Org Counselor')", params![same_org_other_counselor, info.organization_id]).unwrap();
            for (id, organization, counselor) in [
                (&other_org_client, &other_organization, &other_counselor),
                (
                    &other_counselor_client,
                    &info.organization_id,
                    &same_org_other_counselor,
                ),
            ] {
                connection.execute(
                    "INSERT INTO clients (id, organization_id, counselor_id, first_name, last_name, household_size, status, created_at, updated_at)
                     VALUES (?1, ?2, ?3, 'Synthetic', 'Foreign', 1, 'New Intake', ?4, ?4)",
                    params![id, organization, counselor, timestamp()],
                ).unwrap();
            }
            // Even a note authored by another counselor on this counselor's
            // client cannot be returned by the current-session note query.
            connection.execute(
                "INSERT INTO client_notes (id, client_id, organization_id, counselor_id, counselor_name, body, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'Synthetic Other Counselor', 'Synthetic hidden note', ?5)",
                params![Uuid::new_v4().to_string(), local.id, info.organization_id, same_org_other_counselor, timestamp()],
            ).unwrap();
        }
        assert_eq!(workspace.list_clients().unwrap(), vec![local.clone()]);
        assert!(workspace.list_client_notes(&local.id).unwrap().is_empty());
        for foreign_id in [&other_org_client, &other_counselor_client] {
            assert_eq!(
                workspace.get_client_imports(foreign_id).err().unwrap(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .inspect_client_pdf(foreign_id.to_string(), "never-read.pdf".into())
                    .err()
                    .unwrap(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .inspect_client_csv(foreign_id.to_string(), "never-read.csv".into())
                    .err()
                    .unwrap(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace.get_client_cashflow_plan(foreign_id).unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .preview_client_cashflow_plan(foreign_id, cashflow_input(0))
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .save_client_cashflow_plan(foreign_id, cashflow_input(0))
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace.get_client_savings_plan(foreign_id).unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .save_client_savings_plan(foreign_id, savings_input(0))
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace.get_client_debt_plan(foreign_id).unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .save_client_debt_plan(foreign_id, debt_input(0))
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace.get_client_budget(foreign_id).unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .save_client_budget(foreign_id, budget_input(0))
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace.get_client(foreign_id).unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .save_client(ClientInput {
                        id: Some(foreign_id.clone()),
                        ..client_input()
                    })
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace
                    .add_client_note(foreign_id, "Synthetic forbidden note")
                    .unwrap_err(),
                UNAVAILABLE_CLIENT
            );
            assert_eq!(
                workspace.list_client_notes(foreign_id).unwrap_err(),
                UNAVAILABLE_CLIENT
            );
        }
        let count: i64 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM clients", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn notes_and_audit_are_append_only_and_audit_excludes_content() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic confidential note")
            .unwrap();
        let connection = &workspace.session.as_ref().unwrap().connection;
        assert!(connection
            .execute(
                "UPDATE client_notes SET body = 'changed' WHERE id = ?1",
                [&note.id]
            )
            .is_err());
        assert!(connection
            .execute("DELETE FROM client_notes WHERE id = ?1", [&note.id])
            .is_err());
        assert!(connection
            .execute("UPDATE audit_events SET event = 'client_updated'", [])
            .is_err());
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        let mut statement = connection
            .prepare("SELECT event, entity_id FROM audit_events ORDER BY rowid")
            .unwrap();
        let events = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            events,
            vec![
                ("workspace_created".to_owned(), None),
                ("client_created".to_owned(), Some(client.id.clone())),
                ("note_added".to_owned(), Some(client.id.clone())),
            ]
        );
        let columns = connection
            .prepare("PRAGMA table_info(audit_events)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            columns,
            vec![
                "id",
                "organization_id",
                "counselor_id",
                "event",
                "entity_id",
                "created_at"
            ]
        );
        drop(statement);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
    }

    pub(super) fn cashflow_input(revision: i64) -> cashflow::PlanInput {
        cashflow::PlanInput {
            expected_revision: revision,
            data: cashflow::tests::fixture(),
        }
    }
    #[test]
    fn cashflow_preview_persistence_references_scope_and_lock() {
        let (directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let second = workspace
            .save_client(ClientInput {
                first_name: "Other".into(),
                ..client_input()
            })
            .unwrap();
        let budget = workspace
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        let debt = workspace
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap();
        let savings = workspace
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap();
        let preview = workspace
            .preview_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap();
        assert_eq!(preview.revision, 0);
        assert_eq!(preview.context.savings_revision, 1);
        assert!(workspace
            .get_client_cashflow_plan(&client.id)
            .unwrap()
            .data
            .is_none());
        let saved = workspace
            .save_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.context.budget_revision, 1);
        assert_eq!(saved.context.debt_revision, 1);
        assert_eq!(
            saved.summary.as_ref().unwrap().combined_closing.as_deref(),
            Some("3300.00")
        );
        let mut change = cashflow_input(1);
        change.data.schedules[0].first_date = "2026-11-08".into();
        let adjusted = workspace
            .preview_client_cashflow_plan(&client.id, change)
            .unwrap();
        assert!(adjusted
            .summary
            .unwrap()
            .days
            .iter()
            .all(|d| !d.warnings.iter().any(|w| w.contains("shortfall"))));
        assert_eq!(
            workspace.get_client_cashflow_plan(&client.id).unwrap(),
            saved
        );
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), budget);
        assert_eq!(
            serde_json::to_value(workspace.get_client_debt_plan(&client.id).unwrap()).unwrap(),
            serde_json::to_value(&debt).unwrap()
        );
        assert_eq!(
            workspace.get_client_savings_plan(&client.id).unwrap(),
            savings
        );
        assert!(workspace
            .get_client_cashflow_plan(&second.id)
            .unwrap()
            .data
            .is_none());
        let mut invalid = cashflow_input(1);
        invalid.data.schedules[0].amount = Some("1.001".into());
        assert!(workspace
            .save_client_cashflow_plan(&client.id, invalid)
            .is_err());
        assert_eq!(
            workspace.get_client_cashflow_plan(&client.id).unwrap(),
            saved
        );
        let count: i64 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event='cashflow_plan_saved'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        workspace.lock_workspace();
        assert_eq!(
            workspace.get_client_cashflow_plan(&client.id).unwrap_err(),
            LOCKED
        );
        assert_eq!(
            workspace
                .preview_client_cashflow_plan(&client.id, cashflow_input(1))
                .unwrap_err(),
            LOCKED
        );
        assert_eq!(
            workspace
                .save_client_cashflow_plan(&client.id, cashflow_input(1))
                .unwrap_err(),
            LOCKED
        );
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.into()).unwrap();
        assert_eq!(
            reopened.get_client_cashflow_plan(&client.id).unwrap(),
            saved
        );
        let mut updated = budget_input(1);
        updated.lines[0].amount = Some("1800".into());
        reopened.save_client_budget(&client.id, updated).unwrap();
        let refreshed = reopened.get_client_cashflow_plan(&client.id).unwrap();
        assert_eq!(refreshed.context.budget_revision, 2);
        assert_eq!(refreshed.data, saved.data);
        assert_eq!(refreshed.summary, saved.summary);
        assert_eq!(refreshed.revision, 1);
    }
    #[test]
    fn cashflow_competing_writes_and_audit_failure_are_atomic() {
        let (directory, mut workspace, _) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let mut other = Workspace::new(directory.path().to_path_buf());
        other.unlock_workspace(PASSWORD.into()).unwrap();
        let saved = workspace
            .save_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap();
        assert!(other
            .save_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap_err()
            .contains("another window"));
        assert!(other
            .preview_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap_err()
            .contains("another window"));
        workspace.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER reject_cashflow_audit BEFORE INSERT ON audit_events WHEN NEW.event='cashflow_plan_saved' BEGIN SELECT RAISE(ABORT,'Synthetic disk failure'); END;").unwrap();
        let mut changed = cashflow_input(1);
        changed.data.notes = "Changed".into();
        assert!(workspace
            .save_client_cashflow_plan(&client.id, changed)
            .is_err());
        assert_eq!(other.get_client_cashflow_plan(&client.id).unwrap(), saved);
    }
    #[test]
    fn csv_schema_seven_migration_and_failure_preserve_all_records() {
        let (_directory, mut workspace, _) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic pre-import history")
            .unwrap();
        let budget = workspace
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        let debt = workspace
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap();
        let savings = workspace
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap();
        let cashflow = workspace
            .save_client_cashflow_plan(&client.id, cashflow_input(0))
            .unwrap();
        let connection = &mut workspace.session.as_mut().unwrap().connection;
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        connection.execute_batch("DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices;
            DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state;
            CREATE TABLE audit_events_old (
                id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved','cashflow_plan_saved','backup_created','workspace_restored')),
                entity_id TEXT,created_at TEXT NOT NULL,
                FOREIGN KEY(organization_id) REFERENCES organizations(id), FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
            INSERT INTO audit_events_old SELECT * FROM audit_events; DROP TABLE audit_events;
            ALTER TABLE audit_events_old RENAME TO audit_events;
            CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
            CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
            CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
            PRAGMA user_version=7; CREATE TABLE audit_events_v8(collision TEXT);").unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            7
        );
        assert!(connection
            .prepare("SELECT * FROM import_documents")
            .is_err());
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        connection
            .execute("DROP TABLE audit_events_v8", [])
            .unwrap();
        migrate(connection, false).unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            count
        );
        assert!(connection
            .execute("UPDATE audit_events SET event='client_updated'", [])
            .is_err());
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), budget);
        assert_eq!(
            serde_json::to_value(workspace.get_client_debt_plan(&client.id).unwrap()).unwrap(),
            serde_json::to_value(debt).unwrap()
        );
        assert_eq!(
            workspace.get_client_savings_plan(&client.id).unwrap(),
            savings
        );
        assert_eq!(
            workspace.get_client_cashflow_plan(&client.id).unwrap(),
            cashflow
        );
        assert!(workspace
            .get_client_imports(&client.id)
            .unwrap()
            .records
            .is_empty());
    }
    #[test]
    fn cashflow_schema_five_migration_and_failure_preserve_all_records() {
        let (_directory, mut workspace, _) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic historical note")
            .unwrap();
        let budget = workspace
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        let debt = workspace
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap();
        let savings = workspace
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap();
        let connection = &mut workspace.session.as_mut().unwrap().connection;
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        connection.execute_batch("DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices; DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state; DROP TABLE client_cashflow_plans;
            CREATE TABLE audit_events_v5 (
                id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved','savings_plan_saved')),
                entity_id TEXT,created_at TEXT NOT NULL,
                FOREIGN KEY(organization_id) REFERENCES organizations(id),FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id));
            INSERT INTO audit_events_v5 SELECT * FROM audit_events; DROP TABLE audit_events;
            ALTER TABLE audit_events_v5 RENAME TO audit_events;
            CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
            CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
            CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
            PRAGMA user_version=5;
            CREATE TABLE audit_events_v6(collision TEXT);").unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            5
        );
        assert!(connection
            .prepare("SELECT * FROM client_cashflow_plans")
            .is_err());
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        connection
            .execute("DROP TABLE audit_events_v6", [])
            .unwrap();
        migrate(connection, false).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            count
        );
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            SCHEMA_VERSION
        );
        assert!(connection
            .execute("UPDATE audit_events SET event='client_updated'", [])
            .is_err());
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), budget);
        assert_eq!(
            serde_json::to_value(workspace.get_client_debt_plan(&client.id).unwrap()).unwrap(),
            serde_json::to_value(&debt).unwrap()
        );
        assert_eq!(
            workspace.get_client_savings_plan(&client.id).unwrap(),
            savings
        );
        assert_eq!(
            workspace
                .save_client_cashflow_plan(&client.id, cashflow_input(0))
                .unwrap()
                .revision,
            1
        );
    }

    pub(super) fn savings_input(revision: i64) -> savings::PlanInput {
        savings::PlanInput {
            expected_revision: revision,
            data: savings::tests::fixture(),
        }
    }

    #[test]
    fn savings_plans_persist_and_recheck_context_without_rewriting_other_plans() {
        let (directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let other = workspace
            .save_client(ClientInput {
                first_name: "Other".to_owned(),
                ..client_input()
            })
            .unwrap();
        let mut input = budget_input(0);
        input.lines.push(BudgetLine {
            label: "Necessary cost".to_owned(),
            kind: budget::Kind::Necessary,
            amount: Some("1000".to_owned()),
            frequency: budget::Frequency::Monthly,
            estimated: false,
        });
        let budget = workspace.save_client_budget(&client.id, input).unwrap();
        let debt = workspace
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap();
        let saved = workspace
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(
            saved.summary.as_ref().unwrap().emergency_target.as_deref(),
            Some("3000.00")
        );
        assert_eq!(
            saved
                .summary
                .as_ref()
                .unwrap()
                .accessible_emergency
                .as_deref(),
            Some("1000.00")
        );
        assert_eq!(
            saved.summary.as_ref().unwrap().emergency.months_needed,
            Some(10)
        );
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), budget);
        assert_eq!(
            workspace.get_client_debt_plan(&client.id).unwrap().data,
            debt.data
        );
        assert!(workspace
            .get_client_savings_plan(&other.id)
            .unwrap()
            .data
            .is_none());
        assert!(workspace
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap_err()
            .contains("another window"));
        let mut invalid = savings_input(1);
        invalid.data.liquid_savings = Some("100".to_owned());
        assert!(workspace
            .save_client_savings_plan(&client.id, invalid)
            .is_err());
        assert_eq!(
            workspace.get_client_savings_plan(&client.id).unwrap().data,
            saved.data
        );
        workspace.lock_workspace();
        assert_eq!(
            workspace.get_client_savings_plan(&client.id).unwrap_err(),
            LOCKED
        );
        assert_eq!(
            workspace
                .save_client_savings_plan(&client.id, savings_input(1))
                .unwrap_err(),
            LOCKED
        );
        drop(workspace);
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(reopened.get_client_savings_plan(&client.id).unwrap(), saved);
        let mut update = budget_input(1);
        update.lines.push(BudgetLine {
            label: "Necessary cost".to_owned(),
            kind: budget::Kind::Necessary,
            amount: Some("2000".to_owned()),
            frequency: budget::Frequency::Monthly,
            estimated: false,
        });
        reopened.save_client_budget(&client.id, update).unwrap();
        let checked = reopened.get_client_savings_plan(&client.id).unwrap();
        assert_eq!(checked.context.budget_revision, 2);
        assert_eq!(checked.revision, 1);
        assert_eq!(
            checked.summary.unwrap().emergency_target.as_deref(),
            Some("6000.00")
        );
        let event_count: i64 = reopened
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event='savings_plan_saved'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn competing_savings_writes_and_failed_audit_preserve_the_saved_plan() {
        let (directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let mut other = Workspace::new(directory.path().to_path_buf());
        other.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(
            other.get_client_savings_plan(&client.id).unwrap().revision,
            0
        );
        let saved = workspace
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap();
        assert!(other
            .save_client_savings_plan(&client.id, savings_input(0))
            .unwrap_err()
            .contains("another window"));
        workspace.session.as_ref().unwrap().connection.execute_batch("CREATE TRIGGER reject_savings_audit BEFORE INSERT ON audit_events WHEN NEW.event='savings_plan_saved' BEGIN SELECT RAISE(ABORT,'Synthetic write failure'); END;").unwrap();
        let mut changed = savings_input(1);
        changed.data.emergency_contribution = Some("300".to_owned());
        assert!(workspace
            .save_client_savings_plan(&client.id, changed)
            .is_err());
        assert_eq!(other.get_client_savings_plan(&client.id).unwrap(), saved);
    }

    fn restore_version_four_schema(connection: &Connection) {
        connection.execute_batch(
            "DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices; DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state; DROP TABLE client_cashflow_plans; DROP TABLE client_savings_plans;
             CREATE TABLE audit_events_v4 (
                id TEXT PRIMARY KEY NOT NULL,organization_id TEXT NOT NULL,counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK(event IN ('workspace_created','client_created','client_updated','note_added','budget_saved','debt_plan_saved')),
                entity_id TEXT,created_at TEXT NOT NULL,
                FOREIGN KEY(organization_id) REFERENCES organizations(id),
                FOREIGN KEY(counselor_id,organization_id) REFERENCES counselors(id,organization_id)
             );
             INSERT INTO audit_events_v4 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v4 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events(organization_id,counselor_id,created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT,'Audit events are append-only'); END;
             PRAGMA user_version=4;"
        ).unwrap();
    }

    #[test]
    fn savings_schema_upgrade_and_rollback_preserve_all_previous_records() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic prior note")
            .unwrap();
        let budget = workspace
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        let debt = workspace
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap();
        let connection = &mut workspace.session.as_mut().unwrap().connection;
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        restore_version_four_schema(connection);
        connection
            .execute_batch("CREATE TABLE audit_events_v5(collision TEXT);")
            .unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            4
        );
        assert!(connection
            .prepare("SELECT * FROM client_savings_plans")
            .is_err());
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        connection
            .execute("DROP TABLE audit_events_v5", [])
            .unwrap();
        workspace.lock_workspace();
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), budget);
        assert_eq!(
            workspace.get_client_debt_plan(&client.id).unwrap().data,
            debt.data
        );
        let after: i64 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after, count);
        assert_eq!(
            workspace
                .save_client_savings_plan(&client.id, savings_input(0))
                .unwrap()
                .revision,
            1
        );
    }

    pub(super) fn debt_input(revision: i64) -> PlanInput {
        PlanInput {
            expected_revision: revision,
            data: debt::tests::fixture(),
        }
    }

    #[test]
    fn debt_plan_persists_and_is_scoped_without_modifying_budget() {
        let (directory, mut workspace, _info) = configured();
        let first = workspace.save_client(client_input()).unwrap();
        let second = workspace
            .save_client(ClientInput {
                first_name: "Other".to_owned(),
                ..client_input()
            })
            .unwrap();
        let budget = workspace
            .save_client_budget(&first.id, budget_input(0))
            .unwrap();
        let saved = workspace
            .save_client_debt_plan(&first.id, debt_input(0))
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(
            saved
                .comparison
                .as_ref()
                .unwrap()
                .avalanche
                .as_ref()
                .unwrap()
                .total_interest,
            "35.35"
        );
        assert_eq!(workspace.get_client_budget(&first.id).unwrap(), budget);
        assert_eq!(
            workspace.get_client_debt_plan(&second.id).unwrap().revision,
            0
        );
        assert!(workspace
            .get_client_debt_plan(&second.id)
            .unwrap()
            .data
            .is_none());
        assert!(workspace
            .save_client_debt_plan(&first.id, debt_input(0))
            .unwrap_err()
            .contains("another window"));
        let mut invalid = debt_input(1);
        invalid.data.accounts[0].apr = Some("-12".to_owned());
        assert!(workspace.save_client_debt_plan(&first.id, invalid).is_err());
        assert_eq!(
            workspace.get_client_debt_plan(&first.id).unwrap().data,
            saved.data
        );
        let connection = &workspace.session.as_ref().unwrap().connection;
        let events: Vec<String> = connection.prepare("SELECT event || coalesce(entity_id, '') FROM audit_events WHERE event = 'debt_plan_saved'").unwrap().query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(events, vec![format!("debt_plan_saved{}", first.id)]);
        workspace.lock_workspace();
        assert_eq!(
            workspace.get_client_debt_plan(&first.id).unwrap_err(),
            LOCKED
        );
        assert_eq!(
            workspace
                .save_client_debt_plan(&first.id, debt_input(1))
                .unwrap_err(),
            LOCKED
        );
        drop(workspace);
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.to_owned()).unwrap();
        let restored = reopened.get_client_debt_plan(&first.id).unwrap();
        assert_eq!(restored.data, saved.data);
        assert_eq!(restored.comparison, saved.comparison);
        assert_eq!(restored.budget_context.revision, 1);
        let mut changed_budget = budget_input(1);
        changed_budget.lines[0].amount = Some("1".to_owned());
        reopened
            .save_client_budget(&first.id, changed_budget)
            .unwrap();
        let checked = reopened.get_client_debt_plan(&first.id).unwrap();
        assert_eq!(checked.budget_context.revision, 2);
        assert!(checked
            .comparison
            .unwrap()
            .warnings
            .iter()
            .any(|w| w.contains("exceed")));
        assert_eq!(checked.revision, 1); // rechecking context does not rewrite debt facts
        let mut clear = debt_input(1);
        clear.data.accounts.clear();
        assert!(reopened
            .save_client_debt_plan(&first.id, clear)
            .unwrap()
            .comparison
            .unwrap()
            .snowball
            .is_none());
    }

    #[test]
    fn stale_debt_window_and_failed_audit_cannot_replace_saved_plan() {
        let (directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let mut other_window = Workspace::new(directory.path().to_path_buf());
        other_window.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(
            other_window
                .get_client_debt_plan(&client.id)
                .unwrap()
                .revision,
            0
        );
        let saved = workspace
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap();
        assert!(other_window
            .save_client_debt_plan(&client.id, debt_input(0))
            .unwrap_err()
            .contains("another window"));
        workspace.session.as_ref().unwrap().connection.execute_batch(
            "CREATE TRIGGER reject_debt_audit BEFORE INSERT ON audit_events WHEN NEW.event = 'debt_plan_saved'
             BEGIN SELECT RAISE(ABORT, 'Synthetic audit failure'); END;"
        ).unwrap();
        let mut changed = debt_input(1);
        changed.data.accounts[0].balance = Some("9999".to_owned());
        assert!(workspace
            .save_client_debt_plan(&client.id, changed)
            .is_err());
        let current = other_window.get_client_debt_plan(&client.id).unwrap();
        assert_eq!(current.data, saved.data);
        assert_eq!(current.revision, 1);
        let count: i64 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event='debt_plan_saved'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    fn restore_version_three_schema(connection: &Connection) {
        connection.execute_batch(
            "DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices; DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state; DROP TABLE client_cashflow_plans; DROP TABLE client_savings_plans; DROP TABLE client_debt_plans;
             CREATE TABLE audit_events_v3 (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK (event IN ('workspace_created', 'client_created', 'client_updated', 'note_added', 'budget_saved')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             INSERT INTO audit_events_v3 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v3 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events (organization_id, counselor_id, created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             PRAGMA user_version = 3;"
        ).unwrap();
    }

    #[test]
    fn debt_schema_upgrade_and_failed_migration_preserve_existing_records() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic pre-upgrade note")
            .unwrap();
        let budget = workspace
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        let connection = &mut workspace.session.as_mut().unwrap().connection;
        let before: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        restore_version_three_schema(connection);
        connection
            .execute_batch("CREATE TABLE audit_events_v4 (collision TEXT);")
            .unwrap();
        assert!(migrate(connection, false).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i32>(0))
                .unwrap(),
            3
        );
        assert!(connection
            .prepare("SELECT * FROM client_debt_plans")
            .is_err());
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        connection
            .execute("DROP TABLE audit_events_v4", [])
            .unwrap();
        workspace.lock_workspace();
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), budget);
        let connection = &workspace.session.as_ref().unwrap().connection;
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            before
        );
        assert!(connection
            .execute("UPDATE audit_events SET event='debt_plan_saved'", [])
            .is_err());
        assert_eq!(
            workspace
                .save_client_debt_plan(&client.id, debt_input(0))
                .unwrap()
                .revision,
            1
        );
    }

    pub(super) fn budget_input(revision: i64) -> BudgetInput {
        BudgetInput {
            expected_revision: revision,
            lines: vec![BudgetLine {
                label: "Synthetic take-home wages".to_owned(),
                kind: budget::Kind::Income,
                amount: Some("1500.00".to_owned()),
                frequency: budget::Frequency::Biweekly,
                estimated: false,
            }],
        }
    }

    #[test]
    fn budget_persistence_isolation_revisions_and_private_audit() {
        let (directory, mut workspace, _info) = configured();
        let first = workspace.save_client(client_input()).unwrap();
        let second = workspace
            .save_client(ClientInput {
                first_name: "Other".to_owned(),
                ..client_input()
            })
            .unwrap();
        assert_eq!(workspace.get_client_budget(&first.id).unwrap().revision, 0);
        let saved = workspace
            .save_client_budget(&first.id, budget_input(0))
            .unwrap();
        assert_eq!(saved.summary.monthly_income, "3250.00");
        assert_eq!(saved.revision, 1);
        assert!(workspace
            .get_client_budget(&second.id)
            .unwrap()
            .lines
            .is_empty());
        assert!(workspace
            .save_client_budget(&first.id, budget_input(0))
            .unwrap_err()
            .contains("another window"));
        let mut invalid = budget_input(1);
        invalid.lines[0].amount = Some("12.345".to_owned());
        assert!(workspace.save_client_budget(&first.id, invalid).is_err());
        assert_eq!(workspace.get_client_budget(&first.id).unwrap(), saved);
        let connection = &workspace.session.as_ref().unwrap().connection;
        let events: Vec<String> = connection.prepare("SELECT event || coalesce(entity_id, '') FROM audit_events WHERE event = 'budget_saved'").unwrap().query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(events, vec![format!("budget_saved{}", first.id)]);
        assert!(!events.join("").contains("Synthetic take-home wages"));
        workspace.lock_workspace();
        assert_eq!(workspace.get_client_budget(&first.id).unwrap_err(), LOCKED);
        assert_eq!(
            workspace
                .save_client_budget(&first.id, budget_input(1))
                .unwrap_err(),
            LOCKED
        );
        drop(workspace);
        let mut reopened = Workspace::new(directory.path().to_path_buf());
        reopened.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(reopened.get_client_budget(&first.id).unwrap(), saved);
        let replaced = reopened
            .save_client_budget(
                &first.id,
                BudgetInput {
                    expected_revision: 1,
                    lines: vec![],
                },
            )
            .unwrap();
        assert_eq!(replaced.revision, 2);
        assert!(replaced.lines.is_empty());
        assert_eq!(reopened.get_client_budget(&first.id).unwrap(), replaced);
        assert!(!fs::read(reopened.database_path())
            .unwrap()
            .windows(b"Synthetic take-home wages".len())
            .any(|w| w == b"Synthetic take-home wages"));
    }

    #[test]
    fn budget_and_audit_failure_roll_back_together() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let saved = workspace
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        workspace.session.as_ref().unwrap().connection.execute_batch(
            "CREATE TRIGGER reject_budget_audit BEFORE INSERT ON audit_events WHEN NEW.event = 'budget_saved'
             BEGIN SELECT RAISE(ABORT, 'Synthetic write failure'); END;"
        ).unwrap();
        let mut replacement = budget_input(1);
        replacement.lines[0].amount = Some("9999.99".to_owned());
        assert!(workspace
            .save_client_budget(&client.id, replacement)
            .is_err());
        assert_eq!(workspace.get_client_budget(&client.id).unwrap(), saved);
        let count: i64 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE event = 'budget_saved'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    fn restore_version_two_schema(connection: &Connection) {
        // Recreate the actual old audit constraint, not just its version number.
        connection.execute_batch(
            "DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices; DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state; DROP TABLE client_cashflow_plans; DROP TABLE client_savings_plans; DROP TABLE client_debt_plans; DROP TABLE client_budgets;
             CREATE TABLE audit_events_v2 (
                id TEXT PRIMARY KEY NOT NULL, organization_id TEXT NOT NULL, counselor_id TEXT NOT NULL,
                event TEXT NOT NULL CHECK (event IN ('workspace_created', 'client_created', 'client_updated', 'note_added')),
                entity_id TEXT, created_at TEXT NOT NULL,
                FOREIGN KEY (organization_id) REFERENCES organizations(id),
                FOREIGN KEY (counselor_id, organization_id) REFERENCES counselors(id, organization_id)
             );
             INSERT INTO audit_events_v2 SELECT * FROM audit_events;
             DROP TABLE audit_events;
             ALTER TABLE audit_events_v2 RENAME TO audit_events;
             CREATE INDEX audit_events_scope ON audit_events (organization_id, counselor_id, created_at);
             CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events
                BEGIN SELECT RAISE(ABORT, 'Audit events are append-only'); END;
             PRAGMA user_version = 2;"
        ).unwrap();
    }

    #[test]
    fn version_two_migration_preserves_notes_audit_and_supports_budgets() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic migration note")
            .unwrap();
        let count_before: i64 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        restore_version_two_schema(&workspace.session.as_ref().unwrap().connection);
        workspace.lock_workspace();
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
        let connection = &workspace.session.as_ref().unwrap().connection;
        let count_after: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_before, count_after);
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        assert!(connection
            .execute("UPDATE audit_events SET event = 'budget_saved'", [])
            .is_err());
        assert_eq!(
            workspace
                .save_client_budget(&client.id, budget_input(0))
                .unwrap()
                .revision,
            1
        );
    }

    #[test]
    fn failed_budget_migration_rolls_back_and_preserves_prior_records() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        let note = workspace
            .add_client_note(&client.id, "Synthetic rollback note")
            .unwrap();
        restore_version_two_schema(&workspace.session.as_ref().unwrap().connection);
        workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch("CREATE TABLE audit_events_v3 (collision TEXT);")
            .unwrap();
        let connection = &mut workspace.session.as_mut().unwrap().connection;
        assert!(migrate(connection, false).is_err());
        let version: i32 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 2);
        assert!(connection.prepare("SELECT * FROM client_budgets").is_err());
        assert!(connection.execute("DELETE FROM audit_events", []).is_err());
        connection
            .execute("DROP TABLE audit_events_v3", [])
            .unwrap();
        workspace.lock_workspace();
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
        assert_eq!(
            workspace
                .save_client_budget(&client.id, budget_input(0))
                .unwrap()
                .revision,
            1
        );
    }

    #[test]
    fn separate_desktop_connections_cannot_overwrite_a_stale_budget() {
        let (directory, mut first_window, _info) = configured();
        let client = first_window.save_client(client_input()).unwrap();
        let mut second_window = Workspace::new(directory.path().to_path_buf());
        second_window.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(
            second_window
                .get_client_budget(&client.id)
                .unwrap()
                .revision,
            0
        );
        let saved = first_window
            .save_client_budget(&client.id, budget_input(0))
            .unwrap();
        assert!(second_window
            .save_client_budget(&client.id, budget_input(0))
            .unwrap_err()
            .contains("another window"));
        assert_eq!(second_window.get_client_budget(&client.id).unwrap(), saved);
        let next = second_window
            .save_client_budget(&client.id, budget_input(1))
            .unwrap();
        assert_eq!(first_window.get_client_budget(&client.id).unwrap(), next);
    }

    #[test]
    fn version_one_migration_preserves_existing_clients() {
        let (_directory, mut workspace, _info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .execute_batch(
                "DROP TABLE IF EXISTS client_assessment_records; DROP TABLE IF EXISTS financial_fact_reviews; DROP TABLE IF EXISTS statement_balance_reviews; DROP TABLE recurring_review_invalidations; DROP TABLE recurring_review_history; DROP TABLE transaction_review_history; DROP TABLE transaction_choices; DROP TABLE import_records; DROP TABLE import_documents; DROP TABLE statement_accounts; DROP TABLE client_import_state; DROP TABLE client_cashflow_plans; DROP TABLE client_savings_plans; DROP TABLE client_debt_plans; DROP TABLE client_budgets; DROP TABLE client_notes; DROP TABLE audit_events; PRAGMA user_version = 1;",
            )
            .unwrap();
        workspace.lock_workspace();
        workspace.unlock_workspace(PASSWORD.to_owned()).unwrap();
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
        let version: i32 = workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let note = workspace
            .add_client_note(&client.id, "Synthetic migrated note")
            .unwrap();
        assert_eq!(workspace.list_client_notes(&client.id).unwrap(), vec![note]);
    }

    #[test]
    fn unsupported_schema_is_not_downgraded_or_overwritten() {
        let (_directory, mut workspace, _info) = configured();
        workspace
            .session
            .as_ref()
            .unwrap()
            .connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        workspace.lock_workspace();
        let original_database = fs::read(workspace.database_path()).unwrap();
        let original_metadata = fs::read(workspace.metadata_path()).unwrap();
        let error = workspace.unlock_workspace(PASSWORD.to_owned()).unwrap_err();
        assert!(error.contains("newer application"));
        assert!(workspace.session.is_none());
        assert_eq!(
            original_database,
            fs::read(workspace.database_path()).unwrap()
        );
        assert_eq!(
            original_metadata,
            fs::read(workspace.metadata_path()).unwrap()
        );
    }

    #[test]
    fn partial_and_corrupt_storage_is_preserved() {
        let directory = tempdir().unwrap();
        let mut workspace = Workspace::new(directory.path().to_path_buf());
        let orphaned_journal = directory.path().join(format!("{DATABASE_NAME}-journal"));
        fs::write(&orphaned_journal, b"Synthetic interrupted recovery file").unwrap();
        assert!(workspace.setup_status().is_err());
        assert_eq!(
            fs::read(&orphaned_journal).unwrap(),
            b"Synthetic interrupted recovery file"
        );
        fs::remove_file(orphaned_journal).unwrap();
        let existing = b"Synthetic existing incomplete database";
        fs::write(workspace.database_path(), existing).unwrap();
        assert!(workspace.setup_status().is_err());
        assert!(workspace
            .setup_workspace(SetupWorkspaceInput {
                organization_name: "Synthetic Organization".to_owned(),
                counselor_name: "Synthetic Counselor".to_owned(),
                password: PASSWORD.to_owned(),
            })
            .is_err());
        assert_eq!(fs::read(workspace.database_path()).unwrap(), existing);
        let (_directory, mut workspace, _info) = configured();
        workspace.lock_workspace();
        let mut damaged = fs::read(workspace.database_path()).unwrap();
        let last_byte = damaged.len() - 1;
        damaged[last_byte] ^= 0xff;
        fs::write(workspace.database_path(), &damaged).unwrap();
        assert!(workspace.unlock_workspace(PASSWORD.to_owned()).is_err());
        assert!(workspace.session.is_none());
        assert_eq!(fs::read(workspace.database_path()).unwrap(), damaged);
        let corrupt_metadata = b"{not valid workspace metadata}";
        fs::write(workspace.metadata_path(), corrupt_metadata).unwrap();
        assert!(workspace.setup_status().is_err());
        assert_eq!(
            fs::read(workspace.metadata_path()).unwrap(),
            corrupt_metadata
        );
    }

    #[test]
    fn setup_cannot_replace_an_existing_workspace() {
        let (_directory, mut workspace, info) = configured();
        let client = workspace.save_client(client_input()).unwrap();
        assert!(workspace
            .setup_workspace(SetupWorkspaceInput {
                organization_name: "Replacement Organization".to_owned(),
                counselor_name: "Replacement Counselor".to_owned(),
                password: PASSWORD.to_owned(),
            })
            .is_err());
        assert_eq!(workspace.workspace_info().unwrap(), info);
        assert_eq!(workspace.get_client(&client.id).unwrap(), client);
    }
}
