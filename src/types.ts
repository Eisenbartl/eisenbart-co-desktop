export interface WorkspaceInfo {
  organizationId: string;
  organizationName: string;
  counselorId: string;
  counselorName: string;
}

export const CLIENT_STATUSES = [
  'New Intake',
  'Information Collection',
  'Financial Review',
  'Planning',
  'Action Plan',
  'Follow-Up',
  'Archived',
] as const;

export type ClientStatus = (typeof CLIENT_STATUSES)[number];

export interface Client {
  id: string;
  organizationId: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  householdSize: number;
  status: ClientStatus;
  goals: string;
  createdAt: string;
  updatedAt: string;
}

export interface ClientInput {
  id?: string | null;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  householdSize: number;
  status: ClientStatus;
  goals: string;
}

export interface ClientNote {
  id: string;
  clientId: string;
  counselorId: string;
  counselorName: string;
  body: string;
  createdAt: string;
}

export interface ClientIntake {
  address: string; household: string; employment: string; employer: string;
  occupation: string; incomeNotes: string;
}
export interface ClientFollowUp {
  id: string; kind: 'task' | 'appointment'; title: string; dueDate: string;
  time: string; status: 'open' | 'completed' | 'cancelled'; notes: string;
}
export interface ClientCareData { intake: ClientIntake; followUps: ClientFollowUp[] }
export interface ClientCareRecord {
  clientId: string; revision: number; updatedAt: string | null; reviewDate: string;
  overdue: number; dueToday: number; data: ClientCareData;
}

export interface FollowUpQuery {
  view: 'open'|'overdue'|'today'|'upcoming'|'closed'; kind: 'all'|'task'|'appointment';
  includeArchived: boolean; search: string; page: number;
}
export interface FollowUpItem {
  clientId: string; clientName: string; clientStatus: string; followUpId: string;
  kind: 'task'|'appointment'; title: string; dueDate: string; time: string;
  status: 'open'|'completed'|'cancelled'; careRevision: number;
}
export interface FollowUpOverviewData {
  organizationId: string; counselorId: string; reviewDate: string; timeZone: string;
  input: FollowUpQuery; overdue: number; dueToday: number; upcoming: number; closed: number;
  total: number; pageSize: number; items: FollowUpItem[];
}

export interface SetupInput {
  organizationName: string;
  counselorName: string;
  password: string;
}

export type BudgetKind = 'income' | 'necessary' | 'discretionary' | 'savings' | 'debtPayment';
export type BudgetFrequency = 'weekly' | 'biweekly' | 'semimonthly' | 'monthly' | 'annual' | 'irregular';
export interface BudgetLine {
  label: string;
  kind: BudgetKind;
  amount: string | null;
  frequency: BudgetFrequency;
  estimated: boolean;
}
export interface BudgetSummary {
  monthlyIncome: string;
  necessaryExpenses: string;
  discretionaryExpenses: string;
  totalSpending: string;
  surplusBeforeAllocations: string;
  proposedSavings: string;
  availableForDebt: string;
  plannedDebtPayments: string;
  unallocated: string;
  monthlyAmounts: (string | null)[];
  missingCount: number;
  estimatedCount: number;
  warnings: string[];
}
export interface ClientBudget {
  clientId: string;
  revision: number;
  updatedAt: string | null;
  lines: BudgetLine[];
  summary: BudgetSummary;
}
export interface BudgetInput {
  expectedRevision: number;
  lines: BudgetLine[];
}

