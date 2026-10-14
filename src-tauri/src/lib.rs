mod budget;
mod cashflow;
mod dashboard;
mod debt;
mod financial_facts;
mod imports;
mod models;
mod pdf;
#[cfg(test)]
mod pdf_fixture;
#[doc(hidden)]
pub mod pdf_process;
mod reconciliation;
mod recurring;
mod savings;
mod storage;

use models::{Client, ClientInput, ClientNote, SetupStatus, SetupWorkspaceInput, WorkspaceInfo};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use storage::Workspace;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone)]
struct DesktopState {
    workspace: Arc<Mutex<Workspace>>,
    handle: tauri::AppHandle,
}

fn access<T>(
    state: &DesktopState,
    operation: impl FnOnce(&mut Workspace) -> Result<T, String>,
) -> Result<T, String> {
    let mut workspace = state
        .workspace
        .lock()
        .map_err(|_| "Workspace is unavailable. Restart the application.".to_string())?;
    if workspace.expire_if_idle() {
        let _ = state.handle.emit("workspace-locked", ());
    }
    operation(&mut workspace)
}

#[tauri::command]
async fn choose_client_csv(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_client(&client_id))?;
        state
            .handle
            .dialog()
            .file()
            .add_filter("UTF-8 CSV statements", &["csv"])
            .blocking_pick_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local CSV file.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "CSV selection could not finish.".to_owned())?
}
#[tauri::command]
async fn choose_client_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_client(&client_id))?;
        state
            .handle
            .dialog()
            .file()
            .add_filter("Selectable-text PDF statements", &["pdf"])
            .blocking_pick_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local PDF file.".into())
            })
            .transpose()
    })
    .await
    .map_err(|_| "PDF selection could not finish.".to_owned())?
}
#[tauri::command]
async fn inspect_client_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    path: String,
) -> Result<imports::Preview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.inspect_client_pdf(client_id, path))
    })
    .await
    .map_err(|_| "PDF inspection could not finish.".to_owned())?
}
#[tauri::command]
async fn inspect_client_csv(
    state: State<'_, DesktopState>,
    client_id: String,
    path: String,
) -> Result<imports::Preview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.inspect_client_csv(client_id, path))
    })
    .await
    .map_err(|_| "CSV inspection could not finish.".to_owned())?
}
#[tauri::command]
fn cancel_client_csv(state: State<'_, DesktopState>, token: String) -> Result<(), String> {
    access(&state, |w| {
        w.cancel_client_csv(&token);
        Ok(())
    })
}
#[tauri::command]
fn review_financial_facts(
    state: State<'_, DesktopState>,
    client_id: String,
    input: financial_facts::Input,
) -> Result<imports::Snapshot, String> {
    access(&state, |w| w.review_financial_facts(&client_id, input))
}
#[tauri::command]
fn get_client_dashboard(
    state: State<'_, DesktopState>,
    client_id: String,
    input: dashboard::Input,
) -> Result<dashboard::Assessment, String> {
    access(&state, |w| w.get_client_dashboard(&client_id, input))
}
#[tauri::command]
fn get_client_imports(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<imports::Snapshot, String> {
    access(&state, |w| w.get_client_imports(&client_id))
}
#[tauri::command]
async fn stage_client_csv(
    state: State<'_, DesktopState>,
    client_id: String,
    input: imports::StageInput,
) -> Result<imports::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.stage_client_csv(&client_id, input))
    })
    .await
    .map_err(|_| "Import/review operation could not finish.".to_owned())?
}
#[tauri::command]
async fn review_statement_balances(
    state: State<'_, DesktopState>,
    client_id: String,
    input: reconciliation::ReviewInput,
) -> Result<imports::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.review_statement_balances(&client_id, input))
    })
    .await
    .map_err(|_| "Statement balance review could not finish.".to_owned())?
}
#[tauri::command]
async fn review_recurring_payment(
    state: State<'_, DesktopState>,
    client_id: String,
    input: recurring::ReviewInput,
) -> Result<imports::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.review_recurring_payment(&client_id, input))
    })
    .await
    .map_err(|_| "Payment review could not finish.".to_owned())?
}
#[tauri::command]
async fn review_transaction(
    state: State<'_, DesktopState>,
    client_id: String,
    input: imports::ReviewInput,
) -> Result<imports::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.review_transaction(&client_id, input))
    })
    .await
    .map_err(|_| "Import/review operation could not finish.".to_owned())?
}
#[tauri::command]
async fn bulk_approve_transactions(
    state: State<'_, DesktopState>,
    client_id: String,
    input: imports::BulkInput,
) -> Result<imports::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.bulk_approve_transactions(&client_id, input))
    })
    .await
    .map_err(|_| "Import/review operation could not finish.".to_owned())?
}
#[tauri::command]
async fn pair_transfer_transactions(
    state: State<'_, DesktopState>,
    client_id: String,
    input: imports::PairInput,
) -> Result<imports::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.pair_transfer_transactions(&client_id, input))
    })
    .await
    .map_err(|_| "Import/review operation could not finish.".to_owned())?
}

