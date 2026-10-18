import type {RawCsvRecord} from './types';
export type SpendingRole='necessary'|'discretionary';
export interface SpendingQuery {fromMonth:string;throughMonth:string;accountId:string|null;search:string;category:string|null;role:string|null;page:number;pageSize:number;comparePrevious:boolean;opportunity:string|null}
export interface SpendingRow {id:string;approvalId:string;date:string;merchant:string;amount:string;category:string;role:string;accountId:string;accountLabel:string;sourceLabel:string;editable:boolean}
export interface SpendingCategory {label:string;amount:string;share:string;basisPoints:number;scale:number}
export interface SpendingTotals {spending:string;necessary:string;discretionary:string;unclassified:string;refunds:string;netSpending:string;debtPayments:string;transfers:string;necessaryShare:string|null;discretionaryShare:string|null}
export interface SpendingOpportunity {kind:string;title:string;amount:string;count:number;detail:string;rowIds:string[]}
export interface SpendingResult {clientId:string;context:string;periodStart:string;periodEnd:string;accounts:[string,string][];categories:string[];totals:SpendingTotals;previous:SpendingTotals|null;spendingChange:string|null;mix:SpendingCategory[];necessaryCategories:SpendingCategory[];discretionaryCategories:SpendingCategory[];rows:SpendingRow[];matchingRows:number;page:number;pageSize:number;opportunities:SpendingOpportunity[];excludedUnapproved:number;excludedUncategorized:number;warnings:string[];discretionaryBudget:string|null}
export interface SpendingChange {context:string;query:SpendingQuery;rowId:string;approvalId:string;role:SpendingRole;reason:string;acknowledge:boolean}
export interface SpendingHistory {clientId:string;rowId:string;original:RawCsvRecord;events:{reviewedAt:string;counselorName:string;role:string;approvalId:string|null;reason:string}[]}