export type DebtType = 'creditCard' | 'autoLoan' | 'studentLoan' | 'personalLoan' | 'mortgage' | 'other' | 'custom';
export type InterestModel = 'unknown' | 'fixedMonthly' | 'daily' | 'variable' | 'promotional';
export interface DebtAccount {
  label: string; debtType: DebtType; balance: string | null; apr: string | null;
  minimumPayment: string | null; monthlyFee: string | null; interestModel: InterestModel;
  verified: boolean; termsNote: string;
}
export interface DatedAmount { month: string; amount: string }
export interface DebtPlanData {
  startMonth: string; monthlyPayment: string | null; accounts: DebtAccount[];
  extras: DatedAmount[]; paymentChanges: DatedAmount[];
}
export interface DebtPlanInput { expectedRevision: number; data: DebtPlanData }
export interface DebtMonth {
  month: string; availablePayment: string; payments: string[]; interest: string[];
  fees: string[]; balances: string[]; totalPayment: string; remainingBalance: string;
}
export interface DebtProjection {
  status: 'paidOff' | 'insufficientBudget' | 'horizonReached' | 'modelLimit';
  payoffMonth: string | null; accountPayoffMonths: (string | null)[];
  totalInterest: string; totalFees: string; totalPaid: string; remainingBalance: string;
  negativeAmortizationAccounts: number[]; warnings: string[]; schedule: DebtMonth[];
}
export interface DebtComparison { warnings: string[]; snowball: DebtProjection | null; avalanche: DebtProjection | null }
export interface DebtBudgetContext { revision: number; monthlyCapacity: string; plannedDebtPayments: string; warnings: string[] }
export interface ClientDebtPlan {
  clientId: string; revision: number; updatedAt: string | null; data: DebtPlanData | null;
  comparison: DebtComparison | null; budgetContext: DebtBudgetContext;
}

export interface SavingsGoal {
  label: string; targetAmount: string | null; targetDate: string | null;
  reservedAmount: string | null; monthlyContribution: string | null;
}
export interface SavingsPlanData {
  startMonth: string; targetMonths: number; liquidSavings: string | null;
  restrictedSavings: string | null; obligationsReserved: string | null;
  emergencyContribution: string | null; alternativeContributions: string[];
  goals: SavingsGoal[]; prioritiesNote: string;
}
export interface SavingsPlanInput { expectedRevision: number; data: SavingsPlanData }
export interface SavingsContext {
  budgetRevision: number; debtRevision: number; necessaryExpenses: string | null;
  monthlySurplus: string | null; budgetSavings: string | null; budgetDebtPayments: string | null;
  debtStartMonth: string | null; debtMonthlyPayment: string | null; debtChanges: DatedAmount[];
  debtExtras: DatedAmount[]; warnings: string[];
}
export interface SavingsForecast {
  status: 'funded' | 'projected' | 'noContribution' | 'unknown' | 'horizonReached';
  remaining: string | null; contribution: string | null; monthsNeeded: number | null; completionDate: string | null;
}
export interface SavingsGoalResult {
  suggestedContribution: string | null; contributionMonthsBeforeDue: number | null;
  onTrack: boolean | null; forecast: SavingsForecast; warnings: string[];
}
export interface SavingsMonth {
  month: string; emergencyContribution: string; goalContributions: string[];
  totalSavings: string; debtReserved: string | null; unallocated: string | null;
}
export interface SavingsSummary {
  accessibleEmergency: string | null; obligationsReserved: string | null;
  sinkingFundsReserved: string | null; restrictedSavings: string | null;
  coverageMonths: string | null; emergencyTarget: string | null; emergency: SavingsForecast;
  alternatives: { forecast: SavingsForecast; allocationWarning: boolean | null }[];
  goals: SavingsGoalResult[]; requestedMonthlySavings: string | null;
  warnings: string[]; schedule: SavingsMonth[];
}
export interface ClientSavingsPlan {
  clientId: string; revision: number; updatedAt: string | null; data: SavingsPlanData | null;
  summary: SavingsSummary | null; context: SavingsContext;
}

