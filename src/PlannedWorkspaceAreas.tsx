import type {ModuleView} from './workspaceNavigation';
export const workingAreas:Partial<Record<ModuleView,string[]>>={'Budget Planner':['Paycheck allocation'],'Debt Planner':['Advanced debt terms'],'Savings Planner':['Funding & drawdown'],'Financial Scenarios':['New goals & funding sources'],'Cash-Flow Calendar':['Transfer settlement'],Transactions:['All transactions','Card transactions','Verified figures','Running balances','Recurring payments','Transfer matching','Source checks','Activity history','Classification assistance','Completeness & final validation','Bank & card activity','Card completeness'],'Financial Review':['Savings progress','Completeness & final validation','Bank & card activity','Card completeness'],Reports:['Final report checklist','Print & presentation'],Documents:['Collection checklist','Statement accounts','Credit-card statements']};
// Keep implementation entry points and acceptance criteria in docs/prototype-development-map.md.
export const plannedAreas:Record<ModuleView,{title:string;detail:string}[]>={
 'Spending Analysis':[],
 'Financial Review':[],
 Transactions:[],
 Documents:[{title:'Scanned documents / OCR',detail:'Bounded local OCR, page evidence and mandatory verification before extracted facts can be approved.'},{title:'Additional document types',detail:'Loan, paystub and broader institution parsers, each with validated mappings and source reconciliation. Checking, savings and credit-card CSV/selectable-text PDF import is available.'}],
 'Budget Planner':[],
 'Debt Planner':[],
 'Savings Planner':[],
 'Cash-Flow Calendar':[],
 'Financial Scenarios':[],
 Reports:[],
};
export default function PlannedWorkspaceArea({view,title}:{view:ModuleView;title:string}) {
 const area=plannedAreas[view].find(a=>a.title===title);
 if(!area)return null;
 return <section className="panel planned-module-page" aria-label={`${title} planned feature`}><span className="feature-state">Planned</span><h2>{area.title}</h2><p className="muted">{area.detail}</p><p className="field-help">This area is reserved for future functionality. It does not create records or calculate results yet. Return to Working tools to use the available workflow.</p></section>;
}
