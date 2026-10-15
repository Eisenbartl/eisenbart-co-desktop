import { invoke } from '@tauri-apps/api/core';
import type { SavedScenario, ScenarioDirectory, ScenarioInput, ScenarioComparison, BudgetMethodInput, BudgetMethodComparison, AssessmentEvidencePreview, ActionPlanReview, ActionPlanInput, SavedActionPlan, ActionPlanDirectory, AssessmentPlans, AssessmentDirectory, SavedAssessment, AssessmentRecordInput, FinancialFactsInput, FinancialAssessment, StatementBalanceInput,RecurringReviewInput,ClientImports,CsvPreview,CsvStageInput,TransactionReviewInput,TransactionBulkInput,TransactionPairInput, BackupResult, RestoreReview, RestoreResult, ClientCashflowPlan, CashflowPlanInput, ClientSavingsPlan, SavingsPlanInput, ClientDebtPlan, DebtPlanInput, BudgetInput, ClientBudget, Client, ClientInput, ClientNote, SetupInput, WorkspaceInfo } from './types';

export function desktopAvailable(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

let sessionEpoch = 0;
export function resetApiSession(): void {
  sessionEpoch += 1;
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const requestEpoch = sessionEpoch;
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    const text = error instanceof Error ? error.message : String(error);
    if (requestEpoch === sessionEpoch && /workspace is locked/i.test(text)) window.dispatchEvent(new Event('workspace-access-expired'));
    throw error;
  }
}