export interface CashflowAccount { id: string; label: string; openingBalance: string | null }
export type CashflowKind = 'paycheck' | 'bill' | 'loan' | 'savings' | 'otherInflow' | 'otherOutflow' | 'transfer';
export type CashflowFrequency = 'once' | 'weekly' | 'biweekly' | 'semimonthly' | 'monthly';
export interface CashflowSchedule {
  label: string; accountId: string; destinationId: string | null; kind: CashflowKind; amount: string | null;
  firstDate: string; frequency: CashflowFrequency; secondDay: number | null; untilDate: string | null;
  certainty: 'confirmed' | 'estimated';
}
export interface CashflowPlanData { startDate: string; endDate: string; accounts: CashflowAccount[]; schedules: CashflowSchedule[]; notes: string }
export interface CashflowPlanInput { expectedRevision: number; data: CashflowPlanData }
export interface CashflowContext {
  budgetRevision: number; debtRevision: number; savingsRevision: number; monthlySurplus: string | null;
  budgetDebt: string | null; budgetSavings: string | null; debtMonthlyPayment: string | null; savingsMonthlyContribution: string | null;
}
export interface CashflowBalance { accountId: string; amount: string | null }
export interface CashflowEntry {
  scheduleIndex: number; date: string; label: string; kind: CashflowKind; certainty: 'confirmed' | 'estimated';
  accountId: string; destinationId: string | null; amount: string | null; sourceBalance: string | null;
  destinationBalance: string | null; combinedBalance: string | null; nextPaycheck: string | null; warnings: string[];
}
export interface CashflowDay { date: string; balances: CashflowBalance[]; combinedBalance: string | null; entryIndices: number[]; warnings: string[] }
export interface CashflowSummary { ledger: CashflowEntry[]; days: CashflowDay[]; closingBalances: CashflowBalance[]; combinedClosing: string | null; warnings: string[] }
export interface ClientCashflowPlan { clientId: string; revision: number; updatedAt: string | null; data: CashflowPlanData | null; summary: CashflowSummary | null; context: CashflowContext }

export interface BackupResult { path: string; createdAt: string; auditRecorded: boolean; durabilityWarning: boolean }
export interface RestoreReview { token: string; organizationName: string; counselorName: string; clientCount: number; createdAt: string; sourceSchema: number; currentWorkspacePresent: boolean }
export interface RestoreResult { previousWorkspacePreserved: boolean; durabilityWarning: boolean }

export type TransactionKind='unclassified'|'income'|'expense'|'transfer'|'refund'|'fee'|'interest'|'balanceMovement';
export type ReviewStatus='Pending'|'Needs Clarification'|'Approved'|'Rejected';
export interface CsvMapping {date:number;description:number;amountMode:'signed'|'debitCredit';amount:number|null;debit:number|null;credit:number|null;signedIsDebit:boolean;dateFormat:'iso'|'mdy'|'dmy';merchant:number|null;category:number|null;kind:number|null;reference:number|null;balance:number|null}
export interface TransactionFields {date:string|null;amount:string|null;description:string;merchant:string;category:string;kind:TransactionKind}
export interface RawCsvRecord {page?:number|null;reportedLine?:string|null;extractionMethod:string;recordNumber:number;line:number;reportedDate:string;reportedAmount:string|null;reportedDebit:string|null;reportedCredit:string|null;reportedDescription:string;reportedMerchant:string|null;reportedCategory:string|null;reportedKind:string|null;reportedReference:string|null;reportedBalance:string|null;extracted:TransactionFields;reference:string|null;balance:string|null;issues:string[]}
export interface CsvDocumentInput {balanceOrder?:BalanceOrder|null;pdfSelection?:PdfLocator[]|null;accountId:string|null;accountLabel:string;accountKind:string;periodStart:string|null;periodEnd:string|null;openingBalance:string|null;closingBalance:string|null;retainSource:boolean;mapping:CsvMapping}
export interface CsvPreview {pdfPages?:PdfPage[]|null;clientId:string;token:string;fileName:string;sha256:string;byteCount:number;headers:string[];sampleRows:string[][];rowCount:number}
export interface StatementAccount {id:string;label:string;kind:string}
export interface CsvDocument {balanceHistory:StatementBalanceHistory[];balanceReconciliation:StatementBalanceReport|null;id:string;accountId:string;fileName:string;sha256:string;byteCount:number;importedAt:string;sourceRetained:boolean;input:CsvDocumentInput;rowCount:number;sourceExpectedClosing:string|null;sourceDifference:string|null;approvedDifference:string|null;warnings:string[]}
export interface ImportedRecord {id:string;documentId:string;accountId:string;raw:RawCsvRecord;fields:TransactionFields;status:ReviewStatus;linkedId:string|null;reason:string;warnings:string[];duplicateIds:string[];bulkEligible:boolean}
export interface ReviewHistory {id:string;recordId:string;fromStatus:ReviewStatus;toStatus:ReviewStatus;fields:TransactionFields;reason:string;reviewedAt:string;counselorName:string}
export interface ImportSummary {pending:number;needsClarification:number;approved:number;rejected:number;income:string;spending:string;refunds:string;netSpending:string;netCash:string;accountNet:[string,string][];warnings:string[]}
export interface ClientImports {financialFacts?:FinancialFacts;merchantGroups:MerchantGroup[];recurringHistory:RecurringReview[];clientId:string;revision:number;accounts:StatementAccount[];documents:CsvDocument[];records:ImportedRecord[];history:ReviewHistory[];summary:ImportSummary}
export interface CsvStageInput {expectedRevision:number;token:string;document:CsvDocumentInput}
export interface TransactionReviewInput {expectedRevision:number;recordId:string;status:ReviewStatus;fields:TransactionFields;reason:string;acknowledgeWarnings:boolean}
export interface TransactionBulkInput {expectedRevision:number;recordIds:string[]}
export interface TransactionPairInput {expectedRevision:number;leftId:string;rightId:string;reason:string}

