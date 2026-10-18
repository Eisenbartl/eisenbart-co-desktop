import type {ClientBudget,BudgetLine,ClientCashflowPlan,ClientSavingsPlan} from './types';
import type {SpendingRow} from './spendingTypes';
export interface Consent {income:boolean;balances:boolean;transactions:boolean;planning:boolean;evidence:string}
export interface Member {id:string;clientId:string|null;name:string;relationship:string;start:string;end:string|null;consent:Consent;monthlyIncome:string|null}
export interface Share {memberId:string;fixed:string;weight:string}
export interface Allocation {method:string;shares:Share[];incomeBasis?:string[]|null}
export interface Rule {id:string;category:string;effective:string;until:string|null;allocation:Allocation;note:string}
export interface Expense {id:string;agreementId:string;name:string;category:string;amount:string;frequency:string;due:string;until:string|null;effective:string;status:string;purpose:string;payer:string;accountId:string|null;reimbursementDate:string|null;allocation:Allocation|null;note:string}
export interface Account {id:string;label:string;owners:string[];openingBalance:string|null;balanceDate:string;verified:boolean}
export interface Source {clientId:string;rowId:string;approvalId:string}
export interface Payment {id:string;expenseId:string|null;period:string;fromMember:string;toMember:string|null;accountId:string|null;destinationAccount:string|null;date:string;amount:string;kind:string;state:string;source:Source|null;note:string}
export interface Goal {id:string;label:string;target:string;reserved:string;monthly:string;targetDate:string|null;allocation:Allocation;note:string}
export interface Proposal {id:string;name:string;members:Member[];expenses:Expense[];note:string}
export interface Report {id:string;month:string;revision:number;savedAt:string;permissions:string;body:string}
export interface HouseholdData {name:string;archived:boolean;members:Member[];expenses:Expense[];rules:Rule[];accounts:Account[];payments:Payment[];voids:string[];goals:Goal[];proposals:Proposal[];accepted:string[];reports:Report[]}
export interface Portion {memberId:string;amount:string;percent:number}
export interface Bill {agreementId:string;versionId:string;name:string;category:string;purpose:string;date:string;amount:string;status:string;payer:string;accountId:string|null;reimbursementDate:string|null;shares:Portion[]}
export interface MemberTotal {memberId:string;name:string;obligation:string;billPayments:string;settlementsSent:string;settlementsReceived:string;netContribution:string;remaining:string;percent:number;knownIncome:string|null;afterObligations:string|null}
export interface Category {label:string;amount:string;percent:number}
export interface Summary {month:string;total:string;necessary:string;discretionary:string;confirmedPaid:string;knownIncome:string|null;incomeComplete:boolean;activeMembers:number;unpaidBills:string;memberTotals:MemberTotal[];categories:Category[];bills:Bill[];settlements:{fromMember:string;toMember:string;amount:string}[];invalidAllocations:number;warnings:string[]}
export interface HouseholdRecord {id:string;revision:number;data:HouseholdData;summary:Summary;upcoming:Bill[];upcomingAsOf:string;history:{revision:number;at:string;action:string;note:string}[];cashflow:ClientCashflowPlan['summary'];cashflowWarnings:string[];goalResults:NonNullable<ClientSavingsPlan['summary']>['goals'];goalShares:Portion[][];sourceRows:{memberId:string;paymentId:string;clientId:string;row:SpendingRow}[];actualMix:Category[];discretionaryMix:Category[];memberMix:Category[];actualTotal:string;actualNecessary:string;actualDiscretionary:string;actualRefunds:string;actualNet:string;actualDebtPayments:string;trends:Category[];opportunities:{title:string;amount:string;count:number;detail:string}[]}
export interface HouseholdDirectoryItem {id:string;name:string;archived:boolean;members:number;revision:number}
export type HouseholdAction = {action:'rename';data:string}|{action:'archive';data:boolean}|{action:'member';data:Member}|{action:'expense';data:Expense}|{action:'rule';data:Rule}|{action:'account';data:Account}|{action:'payment';data:Payment}|{action:'voidPayment';data:string}|{action:'goal';data:Goal}|{action:'proposal';data:Proposal}|{action:'acceptProposal';data:string};
export interface HouseholdInput {expectedRevision:number;action:HouseholdAction}
export interface HouseholdBudgetPreview {clientId:string;householdRevision:number;budget:Pick<ClientBudget,'clientId'|'revision'|'lines'>;proposedLines:BudgetLine[];suggestedReplacements:number[];warnings:string[]}
export interface HouseholdCashflowPreview {clientId:string;householdRevision:number;revision:number;accounts:[string,string][];existing:NonNullable<ClientCashflowPlan['data']>['schedules'];proposed:NonNullable<ClientCashflowPlan['data']>['schedules'];suggestedReplacements:number[];warnings:string[]}