export const api = {
  getDocumentCollection:(clientId:string,revision:number|null=null)=>command<import('./types').DocumentCollection>('get_document_collection',{clientId,revision}),
  saveDocumentCollection:(clientId:string,input:{expectedRevision:number;items:import('./types').DocumentRequest[]})=>command<import('./types').DocumentCollection>('save_document_collection',{clientId,input}),
  getAssessmentIntake:(clientId:string)=>command<import('./types').ReportIntakePreview>('get_assessment_intake',{clientId}),
  getFollowUpOverview:(input:import('./types').FollowUpQuery)=>command<import('./types').FollowUpOverviewData>('get_follow_up_overview',{input}),
  getClientCare:(clientId:string)=>command<import('./types').ClientCareRecord>('get_client_care',{clientId}),
  saveClientCare:(clientId:string,input:{expectedRevision:number;data:import('./types').ClientCareData})=>command<import('./types').ClientCareRecord>('save_client_care',{clientId,input}),
  listClientScenarios:(clientId:string)=>command<ScenarioDirectory>('list_client_scenarios',{clientId}),
  getClientScenario:(clientId:string,id:string)=>command<SavedScenario>('get_client_scenario',{clientId,id}),
  saveClientScenario:(clientId:string,input:{expectedCount:number;proposal:ScenarioInput})=>command<SavedScenario>('save_client_scenario',{clientId,input}),
  compareFinancialScenarios:(clientId:string,input:ScenarioInput)=>command<ScenarioComparison>('compare_financial_scenarios',{clientId,input}),
  compareBudgetMethods:(clientId:string,input:BudgetMethodInput)=>command<BudgetMethodComparison>('compare_budget_methods',{clientId,input}),
  getAssessmentEvidence:(clientId:string)=>command<AssessmentEvidencePreview>('get_assessment_evidence',{clientId}),
  reviewClientActionPlan: (clientId:string,assessmentId:string)=>command<ActionPlanReview>('review_client_action_plan',{clientId,assessmentId}),
  listClientActionPlans: (clientId:string)=>command<ActionPlanDirectory>('list_client_action_plans',{clientId}),
  getClientActionPlan: (clientId:string,id:string)=>command<SavedActionPlan>('get_client_action_plan',{clientId,id}),
  saveClientActionPlan: (clientId:string,input:ActionPlanInput)=>command<SavedActionPlan>('save_client_action_plan',{clientId,input}),
  chooseActionPlanPdf: (clientId:string,id:string)=>command<string|null>('choose_action_plan_pdf',{clientId,id}),
  exportClientActionPlan: (clientId:string,id:string,path:string)=>command<{path:string;pages:number;recordId:string}>('export_client_action_plan',{clientId,id,path}),
  getAssessmentPlanning: (clientId:string)=>command<AssessmentPlans>('get_assessment_planning',{clientId}),
  chooseAssessmentPdf: (clientId:string,id:string)=>command<string|null>('choose_assessment_pdf',{clientId,id}),
  exportAssessmentPdf: (clientId:string,id:string,path:string)=>command<{path:string;pages:number;recordId:string}>('export_assessment_pdf',{clientId,id,path}),
  listAssessmentRecords: (clientId:string)=>command<AssessmentDirectory>('list_assessment_records',{clientId}),
  getAssessmentRecord: (clientId:string,id:string)=>command<SavedAssessment>('get_assessment_record',{clientId,id}),
  saveAssessmentRecord: (clientId:string,input:AssessmentRecordInput)=>command<SavedAssessment>('save_assessment_record',{clientId,input}),
  reviewFinancialFacts: (clientId:string,input:FinancialFactsInput) => command<ClientImports>('review_financial_facts',{clientId,input}),
  getClientDashboard: (clientId:string,input:{fromMonth:string;throughMonth:string}) => command<FinancialAssessment>('get_client_dashboard',{clientId,input}),
  chooseClientPdf: (clientId:string) => command<string|null>('choose_client_pdf',{clientId}),
  inspectClientPdf: (clientId:string,path:string) => command<CsvPreview>('inspect_client_pdf',{clientId,path}),
  chooseClientCsv: (clientId:string) => command<string|null>('choose_client_csv',{clientId}),
  inspectClientCsv: (clientId:string,path:string) => command<CsvPreview>('inspect_client_csv',{clientId,path}),
  cancelClientCsv: (token:string) => command<void>('cancel_client_csv',{token}),
  reviewRecurringPayment: (clientId:string,input:RecurringReviewInput) => command<ClientImports>('review_recurring_payment',{clientId,input}),
  reviewStatementBalances: (clientId:string,input:StatementBalanceInput) => command<ClientImports>('review_statement_balances',{clientId,input}),
  getClientImports: (clientId:string) => command<ClientImports>('get_client_imports',{clientId}),
  stageClientCsv: (clientId:string,input:CsvStageInput) => command<ClientImports>('stage_client_csv',{clientId,input}),
  reviewTransaction: (clientId:string,input:TransactionReviewInput) => command<ClientImports>('review_transaction',{clientId,input}),
  bulkApproveTransactions: (clientId:string,input:TransactionBulkInput) => command<ClientImports>('bulk_approve_transactions',{clientId,input}),
  pairTransferTransactions: (clientId:string,input:TransactionPairInput) => command<ClientImports>('pair_transfer_transactions',{clientId,input}),
  chooseBackupFolder: () => command<string | null>('choose_backup_folder'),
  chooseRestoreFile: () => command<string | null>('choose_restore_file'),
  createWorkspaceBackup: (directory: string, password: string) => command<BackupResult>('create_workspace_backup', {directory,password}),
  prepareWorkspaceRestore: (path: string, password: string) => command<RestoreReview>('prepare_workspace_restore', {path,password}),
  confirmWorkspaceRestore: (token: string, confirmation: string) => command<RestoreResult>('confirm_workspace_restore', {token,confirmation}),
  cancelWorkspaceRestore: (token: string) => command<void>('cancel_workspace_restore', {token}),
  getClientCashflowPlan: (clientId: string) => command<ClientCashflowPlan>('get_client_cashflow_plan', {clientId}),
  previewClientCashflowPlan: (clientId: string, input: CashflowPlanInput) => command<ClientCashflowPlan>('preview_client_cashflow_plan', {clientId,input}),
  saveClientCashflowPlan: (clientId: string, input: CashflowPlanInput) => command<ClientCashflowPlan>('save_client_cashflow_plan', {clientId,input}),
  getClientSavingsPlan: (clientId: string) => command<ClientSavingsPlan>('get_client_savings_plan', {clientId}),
  saveClientSavingsPlan: (clientId: string, input: SavingsPlanInput) => command<ClientSavingsPlan>('save_client_savings_plan', {clientId,input}),
  getClientDebtPlan: (clientId: string) => command<ClientDebtPlan>('get_client_debt_plan', { clientId }),
  saveClientDebtPlan: (clientId: string, input: DebtPlanInput) => command<ClientDebtPlan>('save_client_debt_plan', { clientId, input }),
  getClientBudget: (clientId: string) => command<ClientBudget>('get_client_budget', { clientId }),
  saveClientBudget: (clientId: string, input: BudgetInput) => command<ClientBudget>('save_client_budget', { clientId, input }),
  setupStatus: () => command<{ configured: boolean }>('setup_status'),
  setupWorkspace: (input: SetupInput) => command<WorkspaceInfo>('setup_workspace', { input }),
  unlockWorkspace: (password: string) => command<WorkspaceInfo>('unlock_workspace', { password }),
  lockWorkspace: () => command<void>('lock_workspace'),
  workspaceInfo: () => command<WorkspaceInfo>('workspace_info'),
  listClients: () => command<Client[]>('list_clients'),
  getClient: (id: string) => command<Client>('get_client', { id }),
  saveClient: (input: ClientInput) => command<Client>('save_client', { input }),
  addClientNote: (clientId: string, body: string) => command<ClientNote>('add_client_note', { clientId, body }),
  listClientNotes: (clientId: string) => command<ClientNote[]>('list_client_notes', { clientId }),
};