export interface PdfLocator {page:number;line:number}
export interface PdfPage {number:number;text:string}

export type RecurringFrequency='weekly'|'fortnightly'|'monthly'|'quarterly'|'annual'|'irregular';
export interface PaymentEvidence {recordId:string;approvalId:string;documentId:string;accountId:string;page:number|null;line:number;fields:TransactionFields;duplicateIds:string[]}
export interface RecurringReview {invalidated:boolean;id:string;groupId:string;evidenceHash:string;label:string;decision:'confirmed'|'dismissed';frequency:RecurringFrequency|null;subscription:boolean;reason:string;reviewedAt:string;counselorName:string;evidence:PaymentEvidence[]}
export interface MerchantGroup {id:string;merchant:string;usesDescription:boolean;accountId:string;kind:TransactionKind;evidenceHash:string;evidence:PaymentEvidence[];totalPaid:string;smallestPayment:string;largestPayment:string;suggestedFrequency:RecurringFrequency|null;canConfirm:boolean;warnings:string[];state:'Unreviewed'|'Confirmed'|'Dismissed'|'Needs review';review:RecurringReview|null}
export interface RecurringReviewInput {expectedRevision:number;groupId:string;evidenceHash:string;label:string;decision:'confirmed'|'dismissed';frequency:RecurringFrequency|null;subscription:boolean;reason:string}

export type BalanceOrder='oldestFirst'|'newestFirst';
export type BalanceState='Matched'|'Difference'|'Unknown'|'Pending'|'Excluded'|'NotConfigured'|'OrderConflict';
export interface BalanceComparison {state:BalanceState;expected:string|null;difference:string|null;note:string}
export interface StatementBalanceRow {recordId:string;reported:string|null;source:BalanceComparison;approved:BalanceComparison}
export interface StatementBalanceReport {order:BalanceOrder|null;sourceSequence:'Valid'|'Unknown'|'Conflict'|'NotConfigured';approvedSequence:'Valid'|'Unknown'|'Conflict'|'NotConfigured';rows:StatementBalanceRow[];sourceMatched:number;sourceDifferences:number;approvedMatched:number;approvedDifferences:number;reportedClosing:BalanceComparison;warnings:string[]}
export interface StatementBalanceHistory {id:string;documentId:string;order:BalanceOrder|null;reason:string;reviewedAt:string;counselorName:string}
export interface StatementBalanceInput {expectedRevision:number;documentId:string;order:BalanceOrder|null;reason:string}

export interface FinancialAssessment {
 factsMetrics?:FactMetrics;
 clientId:string;revision:number;periodStart:string;periodEnd:string;calculatedOnUtc:string;
 latestApprovedDate:string|null;outdated:boolean;totals:ClientImports['summary'];operatingCashflow:string;
 months:{month:string;totals:ClientImports['summary'];operatingCashflow:string;incomeScale:number;spendingScale:number;cashflowScale:number}[];
 categories:{label:string;spending:string;scale:number}[];undatedRecords:number;reviewedPercent:number|null;
 documentIds:string[];warnings:string[];
}

