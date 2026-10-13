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
