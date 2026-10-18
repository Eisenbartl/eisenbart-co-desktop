import type {CsvMapping,PdfLocator,PdfPage} from './types';
export interface PdfConfig {account:'bank'|'card';dateFormat:CsvMapping['dateFormat']|null;layout:'auto'|'signed'|'amountBalance'|'balanceAmount'|'debitCredit'|'debitCreditBalance';invert:boolean;periodStart:string|null;periodEnd:string|null}
export interface PdfEvidence {version:number;config:PdfConfig;selected:PdfLocator[];pages:PdfPage[]}
export interface PdfChoice {locator:PdfLocator;checked:boolean}
export interface PdfCandidate {locator:PdfLocator;throughLine:number;text:string;date:string|null;description:string;amount:string|null;balance:string|null;clear:boolean;explanation:string}
export interface PdfInspection {clientId:string;token:string;candidates:PdfCandidate[];selected:PdfLocator[];summary:{count:number;outflows:string;inflows:string;net:string;unknownAmounts:number};pageCount:number;transactionPages:number;excludedLines:number;warnings:string[];detectedStart:string|null;detectedEnd:string|null}
export const pdfConfig=(account:'bank'|'card'):PdfConfig=>({account,dateFormat:null,layout:'auto',invert:false,periodStart:null,periodEnd:null});