export type FactRole='necessary'|'discretionary'|'takeHomePay'|'otherIncome';
export type FactBalanceKind='cash'|'accessibleSavings'|'restrictedSavings'|'otherAsset'|'debt';
export interface FactClassification {recordId:string;approvalId:string;role:FactRole}
export interface FactBalance {label:string;kind:FactBalanceKind;amount:string|null;monthlyPayment:string|null;evidence:string;documentId:string|null}
export interface FinancialFactsData {asOf:string|null;fullInventory:boolean;balances:FactBalance[];classifications:FactClassification[]}
export interface FinancialFacts {data:FinancialFactsData|null;history:{id:string;data:FinancialFactsData;reason:string;reviewedAt:string;counselorName:string}[]}
export interface FinancialFactsInput {expectedRevision:number;data:FinancialFactsData;reason:string;acknowledge:boolean}
export interface FactMetrics {
 necessarySpending:string;discretionarySpending:string;unclassifiedSpending:string;takeHomePay:string;unclassifiedIncome:string;staleClassifications:number;
 asOf:string|null;balanceComplete:boolean;balanceAligned:boolean;balanceOutdated:boolean;
 cash:string|null;accessibleSavings:string|null;restrictedSavings:string|null;debt:string|null;netWorth:string|null;
 monthlyNecessaryEstimate:string|null;emergencyMonthsEstimate:string|null;takeHomePaymentPercentEstimate:string|null;
 debtBalances:{label:string;amount:string|null;scale:number}[];
}

export interface AssessmentEvidencePreview {clientId:string;sourceRevision:number;notesCount:number;notes:ClientNote[];groupCount:number;reviewCount:number}
export interface CapturedReportEvidence {calculator:string;notesCount:number;notes:ClientNote[]}
export type IntakeSection = 'address'|'household'|'employment';
export interface ReportIntakeSelection {expectedRevision:number;sections:IntakeSection[]}
export interface ReportIntakePreview {clientId:string;revision:number;updatedAt:string|null;values:ClientIntake}
export interface ReportIntakeSnapshot {clientId:string;revision:number;updatedAt:string;sections:IntakeSection[];values:ClientIntake}
export interface AssessmentRecordInput {cards?:{revision:number};completeness?:{revision:number};expectedBrandingRevision?:number;intake?:ReportIntakeSelection;selectedScenario?:SavedScenarioSelection;supporting?:{expectedNotesCount:number;noteIds:string[]}|null;planning?:PlanningSelection|null;expectedRevision:number;expectedProfileUpdatedAt:string;fromMonth:string;throughMonth:string;title:string;findings:string;recommendations:string;followUp:string;acknowledge:boolean}
export interface AssessmentRecordSummary {id:string;title:string;createdAt:string;periodStart:string;periodEnd:string;sourceRevision:number;needsReview:boolean}
export interface AssessmentDirectory {clientId:string;records:AssessmentRecordSummary[]}
export interface SavedAssessment {validation?:ReportValidation;scenario?:SavedScenario;clientId:string;digest:string;needsReview:boolean;reviewReasons:string[];payload:{cards?:CapturedCardReview;completeness?:CoverageReviewSnapshot;branding?:FirmBrandingSnapshot;intake?:ReportIntakeSnapshot;selectedScenario?:SavedScenarioSelection;supporting?:CapturedReportEvidence|null;planning?:CapturedPlanning|null;format:number;calculator:string;appVersion:string;id:string;createdAt:string;workspace:WorkspaceInfo;client:Client;title:string;findings:string;recommendations:string;followUp:string;assessment:FinancialAssessment;basis:ClientImports}}

export interface PlanningRevisions {budget:number;debt:number;savings:number;cashflow:number}
export type PlanningDebtStrategy = 'none'|'snowball'|'avalanche';
export interface AssessmentPlans {clientId:string;budget:ClientBudget;debt:ClientDebtPlan;savings:ClientSavingsPlan;cashflow:ClientCashflowPlan}
export interface PlanningSelection {expectedRevisions:PlanningRevisions;label:string;assumptions:string;debtStrategy:PlanningDebtStrategy}
export interface CapturedPlanning {calculator:string;label:string;assumptions:string;debtStrategy:PlanningDebtStrategy;revisions:PlanningRevisions;plans:AssessmentPlans}

