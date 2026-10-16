mod action_plans;
mod assessment_pdf;
mod assessments;
mod branding;
mod budget;
mod budget_methods;
mod card_facts;
mod card_final_policy;
mod card_review;
mod cards;
mod cashflow;
mod client_care;
mod completeness;
mod coverage_review;
mod dashboard;
mod debt;
mod document_collection;
mod final_reports;
mod financial_facts;
mod follow_up_overview;
mod imports;
mod income_periods;
mod models;
mod pdf;
#[cfg(test)]
mod pdf_fixture;
#[doc(hidden)]
pub mod pdf_process;
mod planning;
mod reconciliation;
mod recurring;
mod report_evidence;
mod report_intake;
mod report_presentation;
mod report_viewer;
mod saved_scenarios;
mod savings;
mod scenarios;
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
async fn get_assessment_evidence(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<report_evidence::Preview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_assessment_evidence(&client_id))
    })
    .await
    .map_err(|_| "Report evidence could not be loaded.".to_owned())?
}
#[tauri::command]
async fn review_client_action_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    assessment_id: String,
) -> Result<action_plans::Review, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| {
            w.review_client_action_plan(&client_id, &assessment_id)
        })
    })
    .await
    .map_err(|_| "Action plan review could not finish.".to_owned())?
}
#[tauri::command]
async fn list_client_action_plans(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<action_plans::Directory, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.list_client_action_plans(&client_id))
    })
    .await
    .map_err(|_| "Action plans could not be loaded.".to_owned())?
}
#[tauri::command]
async fn get_client_action_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<action_plans::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_client_action_plan(&client_id, &id))
    })
    .await
    .map_err(|_| "Action plan could not be loaded.".to_owned())?
}
#[tauri::command]
async fn save_client_action_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    input: action_plans::Input,
) -> Result<action_plans::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.save_client_action_plan(&client_id, input))
    })
    .await
    .map_err(|_| "Action plan could not be recorded.".to_owned())?
}
#[tauri::command]
async fn export_client_action_plan(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
    path: String,
) -> Result<assessment_pdf::Export, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| {
            w.export_client_action_plan(&client_id, &id, &path)
        })
    })
    .await
    .map_err(|_| "Client PDF export could not finish.".to_owned())?
}
#[tauri::command]
async fn choose_action_plan_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_client_action_plan(&client_id, &id))?;
        state
            .handle
            .dialog()
            .file()
            .add_filter("Client action plan PDF", &["pdf"])
            .set_file_name("client-action-plan.pdf")
            .blocking_save_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local PDF path.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "Client PDF destination selection could not finish.".to_owned())?
}
#[tauri::command]
async fn get_assessment_planning(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<planning::Plans, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_assessment_planning(&client_id))
    })
    .await
    .map_err(|_| "Saved planning context could not be loaded.".to_owned())?
}
#[tauri::command]
async fn list_assessment_records(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<assessments::Directory, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.list_assessment_records(&client_id))
    })
    .await
    .map_err(|_| "Assessment records could not be loaded.".to_owned())?
}
#[tauri::command]
async fn get_card_facts(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<card_facts::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || access(&state, |w| w.get_card_facts(&client_id)))
        .await
        .map_err(|_| "Card completeness could not finish.".to_owned())?
}
#[tauri::command]
async fn save_card_facts(
    state: State<'_, DesktopState>,
    client_id: String,
    input: card_facts::Input,
) -> Result<card_facts::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.save_card_facts(&client_id, input))
    })
    .await
    .map_err(|_| "Card completeness could not finish.".to_owned())?
}
#[tauri::command]
async fn get_card_review(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<card_review::Directory, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || access(&state, |w| w.get_card_review(&client_id)))
        .await
        .map_err(|_| "Card review could not finish.".to_owned())?
}
#[tauri::command]
async fn save_card_review(
    state: State<'_, DesktopState>,
    client_id: String,
    input: card_review::Input,
) -> Result<card_review::Directory, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.save_card_review(&client_id, input))
    })
    .await
    .map_err(|_| "Card review could not finish.".to_owned())?
}
#[tauri::command]
async fn preview_card_review(
    state: State<'_, DesktopState>,
    client_id: String,
    input: dashboard::Input,
) -> Result<card_review::Review, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.preview_card_review(&client_id, input))
    })
    .await
    .map_err(|_| "Card review could not finish.".to_owned())?
}
#[tauri::command]
async fn query_transaction_activity(
    state: State<'_, DesktopState>,
    client_id: String,
    input: storage::activity_store::Query,
) -> Result<storage::activity_store::Page, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.query_transaction_activity(&client_id, input))
    })
    .await
    .map_err(|_| "Activity search could not finish. Refresh activity.".to_owned())?
}
#[tauri::command]
async fn get_card_statements(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<cards::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_card_statements(&client_id))
    })
    .await
    .map_err(|_| "Card operation could not finish. Refresh before another attempt.".to_owned())?
}
#[tauri::command]
async fn stage_card_statement(
    state: State<'_, DesktopState>,
    client_id: String,
    input: cards::Stage,
) -> Result<cards::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.stage_card_statement(&client_id, input))
    })
    .await
    .map_err(|_| "Card operation could not finish. Refresh before another attempt.".to_owned())?
}
#[tauri::command]
async fn review_card_transaction(
    state: State<'_, DesktopState>,
    client_id: String,
    input: cards::ReviewInput,
) -> Result<cards::Snapshot, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.review_card_transaction(&client_id, input))
    })
    .await
    .map_err(|_| "Card operation could not finish. Refresh before another attempt.".to_owned())?
}
#[tauri::command]
async fn get_assessment_record(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<assessments::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_assessment_record(&client_id, &id))
    })
    .await
    .map_err(|_| "Assessment record could not be loaded.".to_owned())?
}
#[tauri::command]
async fn review_final_report(
    state: State<'_, DesktopState>,
    client_id: String,
    assessment_id: String,
) -> Result<final_reports::Preview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| {
            w.review_final_report(&client_id, &assessment_id)
        })
    })
    .await
    .map_err(|_| "Final report readiness could not be loaded.".to_owned())?
}
#[tauri::command]
async fn get_final_report(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<final_reports::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_final_report(&client_id, &id))
    })
    .await
    .map_err(|_| "Final report could not be loaded.".to_owned())?
}
#[tauri::command]
async fn approve_final_report(
    state: State<'_, DesktopState>,
    client_id: String,
    input: final_reports::Input,
) -> Result<final_reports::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.approve_final_report(&client_id, input))
    })
    .await
    .map_err(|_| "Final approval could not finish. Reopen readiness to check the stored result before another attempt.".to_owned())?
}
#[tauri::command]
async fn get_report_presentation(
    state: State<'_, DesktopState>,
    client_id: String,
    assessment_id: String,
) -> Result<report_presentation::Context, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| {
            w.get_report_presentation(&client_id, &assessment_id)
        })
    })
    .await
    .map_err(|_| "Report presentation could not load.".to_owned())?
}
#[tauri::command]
async fn export_report_presentation(
    state: State<'_, DesktopState>,
    client_id: String,
    input: report_presentation::ExportInput,
) -> Result<report_presentation::Export, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.export_report_presentation(&client_id, input))
    })
    .await
    .map_err(|_| {
        "Report PDF export could not finish. Check the destination before another attempt."
            .to_owned()
    })?
}
#[tauri::command]
async fn open_report_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    ticket: String,
) -> Result<report_viewer::Handoff, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || access(&state, |w| w.open_report_pdf(&client_id, &ticket)))
        .await.map_err(|_| "The PDF opening request could not finish. Check your viewer before trying again; no printing was confirmed.".to_owned())?
}
#[tauri::command]
async fn choose_report_presentation_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    assessment_id: String,
    kind: report_presentation::Kind,
    layout: report_presentation::Layout,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| {
            w.get_report_presentation(&client_id, &assessment_id)
        })?
        .selected(kind)?;
        let name = match (kind, layout) {
            (report_presentation::Kind::Internal, report_presentation::Layout::SummaryAppendix) => {
                "internal-counselor-summary.pdf"
            }
            (report_presentation::Kind::Final, report_presentation::Layout::SummaryAppendix) => {
                "approved-counselor-summary.pdf"
            }
            (report_presentation::Kind::Internal, report_presentation::Layout::Detailed) => {
                "internal-counselor-evidence.pdf"
            }
            (report_presentation::Kind::Final, report_presentation::Layout::Detailed) => {
                "approved-counselor-evidence.pdf"
            }
        };
        state
            .handle
            .dialog()
            .file()
            .add_filter("Counselor report PDF", &["pdf"])
            .set_file_name(name)
            .blocking_save_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local PDF path.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "PDF destination selection could not finish.".to_owned())?
}
#[tauri::command]
async fn export_final_report_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
    path: String,
) -> Result<assessment_pdf::Export, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| {
            w.export_final_report_pdf(&client_id, &id, &path)
        })
    })
    .await
    .map_err(|_| "Final PDF export could not finish.".to_owned())?
}
#[tauri::command]
async fn choose_final_report_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_final_report(&client_id, &id))?;
        state
            .handle
            .dialog()
            .file()
            .add_filter("Final counselor report PDF", &["pdf"])
            .set_file_name("final-counselor-report.pdf")
            .blocking_save_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local PDF path.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "PDF destination selection could not finish.".to_owned())?
}
#[tauri::command]
async fn choose_assessment_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_assessment_record(&client_id, &id))?;
        state
            .handle
            .dialog()
            .file()
            .add_filter("Internal assessment PDF", &["pdf"])
            .set_file_name("counselor-assessment.pdf")
            .blocking_save_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local PDF path.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "PDF destination selection could not finish.".to_owned())?
}
#[tauri::command]
async fn export_assessment_pdf(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
    path: String,
) -> Result<assessment_pdf::Export, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.export_assessment_pdf(&client_id, &id, &path))
    })
    .await
    .map_err(|_| "PDF export could not finish.".to_owned())?
}
#[tauri::command]
async fn save_assessment_record(
    state: State<'_, DesktopState>,
    client_id: String,
    input: assessments::Input,
) -> Result<assessments::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.save_assessment_record(&client_id, input))
    })
    .await
    .map_err(|_| "Assessment record could not be saved.".to_owned())?
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
fn get_coverage_review(
    state: State<'_, DesktopState>,
    client_id: String,
    revision: Option<i64>,
) -> Result<coverage_review::Record, String> {
    access(&state, |w| w.get_coverage_review(&client_id, revision))
}
#[tauri::command]
fn save_coverage_review(
    state: State<'_, DesktopState>,
    client_id: String,
    input: coverage_review::Input,
) -> Result<coverage_review::Record, String> {
    access(&state, |w| w.save_coverage_review(&client_id, input))
}
#[tauri::command]
fn get_financial_completeness(
    state: State<'_, DesktopState>,
    client_id: String,
    input: dashboard::Input,
) -> Result<completeness::Review, String> {
    access(&state, |w| w.get_financial_completeness(&client_id, input))
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
fn get_assessment_intake(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<report_intake::Preview, String> {
    access(&state, |w| w.get_assessment_intake(&client_id))
}

#[tauri::command]
fn get_firm_branding(state: State<'_, DesktopState>) -> Result<branding::Record, String> {
    access(&state, Workspace::get_firm_branding)
}
#[tauri::command]
fn save_firm_branding(
    state: State<'_, DesktopState>,
    input: branding::Input,
) -> Result<branding::Record, String> {
    access(&state, |w| w.save_firm_branding(input))
}
#[tauri::command]
fn inspect_branding_logo(state: State<'_, DesktopState>, path: String) -> Result<String, String> {
    access(&state, |w| w.inspect_branding_logo(path))
}
#[tauri::command]
async fn choose_branding_logo(state: State<'_, DesktopState>) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, Workspace::workspace_info)?;
        state
            .handle
            .dialog()
            .file()
            .add_filter("PNG logo", &["png"])
            .blocking_pick_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|_| "Choose a local PNG logo.".to_owned())
            })
            .transpose()
    })
    .await
    .map_err(|_| "Logo selection could not finish.".to_owned())?
}
#[tauri::command]
fn get_statement_accounts(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<storage::statement_accounts_store::Directory, String> {
    access(&state, |w| w.get_statement_accounts(&client_id))
}
#[tauri::command]
fn save_statement_account(
    state: State<'_, DesktopState>,
    client_id: String,
    input: storage::statement_accounts_store::Input,
) -> Result<storage::statement_accounts_store::Directory, String> {
    access(&state, |w| w.save_statement_account(&client_id, input))
}
#[tauri::command]
fn get_document_collection(
    state: State<'_, DesktopState>,
    client_id: String,
    revision: Option<i64>,
) -> Result<document_collection::Record, String> {
    access(&state, |w| w.get_document_collection(&client_id, revision))
}
#[tauri::command]
fn save_document_collection(
    state: State<'_, DesktopState>,
    client_id: String,
    input: document_collection::Input,
) -> Result<document_collection::Record, String> {
    access(&state, |w| w.save_document_collection(&client_id, input))
}
#[tauri::command]
fn get_client_care(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<client_care::Record, String> {
    access(&state, |w| w.get_client_care(&client_id))
}
#[tauri::command]
async fn get_follow_up_overview(
    state: State<'_, DesktopState>,
    input: follow_up_overview::Input,
) -> Result<follow_up_overview::Overview, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_follow_up_overview(input))
    })
    .await
    .map_err(|_| "The follow-up overview could not be loaded.".to_owned())?
}
#[tauri::command]
fn save_client_care(
    state: State<'_, DesktopState>,
    client_id: String,
    input: client_care::Input,
) -> Result<client_care::Record, String> {
    access(&state, |w| w.save_client_care(&client_id, input))
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
async fn list_client_scenarios(
    state: State<'_, DesktopState>,
    client_id: String,
) -> Result<saved_scenarios::Directory, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.list_client_scenarios(&client_id))
    })
    .await
    .map_err(|_| "Scenario history could not load.".to_owned())?
}
#[tauri::command]
async fn get_client_scenario(
    state: State<'_, DesktopState>,
    client_id: String,
    id: String,
) -> Result<saved_scenarios::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.get_client_scenario(&client_id, &id))
    })
    .await
    .map_err(|_| "Saved scenario could not load.".to_owned())?
}
#[tauri::command]
async fn save_client_scenario(
    state: State<'_, DesktopState>,
    client_id: String,
    input: saved_scenarios::Input,
) -> Result<saved_scenarios::Record, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.save_client_scenario(&client_id, input))
    })
    .await
    .map_err(|_| {
        "Scenario save could not finish. Reload scenario history before recording again.".to_owned()
    })?
}
#[tauri::command]
async fn compare_financial_scenarios(
    state: State<'_, DesktopState>,
    client_id: String,
    input: scenarios::Input,
) -> Result<scenarios::Comparison, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.compare_financial_scenarios(&client_id, input))
    })
    .await
    .map_err(|_| "Financial scenario comparison could not finish.".to_owned())?
}
#[tauri::command]
async fn compare_budget_methods(
    state: State<'_, DesktopState>,
    client_id: String,
    input: budget_methods::Input,
) -> Result<budget_methods::Comparison, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        access(&state, |w| w.compare_budget_methods(&client_id, input))
    })
    .await
    .map_err(|_| "Budget comparison could not finish.".to_owned())?
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
            get_financial_completeness,
            get_coverage_review,
            save_coverage_review,
            review_financial_facts,
            list_assessment_records,
            get_assessment_planning,
            get_assessment_evidence,
            get_assessment_intake,
            review_client_action_plan,
            list_client_action_plans,
            get_client_action_plan,
            save_client_action_plan,
            export_client_action_plan,
            choose_action_plan_pdf,
            get_assessment_record,
            get_card_facts,
            save_card_facts,
            get_card_review,
            save_card_review,
            preview_card_review,
            query_transaction_activity,
            get_card_statements,
            stage_card_statement,
            review_card_transaction,
            save_assessment_record,
            choose_assessment_pdf,
            review_final_report,
            get_final_report,
            approve_final_report,
            choose_final_report_pdf,
            export_final_report_pdf,
            get_report_presentation,
            choose_report_presentation_pdf,
            export_report_presentation,
            open_report_pdf,
            export_assessment_pdf,
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
            get_client_care,
            get_firm_branding,
            save_firm_branding,
            inspect_branding_logo,
            choose_branding_logo,
            get_statement_accounts,
            save_statement_account,
            get_document_collection,
            save_document_collection,
            get_follow_up_overview,
            save_client_care,
            add_client_note,
            list_client_notes,
            compare_budget_methods,
            compare_financial_scenarios,
            list_client_scenarios,
            get_client_scenario,
            save_client_scenario,
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
