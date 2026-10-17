import FinancialHealth from './FinancialHealth';
import {confirmDiscard} from './ConfirmationDialog';
import type {HealthArea} from './healthTypes';
import {useEffect,useLayoutEffect,useState,type FormEvent} from 'react';
import FactMetricsView from './FactMetricsView';
import {api} from './api';
import {usd} from './BudgetEditor';
import type {ClientImports,FinancialAssessment} from './types';
const noop=()=>{};

function Bar({scale,kind}:{scale:number;kind:string}) {
 return <svg className={`financial-bar ${kind}`} viewBox="0 0 1000 16" preserveAspectRatio="none" aria-hidden="true"><rect width="1000" height="16" className="track"/><rect width={scale} height="16"/></svg>;
}
export default function FinancialDashboard({source,disabled,onDirty=noop,onBusy=noop,onOpenExisting,onReload}:{source:ClientImports;disabled:boolean;onDirty?:(v:boolean)=>void;onBusy?:(v:boolean)=>void;onOpenExisting?:(area:HealthArea,period?:import('./healthTypes').HealthQuery)=>void;onReload?:()=>void}) {
 const [healthDirty,setHealthDirty]=useState(false),[healthBusy,setHealthBusy]=useState(false);
 useLayoutEffect(()=>{onDirty(healthDirty);return()=>onDirty(false);},[healthDirty,onDirty]);useLayoutEffect(()=>{onBusy(healthBusy);return()=>onBusy(false);},[healthBusy,onBusy]);
 const [from,setFrom]=useState(''),[through,setThrough]=useState('');
 const [period,setPeriod]=useState<{fromMonth:string;throughMonth:string}|null>(null);
 const [result,setResult]=useState<FinancialAssessment|null>(null),[loading,setLoading]=useState(false),[error,setError]=useState(''),[retry,setRetry]=useState(0);
 useEffect(()=>{
  let active=true;setResult(null);setError('');
  if(!period){setLoading(false);return;}
  setLoading(true);
  api.getClientDashboard(source.clientId,period).then(value=>{
   if(!active)return;
   if(value.clientId!==source.clientId||value.revision!==source.revision){setError('The source records changed. Reopen this client to load the current review queue.');return;}
   setResult(value);
  }).catch(e=>{if(active)setError(e instanceof Error?e.message:String(e));}).finally(()=>{if(active)setLoading(false);});
  return()=>{active=false;};
 },[source.clientId,source.revision,period,retry]);
 function apply(e:FormEvent){e.preventDefault();if(disabled||loading||healthBusy)return;const update=()=>setPeriod({fromMonth:from,throughMonth:through});if(healthDirty)confirmDiscard('Discard temporary reduction choices and change the assessment period?',update);else update();}
 const draft=period&&(from!==period.fromMonth||through!==period.throughMonth);
 return <section className="financial-dashboard" aria-label="Client financial dashboard">
  <h3>Client financial dashboard</h3><p className="field-help">Choose 1–24 complete calendar months. The health assessment uses supported approved bank/card activity; saved planning assumptions remain separate. Bank-specific details are available below. Period choices last while this client is open.</p>
  <form onSubmit={apply}><fieldset disabled={disabled||loading||healthBusy}><div className="form-grid"><label htmlFor="assessment-from">Assessment from month<input id="assessment-from" type="text" placeholder="YYYY-MM" pattern="[0-9]{4}-[0-9]{2}" required value={from} onChange={e=>setFrom(e.target.value)}/></label><label htmlFor="assessment-through">Assessment through month<input id="assessment-through" type="text" placeholder="YYYY-MM" pattern="[0-9]{4}-[0-9]{2}" required value={through} onChange={e=>setThrough(e.target.value)}/></label></div><button className="button secondary" type="submit">View financial dashboard</button></fieldset></form>
  {draft&&<p role="status">Period inputs changed. Results below still use the displayed assessment period; apply the new months to update them.</p>}
  {loading&&<p role="status">Calculating approved assessment…</p>}
  {error&&<div role="alert" className="alert error">{error}<button className="text-button" disabled={disabled||loading} onClick={()=>setRetry(v=>v+1)}>Retry financial dashboard</button></div>}
  {!period&&<p className="empty-note">Select an assessment period to see approved financial activity.</p>}
  {result&&period&&<><FinancialHealth clientId={source.clientId} revision={source.revision} query={period} disabled={disabled||loading||Boolean(draft)} onDirty={setHealthDirty} onBusy={setHealthBusy} onOpenExisting={onOpenExisting} onReload={onReload}/><details className="health-bank-details"><summary>Existing approved bank dashboard details</summary><FinancialAssessmentView value={result}/></details></>}
 </section>;
}