export interface ClientAction {step:string;owner:'client'|'counselor';dueDate:string|null}
export interface ActionPlanData {budgetRecommendations:string;actions:ClientAction[];followUp:string;followUpDate:string|null;limitations:string}
export interface ActionAllocations {monthlyIncome:string;necessary:string;discretionary:string;surplus:string;debt:string;savings:string;unallocated:string}
export interface ActionPlanInput {assessmentId:string;expectedDigest:string;title:string;data:ActionPlanData;acknowledge:boolean}
export interface ActionPlanReview {clientId:string;assessmentId:string;assessmentDigest:string;reportDate:string;allocations:ActionAllocations;disclosures:string[]}
export interface ActionPlanPayload {format:number;calculator:string;id:string;clientId:string;assessmentId:string;assessmentDigest:string;createdAt:string;reportDate:string;appVersion:string;title:string;data:ActionPlanData;allocations:ActionAllocations;disclosures:string[]}
export interface SavedActionPlan {clientId:string;payload:ActionPlanPayload;assessment:SavedAssessment;digest:string;needsReview:boolean;reviewReasons:string[]}
export interface ActionPlanDirectory {clientId:string;records:{id:string;title:string;createdAt:string;reportDate:string;assessmentId:string;needsReview:boolean}[]}

export interface BudgetMethodInput {expectedRevision:number;requiredDebtMinimum:string|null;needsPercent:string;wantsPercent:string;futurePercent:string}
export interface BudgetMethodBucket {label:string;percent:string;actual:string|null;target:string|null;difference:string|null;position:'unavailable'|'at'|'above'|'below'}
export interface BudgetMethodComparison {clientId:string;revision:number;calculator:string;requiredDebtMinimum:string|null;zeroBased:{income:string|null;assigned:string|null;remaining:string|null;status:'incomplete'|'fullyAssigned'|'unassigned'|'overAssigned'};standard:{label:string;buckets:BudgetMethodBucket[]};custom:{label:string;buckets:BudgetMethodBucket[]};warnings:string[]}

export interface ScenarioAmountChange {amount:string|null}
export interface ScenarioBudgetChange {index:number;amount:string|null}
export interface ScenarioCashChange {index:number;amount:string|null;firstDate:string}
export interface ScenarioExpense {
 label:string;kind:'necessary'|'discretionary';amount:string|null;
 frequency:'weekly'|'biweekly'|'semimonthly'|'monthly'|'annual';
 dated:{accountId:string;firstDate:string;secondDay:number|null;untilDate:string|null}|null;
}
export interface ScenarioInput {
 expectedRevisions:PlanningRevisions;label:string;assumptions:string;
 baselineStrategy:PlanningDebtStrategy;proposedStrategy:PlanningDebtStrategy;
 budgetChanges:ScenarioBudgetChange[];debtPayment:ScenarioAmountChange|null;
 emergencyContribution:ScenarioAmountChange|null;cashChanges:ScenarioCashChange[];
 debtExtras?:DatedAmount[];goalContributions?:ScenarioBudgetChange[];
 recurringExpenses?:ScenarioExpense[];
 incomeReductions?:{budgetIndex:number;fromMonth:string;throughMonth:string;amount:string|null;cashSchedule:number|null}[];
}
export interface ScenarioOutcome {strategy:PlanningDebtStrategy;firstMonthBuffer:string|null;plans:AssessmentPlans}
export interface ScenarioComparison {
 calculator:string;clientId:string;createdAt:string;revisions:PlanningRevisions;input:ScenarioInput;
 baseline:ScenarioOutcome;proposed:ScenarioOutcome;warnings:string[];
 incomePeriods?:{month:string;reduced:boolean;normalIncome:string|null;income:string|null;debtReserved:string|null;savingsReserved:string|null;normalBuffer:string|null;buffer:string|null}[];
}