#[tauri::command]
async fn choose_backup_folder(state: State<'_, DesktopState>) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.workspace_info())?;
        state
            .handle
            .dialog()
            .file()
            .blocking_pick_folder()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local or external-drive folder.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "Folder selection could not finish.".to_owned())?
}
#[tauri::command]
async fn choose_restore_file(state: State<'_, DesktopState>) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state
            .handle
            .dialog()
            .file()
            .add_filter("Encrypted workspace backup", &["ebcbackup"])
            .blocking_pick_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local encrypted backup file.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "Backup selection could not finish.".to_owned())?
}
#[tauri::command]
async fn create_workspace_backup(
    state: State<'_, DesktopState>,
    directory: String,
    password: String,
) -> Result<storage::backup::BackupResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.create_backup(directory, password))
    })
    .await
    .map_err(|_| "Backup creation could not finish.".to_owned())?
}
#[tauri::command]
async fn prepare_workspace_restore(
    state: State<'_, DesktopState>,
    path: String,
    password: String,
) -> Result<storage::backup::RestoreReview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.prepare_restore(path, password))
    })
    .await
    .map_err(|_| "Backup verification could not finish.".to_owned())?
}
#[tauri::command]
async fn confirm_workspace_restore(
    state: State<'_, DesktopState>,
    token: String,
    confirmation: String,
) -> Result<storage::backup::RestoreResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = access(&state, |w| w.confirm_restore(token, confirmation))?;
        Ok(result)
    })
    .await
    .map_err(|_| "Recovery could not finish.".to_owned())?
}
#[tauri::command]
fn cancel_workspace_restore(state: State<'_, DesktopState>, token: String) -> Result<(), String> {
    access(&state, |w| {
        w.cancel_restore(&token);
        Ok(())
    })
}

#[tauri::command]
fn setup_status(state: State<'_, DesktopState>) -> Result<SetupStatus, String> {
    access(&state, Workspace::setup_status)
}

#[tauri::command]
async fn setup_workspace(
    state: State<'_, DesktopState>,
    input: SetupWorkspaceInput,
) -> Result<WorkspaceInfo, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |workspace| workspace.setup_workspace(input))
    })
    .await
    .map_err(|_| "Workspace setup could not finish.".to_string())?
}

#[tauri::command]
async fn unlock_workspace(
    state: State<'_, DesktopState>,
    password: String,
) -> Result<WorkspaceInfo, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |workspace| workspace.unlock_workspace(password))
    })
    .await
    .map_err(|_| "Workspace could not be unlocked.".to_string())?
}

#[tauri::command]
fn lock_workspace(state: State<'_, DesktopState>) -> Result<(), String> {
    access(&state, |workspace| {
        workspace.lock_workspace();
        let _ = state.handle.emit("workspace-locked", ());
        Ok(())
    })
}

#[tauri::command]
fn workspace_info(state: State<'_, DesktopState>) -> Result<WorkspaceInfo, String> {
    access(&state, Workspace::workspace_info)
}

#[tauri::command]
fn list_clients(state: State<'_, DesktopState>) -> Result<Vec<Client>, String> {
    access(&state, Workspace::list_clients)
}

#[tauri::command]
fn get_client(state: State<'_, DesktopState>, id: String) -> Result<Client, String> {
    access(&state, |workspace| workspace.get_client(&id))
}

