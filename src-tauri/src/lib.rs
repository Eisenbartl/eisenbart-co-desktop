mod budget;
mod debt;
mod models;
mod savings;
mod storage;

use models::{Client, ClientInput, ClientNote, SetupStatus, SetupWorkspaceInput, WorkspaceInfo};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use storage::Workspace;
use tauri::{Emitter, Manager, State};

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
            get_client_savings_plan,
            save_client_savings_plan
        ])
        .run(tauri::generate_context!())
        .expect("Desktop application could not start");
}
