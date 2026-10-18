import type {PlanningRevisions,ScenarioComparison} from './types';
export interface HealthQuery {fromMonth:string;throughMonth:string}
export type HealthArea='classifications'|'transactions'|'facts'|'coverage'|'cards'|'debt'|'savings'|'cashflow'|'scenarios';
export interface HealthGate {key:string;label:string;passed:boolean;detail:string}
export interface HealthComponent {key:string;label:string;points:number|null;maximum:number;value:string;explanation:string}
export interface HealthMeasure {key:string;label:string;amount:string|null;monthly:string|null;incomePercent:string|null;basis:string}
export interface HealthMonth {month:string;income:string;necessary:string;discretionary:string;netSpending:string;operatingSurplus:string;fees:string;interest:string;approvedRows:number;complete:boolean}
export interface HealthEvidence {rowId:string;approvalId:string;sourceId:string;fileName:string;sha256:string;date:string;description:string;amount:string;category:string;purpose:string;account:string;page:number|null;line:number}
export interface HealthCategory {label:string;netSpending:string;count:number;rowIds:string[]}
export interface HealthInsight {key:string;title:string;basis:string;monthlyImpact:string|null;annualImpact:string|null;impactExplanation:string;rowIds:string[]}
export interface HealthPlanLine {index:number;label:string;amount:string;kind:string}
export interface HealthPlanning {revisions:PlanningRevisions;expenseLines:HealthPlanLine[];allocationLines:HealthPlanLine[];cashLines:HealthPlanLine[];debtPayment:string|null;emergencyContribution:string|null;emergencyTarget:string|null;warnings:string[]}
export interface HealthAssessment {clientId:string;revision:number;context:string;policy:string;periodStart:string;periodEnd:string;score:number|null;balanceBasis:{reviewId:string;reviewedAt:string;counselor:string;asOf:string|null;balances:{label:string;kind:string;amount:string|null;monthlyPayment:string|null;evidence:string;documentId:string|null}[]}|null;components:HealthComponent[];gates:HealthGate[];measures:HealthMeasure[];months:HealthMonth[];categories:HealthCategory[];insights:HealthInsight[];evidence:HealthEvidence[];excludedRows:number;planning:HealthPlanning;warnings:string[]}
export interface ReductionInput {query:HealthQuery;context:string;category:string;reductionPercent:number;expenseLine:number|null;destination:'unallocated'|'debt'|'emergency';allocationLine:number|null;cashLine:number|null;acknowledge:boolean}
export interface HealthReduction {clientId:string;context:string;category:string;basis:string;monthlyImpact:string;annualImpact:string;rowIds:string[];scenario:ScenarioComparison|null;warnings:string[]}