export function FinancialAssessmentView({value:result,historical=false}:{value:FinancialAssessment;historical?:boolean}) {
 return <>
   <p className="assessment-basis">Assessment {result.periodStart} through {result.periodEnd} · Approved records only · Incomplete{result.outdated?' · Outdated period':''} · Source revision {result.revision} · Calculated {result.calculatedOnUtc} (UTC)</p>
   <p className="field-help">Latest approved transaction in period: {result.latestApprovedDate??'None'}. Source documents containing dated records: {result.documentIds.length}.</p>
   <dl className="budget-totals assessment-totals"><div><dt>Approved income receipts</dt><dd>{usd(result.totals.income)}</dd></div><div><dt>Gross spending</dt><dd>{usd(result.totals.spending)}</dd></div><div><dt>Refunds</dt><dd>{usd(result.totals.refunds)}</dd></div><div><dt>Net spending after refunds</dt><dd>{usd(result.totals.netSpending)}</dd></div><div><dt>Operating cash flow</dt><dd>{usd(result.operatingCashflow)}</dd></div><div><dt>All account movement</dt><dd>{usd(result.totals.netCash)}</dd></div></dl>
   <p className="assessment-review">Period review: Approved {result.totals.approved} · Rejected {result.totals.rejected} · Pending {result.totals.pending} · Needs Clarification {result.totals.needsClarification}. Decided rows: {result.reviewedPercent===null?'No dated rows':`${result.reviewedPercent}%`}. Undated rows across this client: {result.undatedRecords}.</p>
   <ul className="budget-warnings">{result.warnings.map(w=><li key={w}>{historical&&w==="This live view is recalculated from current decisions; it is not a saved final report. Period choices are not saved. Review percentages measure decisions on dated imported rows, not statement coverage."?'These figures and the assessment period were captured when the record was saved. Later decisions do not rewrite them. Review percentages measure decisions on dated imported rows, not statement coverage.':w}</li>)}</ul>
   <div className="financial-charts">
    <div><h4>Gross spending by approved category</h4><p className="field-help">Refunds are shown separately above; categories are counselor-approved labels. Bar lengths are relative to the largest category.</p>{result.categories.length?<table aria-label="Approved spending categories"><thead><tr><th>Category</th><th>Gross spending</th></tr></thead><tbody>{result.categories.map(c=><tr key={c.label}><th scope="row">{c.label}</th><td>{usd(c.spending)}<Bar scale={c.scale} kind="spending"/></td></tr>)}</tbody></table>:<p>No approved spending in this period.</p>}</div>
    <div><h4>Monthly income and spending</h4><p className="field-help">Income and gross spending share a relative scale. Months without approved activity show zero imported activity.</p><table aria-label="Monthly approved income and spending"><thead><tr><th>Month / approved rows</th><th>Income</th><th>Gross spending</th></tr></thead><tbody>{result.months.map(m=><tr key={m.month}><th scope="row">{m.month}<small>{m.totals.approved} approved rows</small></th><td>{usd(m.totals.income)}<Bar scale={m.incomeScale} kind="income"/></td><td>{usd(m.totals.spending)}<Bar scale={m.spendingScale} kind="spending"/></td></tr>)}</tbody></table></div>
    <div><h4>Monthly operating cash-flow trend</h4><p className="field-help">Income minus net spending after refunds. Transfers and balance movements are excluded. Negative amounts indicate a deficit; lengths compare absolute amounts.</p><table aria-label="Monthly operating cash flow"><thead><tr><th>Month</th><th>Operating cash flow</th><th>Account movement</th></tr></thead><tbody>{result.months.map(m=><tr key={m.month}><th scope="row">{m.month}</th><td>{usd(m.operatingCashflow)}<Bar scale={m.cashflowScale} kind={m.operatingCashflow.startsWith('-')?'deficit':'income'}/></td><td>{usd(m.totals.netCash)}</td></tr>)}</tbody></table></div>
   </div>
   {result.factsMetrics?<FactMetricsView value={result.factsMetrics}/>:<p className="field-help">Necessary/discretionary expenses, verified take-home pay, cash and accessible savings, outstanding debt, net worth, debt-to-income, and emergency coverage: unavailable. Debt balance and savings progress charts require verified balance facts. Statement arithmetic checks do not verify asset balances.</p>}
 </>;
}
