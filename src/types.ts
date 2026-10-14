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