export interface SavedScenarioSelection {id:string;digest:string}
export interface SavedScenario {clientId:string;digest:string;needsReview:boolean;reviewReasons:string[];payload:{id:string;format:number;appVersion:string;workspace:WorkspaceInfo;comparison:ScenarioComparison}}
export interface ScenarioDirectory {clientId:string;records:{id:string;label:string;createdAt:string;digest:string;needsReview:boolean}[]}

export interface DocumentRequest {
 id:string;title:string;kind:'statement'|'creditCard'|'loan'|'paystub'|'tax'|'other';
 status:'missing'|'received';accountId:string|null;sourceDocumentId:string|null;
 periodStart:string|null;periodEnd:string|null;dueDate:string|null;receivedDate:string|null;
 notes:string;archived:boolean;
}
export interface DocumentCollection {
 clientId:string;revision:number;latestRevision:number;updatedAt:string|null;reviewDate:string;
 items:DocumentRequest[];history:{revision:number;updatedAt:string}[];
 accounts:{id:string;label:string;kind:string}[];sources:{id:string;accountId:string;fileName:string}[];
 missing:number;received:number;archived:number;
}

export interface StatementAccountVersion {revision:number;label:string;archived:boolean;updatedAt:string}
export interface ManagedStatementAccount {id:string;label:string;originalLabel:string;kind:'checking'|'savings';archived:boolean;revision:number;sourceCount:number;history:StatementAccountVersion[]}
export interface StatementAccountsDirectory {clientId:string;importRevision:number;accounts:ManagedStatementAccount[]}
export interface StatementAccountInput {id:string|null;expectedRevision:number;expectedImportRevision:number;label:string;kind:'checking'|'savings';archived:boolean}
export interface FirmBrandingData {name:string;address:string;email:string;phone:string;website:string;disclosure:string;logoPng:string|null}
export interface FirmBrandingSnapshot {organizationId:string;revision:number;updatedAt:string;data:FirmBrandingData}
export interface FirmBrandingRecord {organizationId:string;revision:number;updatedAt:string|null;data:FirmBrandingData;history:{revision:number;name:string;updatedAt:string}[]}

export interface CompletenessData {
 clientId:string;revision:number;periodStart:string;periodEnd:string;approved:number;rejected:number;pending:number;clarification:number;undated:number;outOfPeriod:number;assignmentIssues:number;finalEligible:boolean;blockingReasons:string[];
 accounts:{id:string;label:string;kind:string;documentIds:string[];coveredDays:number;totalDays:number;gaps:{start:string;end:string}[];overlaps:{start:string;end:string}[]}[];
 sources:{id:string;fileName:string;accountId:string;periodStart:string|null;periodEnd:string|null;pending:number;clarification:number;rejected:number;undated:number;outsideDeclaredPeriod:number;issues:string[]}[];
}
export interface ReportValidation {version:string;finalEligible:boolean;coverage:CompletenessData;checks:{label:string;state:string;detail:string}[]}

export interface CoverageChoices {accounts:{id:string;label:string;kind:string}[];sources:{id:string;accountId:string;fileName:string;sha256:string;periodStart:string|null;periodEnd:string|null}[]}
export interface CoverageExpectation {accountId:string|null;label:string;kind:string;expectedStart:string|null;expectedEnd:string|null;exception:string;sourceIds:string[]}
export interface CoverageReviewData {fromMonth:string;throughMonth:string;inventoryReviewed:boolean;inventoryNotes:string;accounts:CoverageExpectation[];missingInformation:string;conclusion:string}
export interface CoverageReviewSnapshot {clientId:string;revision:number;updatedAt:string;sourceRevision:number;profileUpdatedAt:string;choices:CoverageChoices;data:CoverageReviewData}
export interface CoverageReviewRecord {clientId:string;latestRevision:number;snapshot:CoverageReviewSnapshot|null;history:{revision:number;updatedAt:string;fromMonth:string;throughMonth:string}[];choices:CoverageChoices;currentSourceRevision:number;currentProfileUpdatedAt:string;needsReview:boolean;reviewReasons:string[];unresolved:string[]}

