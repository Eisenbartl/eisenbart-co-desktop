import type {ModuleView} from './workspaceNavigation';
export const workingAreas:Partial<Record<ModuleView,string[]>>={'Financial Review':['Completeness & final validation'],Reports:['Final report checklist'],Documents:['Collection checklist','Statement accounts']};
// Keep implementation entry points and acceptance criteria in docs/prototype-development-map.md.
export const plannedAreas:Record<ModuleView,{title:string;detail:string}[]>={
 'Financial Review':[{title:'Savings progress',detail:'Dated savings progress charts based on recorded balances, with unknown-data and source labels.'}],
 Transactions:[{title:'Classification assistance',detail:'Local category suggestions and reusable classification rules, with explicit counselor review and preserved source provenance.'},{title:'Activity history',detail:'A client-wide searchable decision timeline, retaining original corrections, reversals and transfer links.'}],
 Documents:[{title:'Scanned documents / OCR',detail:'Bounded local OCR, page evidence and mandatory verification before extracted facts can be approved.'},{title:'Additional document types',detail:'Credit-card, loan, paystub and broader institution parsers, each with validated mappings and source reconciliation.'}],
 'Budget Planner':[{title:'Paycheck allocation',detail:'Period-specific allocation of saved budget priorities to confirmed or explicitly estimated paychecks without double-counting dated cash flow.'}],
 'Debt Planner':[{title:'Advanced debt terms',detail:'Daily interest, variable/promotional rates, fees and payment frequencies with explicit unknown-term safeguards and independently checked schedules.'}],
 'Savings Planner':[{title:'Funding & drawdown',detail:'Model funding sources, reserved balances and drawdowns without spending the same savings twice or treating transfers as expenses.'}],
 'Cash-Flow Calendar':[{title:'Transfer settlement',detail:'Separate outgoing and incoming transfer dates, with per-account reserves and explicit settlement assumptions.'}],
 'Financial Scenarios':[{title:'New goals & funding sources',detail:'Proposed new savings goals, account-targeted extra payments and funding/drawdown controls that preserve the saved baseline.'}],
 Reports:[{title:'Print & presentation',detail:'Direct desktop printing and organization-branded report/client-plan presentation, preserving the original recorded identity.'}],
};
export default function PlannedWorkspaceArea({view,title}:{view:ModuleView;title:string}) {
 const area=plannedAreas[view].find(a=>a.title===title);
 if(!area)return null;
 return <section className="panel planned-module-page" aria-label={`${title} planned feature`}><span className="feature-state">Planned</span><h2>{area.title}</h2><p className="muted">{area.detail}</p><p className="field-help">This area is reserved for future functionality. It does not create records or calculate results yet. Return to Working tools to use the available workflow.</p></section>;
}
