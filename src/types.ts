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