#[tauri::command]
fn save_client(state: State<'_, DesktopState>, input: ClientInput) -> Result<Client, String> {
    access(&state, |workspace| workspace.save_client(input))
}

#[tauri::command]
fn add_client_note(
    state: State<'_, DesktopState>,
    client_id: String,
    body: String,
) -> Result<ClientNote, String> {
    access(&state, |workspace| {
        workspace.add_client_note(&client_id, &body)
    })
}

#[tauri::command]
fn list_client_notes(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<Vec<ClientNote>, String> {
    access(&state, |workspace| workspace.list_client_notes(&client_id))
}

#[tauri::command]
fn get_client_budget(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<budget::ClientBudget, String> {
    access(&state, |workspace| workspace.get_client_budget(&client_id))
}

#[tauri::command]
fn save_client_budget(
    state: State<'_, DesktopState>,
    client_id: String,
    input: budget::BudgetInput,
) -> Result<budget::ClientBudget, String> {
    access(&state, |workspace| {
        workspace.save_client_budget(&client_id, input)
    })
}

#[tauri::command]
fn get_client_debt_plan(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<debt::ClientDebtPlan, String> {
    access(&state, |workspace| {
        workspace.get_client_debt_plan(&client_id)
    })
}
#[tauri::command]
fn save_client_debt_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    input: debt::PlanInput,
) -> Result<debt::ClientDebtPlan, String> {
    access(&state, |workspace| {
        workspace.save_client_debt_plan(&client_id, input)
    })
}

#[tauri::command]
fn get_client_cashflow_plan(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<cashflow::ClientCashflowPlan, String> {
    access(&state, |workspace| {
        workspace.get_client_cashflow_plan(&client_id)
    })
}
#[tauri::command]
fn preview_client_cashflow_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    input: cashflow::PlanInput,
) -> Result<cashflow::ClientCashflowPlan, String> {
    access(&state, |workspace| {
        workspace.preview_client_cashflow_plan(&client_id, input)
    })
}
#[tauri::command]
fn save_client_cashflow_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    input: cashflow::PlanInput,
) -> Result<cashflow::ClientCashflowPlan, String> {
    access(&state, |workspace| {
        workspace.save_client_cashflow_plan(&client_id, input)
    })
}

#[tauri::command]
fn get_client_savings_plan(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<savings::ClientSavingsPlan, String> {
    access(&state, |workspace| {
        workspace.get_client_savings_plan(&client_id)
    })
}
#[tauri::command]
fn save_client_savings_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    input: savings::PlanInput,
) -> Result<savings::ClientSavingsPlan, String> {
    access(&state, |workspace| {
        workspace.save_client_savings_plan(&client_id, input)
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            let state = DesktopState {
                workspace: Arc::new(Mutex::new(Workspace::new(directory))),
                handle: app.handle().clone(),
            };
            app.manage(state.clone());
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(15));
                if let Ok(mut workspace) = state.workspace.lock() {
                    if workspace.expire_if_idle() {
                        let _ = state.handle.emit("workspace-locked", ());
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            choose_client_csv,
            choose_client_pdf,
            inspect_client_pdf,
            inspect_client_csv,
            cancel_client_csv,
            get_client_imports,
            get_client_dashboard,
            review_financial_facts,
            stage_client_csv,
            review_transaction,
            review_recurring_payment,
            review_statement_balances,
            bulk_approve_transactions,
            pair_transfer_transactions,
            choose_backup_folder,
            choose_restore_file,
            create_workspace_backup,
            prepare_workspace_restore,
            confirm_workspace_restore,
            cancel_workspace_restore,
            setup_status,
            setup_workspace,
            unlock_workspace,
            lock_workspace,
            workspace_info,
            list_clients,
            get_client,
            save_client,
            add_client_note,
            list_client_notes,
            get_client_budget,
            save_client_budget,
            get_client_debt_plan,
            save_client_debt_plan,
            get_client_cashflow_plan,
            preview_client_cashflow_plan,
            save_client_cashflow_plan,
            get_client_savings_plan,
            save_client_savings_plan
        ])
        .run(tauri::generate_context!())
        .expect("Desktop application could not start");
}

/// Private parser process entry: exits before workspace/Tauri initialization.
pub fn pdf_worker_entry() {
    pdf::worker();
}