export interface FinalReadiness {policy:string;clientId:string;assessmentId:string;assessmentDigest:string;reviewedOn:string;ready:boolean;gates:{key:string;label:string;passed:boolean;detail:string}[];accounts:{accountId:string;label:string;applicability:string;expectedStart:string|null;expectedEnd:string|null;exception:string;reasonRequired:boolean}[];planning:string;disclosures:string[]}
export interface FinalDecision {accounts:{accountId:string;reason:string}[];planning:string;planningNotes:string;professionalNotes:string;confirmClient:boolean;confirmSources:boolean;confirmInventory:boolean;confirmAssumptions:boolean}
export interface FinalReportPreview {readiness:FinalReadiness;receipt:string;approvalId:string|null}
export interface FinalReportRecord {clientId:string;digest:string;assessment:SavedAssessment;needsReview:boolean;reviewReasons:string[];payload:{format:number;policy:string;appVersion:string;id:string;clientId:string;assessmentId:string;assessmentDigest:string;approvedAt:string;reportDate:string;approvedBy:WorkspaceInfo;readiness:FinalReadiness;decision:FinalDecision}}

export type CardKind='unclassified'|'purchase'|'refund'|'payment'|'fee'|'interest'|'adjustment';
export interface CardFields {date:string|null;amount:string|null;description:string;category:string;kind:CardKind}
export interface CardInput {accountLabel:string;periodStart:string;periodEnd:string;openingOwed:string;closingOwed:string;retainSource:boolean;mapping:CsvMapping}
export interface CardSource {format:number;id:string;clientId:string;importedAt:string;importRevision:number;fileName:string;sha256:string;byteCount:number;input:CardInput;rows:RawCsvRecord[]}
export interface CardRow {row:number;raw:RawCsvRecord;fields:CardFields;status:ReviewStatus}
export interface CardReview {id:string;row:number;fields:CardFields;status:ReviewStatus;reason:string;reviewedAt:string;counselorName:string;revision:number}
export interface CardSummary {approved:number;rejected:number;pending:number;clarification:number;purchases:string;refunds:string;payments:string;fees:string;interest:string;adjustments:string;netSpending:string;liabilityMovement:string;sourceClosing:string|null;sourceDifference:string|null;approvedClosing:string|null;approvedDifference:string|null;reconciled:boolean}
export interface CardStatement {source:CardSource;rows:CardRow[];history:CardReview[];summary:CardSummary;warnings:string[]}
export interface CardSnapshot {clientId:string;revision:number;statements:CardStatement[]}

export interface CardAccount {id:string;label:string;archived:boolean}
export interface CardAssignment {statementId:string;accountId:string}
export interface CardRepayment {statementId:string;row:number;cardDecisionId:string;bankRecordId:string;bankDecisionId:string}
export interface CardLinkSnapshot {format:number;clientId:string;revision:number;sourceRevision:number;updatedAt:string;counselorName:string;reason:string;accounts:CardAccount[];assignments:CardAssignment[];repayments:CardRepayment[]}
export interface CardReviewDirectory {clientId:string;sourceRevision:number;snapshot:CardLinkSnapshot|null;history:CardLinkSnapshot[];cards:CardSnapshot;bank:ClientImports;staleRepayments:CardRepayment[]}
export type CardReviewAction={type:'createAccount';label:string}|{type:'updateAccount';id:string;label:string;archived:boolean}|{type:'assignStatement';statementId:string;accountId:string|null}|{type:'linkRepayment';statementId:string;row:number;bankRecordId:string}|{type:'unlinkRepayment';statementId:string;row:number};
export interface BankCardReview {clientId:string;calculator:string;sourceRevision:number;linkRevision:number;periodStart:string;periodEnd:string;bankCashMovement:string;bankNetSpending:string;cardNetSpending:string;observedSpending:string|null;cardPayments:string;matchedPayments:string;cardLiabilityMovement:string;issues:string[];disclosures:string[]}
export interface CapturedCardReview {ledger:CardSnapshot;links:CardLinkSnapshot;review:BankCardReview}
