import { useEffect, useRef, useState, type FormEvent } from 'react';
import { api } from './api';
import AdvancedDebtControls from './AdvancedDebtControls';
import TargetedDebtPayments from './TargetedDebtPayments';
import { usd } from './BudgetEditor';
import type { Client, ClientDebtPlan, DebtAccount, DebtPlanData, DebtProjection, DebtType, InterestModel } from './types';

const debtTypes: [DebtType, string][] = [['creditCard','Credit card'],['autoLoan','Auto loan'],['studentLoan','Student loan'],['personalLoan','Personal loan'],['mortgage','Mortgage'],['other','Other installment debt'],['custom','Custom debt']];
const models: [InterestModel,string][] = [['unknown','Unknown terms'],['fixedMonthly','Fixed monthly APR estimate'],['daily','Daily interest — enter day basis below'],['variable','Known variable rates — enter schedule below'],['promotional','Promotional APR — enter reversion below']];
const statuses: Record<DebtProjection['status'], string> = { paidOff: 'Paid off in this model', insufficientBudget: 'Minimum payment shortfall', horizonReached: 'Balance remains after 600 months', modelLimit: 'Model safety limit reached' };
function emptyPlan(): DebtPlanData {
  const now = new Date();
  return { startMonth: `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}`, monthlyPayment: null, accounts: [], extras: [], paymentChanges: [] };
}
function message(error: unknown) { return error instanceof Error ? error.message : String(error); }

export default function DebtEditor({client, onDirty, onBusy, otherSaving,focusAdvanced=false}: {focusAdvanced?:boolean;client: Client; onDirty:(value:boolean)=>void; onBusy:(value:boolean)=>void; otherSaving:boolean}) {
  const [saved, setSaved] = useState<ClientDebtPlan | null>(null);
  const [data, setData] = useState<DebtPlanData>(emptyPlan);
  const [baseline, setBaseline] = useState<DebtPlanData>(data);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [loadError, setLoadError] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [attempt, setAttempt] = useState(0);
  const mounted = useRef(true);
  const dirty = saved !== null && JSON.stringify(data) !== JSON.stringify(baseline);
  useEffect(()=> {onDirty(dirty);},[dirty,onDirty]);
  useEffect(()=> {onBusy(saving);},[saving,onBusy]);
  useEffect(()=> {
    mounted.current = true; let current = true;
    setSaved(null); setLoading(true); setLoadError(''); setError(''); setNotice('');
    api.getClientDebtPlan(client.id).then(result=> {
      if (!current) return;
      if (result.clientId !== client.id) {setLoadError('The debt plan did not match this client.');return;}
      const draft = result.data ?? emptyPlan();
      setSaved(result); setData(draft); setBaseline(draft);
    }).catch(caught=>{if(current)setLoadError(message(caught));}).finally(()=>{if(current)setLoading(false);});
    return ()=>{current=false;mounted.current=false;};
  },[client.id,attempt]);
  function edit(patch:Partial<DebtPlanData>) {setData(previous=>({...previous,...patch}));setNotice('');setError('');}
  function account(index:number,patch:Partial<DebtAccount>) {edit({accounts:data.accounts.map((a,i)=>i===index?{...a,...patch}:a)});}
  function addAccount() {edit({accounts:[...data.accounts,{label:'',debtType:'custom',balance:null,apr:null,minimumPayment:null,monthlyFee:null,interestModel:'unknown',verified:false,termsNote:''}]});}
  async function save(event:FormEvent) {
    event.preventDefault(); if(!saved || saving || otherSaving)return;
    if(data.targetedExtras?.some(e=>e.accountIndex<0||e.accountIndex>=data.accounts.length)){setError('Choose an existing debt account for each targeted extra payment.');return;}
    setSaving(true);onBusy(true);setError('');setNotice('');
    try {
      const result = await api.saveClientDebtPlan(client.id,{expectedRevision:saved.revision,data});
      if(!mounted.current)return;
      if(result.clientId!==client.id || !result.data){setError('The saved debt plan did not match this client. Reopen the record.');return;}
      setSaved(result);setData(result.data);setBaseline(result.data);setNotice('Debt plan saved and compared.');
    }catch(caught){if(mounted.current)setError(message(caught));}
    finally{if(mounted.current){setSaving(false);onBusy(false);}}
  }
  return <section className="panel budget-panel debt-panel" aria-labelledby="debt-title">
    <div className="panel-heading"><div><p className="eyebrow">REPAYMENT PLANNING</p><h2 id="debt-title">Debt payoff planning</h2></div><span className="count-badge">USD</span></div>
    <p className="muted small">Compare plans for {client.firstName} {client.lastName}. Snowball targets the smallest balance; avalanche targets the highest APR. Both first maintain the same entered minimum payments.</p>
    {loading ? <p role="status">Loading debt plan…</p> : loadError ? <><div className="error-message" role="alert">{loadError}</div><button type="button" className="button secondary" onClick={()=>setAttempt(a=>a+1)}>Retry loading debt plan</button></> : <>
      <form onSubmit={save} noValidate><fieldset disabled={saving || otherSaving}>
        <div className="form-grid"><label htmlFor="debt-start-month">First planning month<input id="debt-start-month" value={data.startMonth} maxLength={7} placeholder="YYYY-MM" onChange={e=>edit({startMonth:e.target.value})}/></label>
        <label htmlFor="debt-monthly-payment">Total monthly debt payment (USD)<input id="debt-monthly-payment" inputMode="decimal" value={data.monthlyPayment??''} maxLength={16} placeholder="Unknown" onChange={e=>edit({monthlyPayment:e.target.value===''?null:e.target.value})}/></label></div>
        <p className="field-help">Enter the entire monthly payment budget, including minimums. Use amounts without commas or currency signs. Blank means unknown; 0 means a known zero. Payments occur at month end in this model; use Advanced terms below for daily accrual and scheduled payment frequencies. Bill dates still require separate calendar entries.</p>
        {saved?.budgetContext.revision ? <p className="debt-budget-reference">Budget reference, last checked at revision {saved.budgetContext.revision}: monthly capacity after savings <strong>{usd(saved.budgetContext.monthlyCapacity)}</strong>; planned debt payments <strong>{usd(saved.budgetContext.plannedDebtPayments)}</strong>. Saving this plan checks the latest saved budget; it does not change the budget.</p> : <p className="field-help">No saved budget reference yet. Save a client budget to check repayment capacity.</p>}
        {!data.accounts.length && <p className="empty-note">No debt accounts yet. Add the debts to include in this plan.</p>}
        <div className="debt-accounts">{data.accounts.map((a,index)=><div className="budget-line" key={index}>
          <div className="budget-line-heading"><span>Debt {index+1}</span><button type="button" className="text-button" aria-label={`Remove debt ${index+1}`} disabled={data.targetedExtras?.some(e=>e.accountIndex===index)} onClick={()=>edit({accounts:data.accounts.filter((_,i)=>i!==index),...(data.targetedExtras?{targetedExtras:data.targetedExtras.map(e=>({...e,accountIndex:e.accountIndex>index?e.accountIndex-1:e.accountIndex}))}:{}),...(data.advanced?{advanced:{...data.advanced,accounts:data.advanced.accounts.filter(a=>a.accountIndex!==index).map(a=>({...a,accountIndex:a.accountIndex>index?a.accountIndex-1:a.accountIndex}))}}:{})})}>Remove</button></div>
          <div className="form-grid">
            <label htmlFor={`debt-label-${index}`}>Debt name<input id={`debt-label-${index}`} value={a.label} maxLength={120} onChange={e=>account(index,{label:e.target.value})}/></label>
            <label htmlFor={`debt-type-${index}`}>Debt type<select id={`debt-type-${index}`} value={a.debtType} onChange={e=>account(index,{debtType:e.target.value as DebtType})}>{debtTypes.map(([value,label])=><option key={value} value={value}>{label}</option>)}</select></label>
            {([['balance','Outstanding balance (USD)'],['apr','APR (%)'],['minimumPayment','Monthly minimum (USD)'],['monthlyFee','Known monthly fee (USD)']] as const).map(([key,label])=><label key={key} htmlFor={`debt-${key}-${index}`}>{label}<input id={`debt-${key}-${index}`} inputMode="decimal" value={a[key]??''} maxLength={key==='apr'?12:16} placeholder="Unknown" onChange={e=>account(index,{[key]:e.target.value===''?null:e.target.value})}/></label>)}
            <label htmlFor={`debt-model-${index}`}>Interest model<select id={`debt-model-${index}`} value={a.interestModel} onChange={e=>account(index,{interestModel:e.target.value as InterestModel})}>{models.map(([value,label])=><option key={value} value={value}>{label}</option>)}</select></label>
          </div>
          <label className="budget-estimate" htmlFor={`debt-verified-${index}`}><input id={`debt-verified-${index}`} type="checkbox" checked={a.verified} onChange={e=>account(index,{verified:e.target.checked})}/>Terms checked by counselor</label>
          <label htmlFor={`debt-note-${index}`}>Source and terms notes<textarea id={`debt-note-${index}`} rows={2} value={a.termsNote} maxLength={500} onChange={e=>account(index,{termsNote:e.target.value})}/></label>
        </div>)}</div>
        <div className="budget-add"><button type="button" className="button secondary" disabled={data.accounts.length>=50} onClick={addAccount}>Add debt</button></div>
        <AdvancedDebtControls open={focusAdvanced||undefined} data={data} onChange={edit}/>
        <details className="debt-events"><summary>Extra payments and future payment changes</summary><p className="field-help">An extra payment is added once in its month. A payment change replaces the total monthly budget from that month onward. Both strategies use the same events.</p>
          {(['extras','paymentChanges'] as const).map(key=><div key={key}><h3>{key==='extras'?'One-time extra payments':'Monthly payment changes'}</h3>
            {data[key].map((value,index)=><div className="debt-event" key={index}>
              <label htmlFor={`debt-${key}-month-${index}`}>Month (YYYY-MM)<input id={`debt-${key}-month-${index}`} value={value.month} maxLength={7} onChange={e=>edit({[key]:data[key].map((v,i)=>i===index?{...v,month:e.target.value}:v)})}/></label>
              <label htmlFor={`debt-${key}-amount-${index}`}>{key==='extras'?'Extra amount (USD)':'New total payment (USD)'}<input id={`debt-${key}-amount-${index}`} inputMode="decimal" value={value.amount} maxLength={16} onChange={e=>edit({[key]:data[key].map((v,i)=>i===index?{...v,amount:e.target.value}:v)})}/></label>
              <button type="button" className="text-button" aria-label={`Remove ${key==='extras'?'extra payment':'payment change'} ${index+1}`} onClick={()=>edit({[key]:data[key].filter((_,i)=>i!==index)})}>Remove</button>
            </div>)}
            <button type="button" className="button secondary" disabled={data[key].length>=24} onClick={()=>edit({[key]:[...data[key],{month:data.startMonth,amount:''}]})}>{key==='extras'?'Add extra payment':'Add payment change'}</button>
          </div>)}
        </details>
        <details className="debt-events"><summary>Target an extra payment to a specific account</summary><TargetedDebtPayments accounts={data.accounts} value={data.targetedExtras??[]} onChange={extras=>edit({targetedExtras:extras.length?extras:undefined})}/></details>
        {error && <div className="error-message" role="alert">{error}</div>}
        <div className="form-actions"><span className="save-status" role="status">{notice||(dirty?'Unsaved debt plan changes':saved?.revision?'Saved debt plan':'No saved debt plan yet')}</span><button className="button primary" type="submit">{saving?'Comparing…':'Save debt plan and compare'}</button></div>
      </fieldset></form>
      {dirty ? <p className="budget-pending" role="status">Save the debt plan to compare your changes.</p> : saved?.comparison && <div className="debt-results">
        <h3>Strategy comparison</h3><p className="field-help">Projected months are estimates. Interest and payment totals for an unfinished projection cover only the simulated months; they are not full payoff totals.</p>
        <div className="budget-warnings"><h4>Assumptions and review</h4><ul>{saved.comparison.warnings.map(w=><li key={w}>{w}</li>)}</ul></div>
        {saved.comparison.snowball && saved.comparison.avalanche ? <>
          <div className="debt-table-scroll"><table className="debt-comparison" aria-label="Debt strategy comparison"><thead><tr><th scope="col">Result</th><th scope="col">Snowball</th><th scope="col">Avalanche</th></tr></thead><tbody>
            <tr><th scope="row">Outcome</th>{[saved.comparison.snowball,saved.comparison.avalanche].map((p,i)=><td key={i}>{statuses[p.status]}</td>)}</tr>
            <tr><th scope="row">Estimated debt-free month</th>{[saved.comparison.snowball,saved.comparison.avalanche].map((p,i)=><td key={i}>{p.payoffMonth??(p.status==='paidOff'?'No active starting balance':'Not projected')}</td>)}</tr>
            {([['totalInterest','Interest in simulated months'],['totalFees','Fees in simulated months'],['totalPaid','Payments in simulated months'],['remainingBalance','Remaining balance']] as const).map(([key,label])=><tr key={key}><th scope="row">{label}</th><td>{usd(saved.comparison!.snowball![key])}</td><td>{usd(saved.comparison!.avalanche![key])}</td></tr>)}
          </tbody></table></div>
          {!!saved.comparison.snowball.datedPayments?.length&&<details><summary>Exact entered payment-date estimates</summary><p className="field-help">Same inputs as the monthly schedule; month-end rows can contain accrual without a payment. No bank payments or calendar entries are created.</p>{(['snowball','avalanche'] as const).map(strategy=><div key={strategy}><h4>{strategy}</h4><div className="budget-table-wrap"><table aria-label={`${strategy} dated debt payments`}><thead><tr><th>Date</th>{saved.data?.accounts.map((a,i)=><th key={i}>{a.label} payment / balance</th>)}</tr></thead><tbody>{saved.comparison![strategy]!.datedPayments!.slice(0,120).map((row,i)=><tr key={i}><th>{row.date}</th>{row.payments.map((v,j)=><td key={j}>{usd(v)} / {usd(row.balances[j])}</td>)}</tr>)}</tbody></table></div><p className="field-help">Showing the first 120 dated events. Monthly schedule includes the full projection horizon.</p></div>)}</details>}
          <DebtSchedule key={`${saved.revision}-${saved.clientId}`} plan={saved}/>
        </> : <p className="empty-note">Comparison unavailable until all active debts have known, supported terms. Incomplete debt records have been saved.</p>}
      </div>}
    </>}
  </section>;
}

function DebtSchedule({plan}:{plan:ClientDebtPlan}) {
  const [strategy,setStrategy] = useState<'snowball'|'avalanche'>('snowball');
  const [index,setIndex] = useState(0);
  const projection = plan.comparison![strategy]!;
  const month = projection.schedule[index];
  return <div className="debt-schedule">
    <h3>Monthly account schedule</h3>
    <div className="form-grid"><label htmlFor="debt-strategy">Schedule strategy<select id="debt-strategy" value={strategy} onChange={e=>{setStrategy(e.target.value as typeof strategy);setIndex(0);}}><option value="snowball">Snowball</option><option value="avalanche">Avalanche</option></select></label>
    {projection.schedule.length>0 && <label htmlFor="debt-schedule-month">Schedule month<select id="debt-schedule-month" value={index} onChange={e=>setIndex(Number(e.target.value))}>{projection.schedule.map((m,i)=><option key={m.month} value={i}>{m.month}</option>)}</select></label>}</div>
    {projection.warnings.length>0 && <div className="budget-warnings"><ul>{projection.warnings.map(w=><li key={w}>{w}</li>)}</ul></div>}
    {month ? <><p className="muted small">{month.month}: available payment {usd(month.availablePayment)}, allocated {usd(month.totalPayment)}, remaining balance {usd(month.remainingBalance)}.</p>
      <div className="debt-table-scroll"><table className="debt-month-table" aria-label="Monthly debt account schedule"><thead><tr><th scope="col">Debt</th><th scope="col">Interest</th><th scope="col">Fee</th><th scope="col">Payment</th><th scope="col">Closing balance</th><th scope="col">Estimated payoff month</th></tr></thead><tbody>{plan.data!.accounts.map((a,i)=><tr key={i}><th scope="row">{a.label}{projection.negativeAmortizationAccounts.includes(i)&&<span className="budget-deficit"> · Balance grows in some months</span>}</th><td>{usd(month.interest[i])}</td><td>{usd(month.fees[i])}</td><td>{usd(month.payments[i])}</td><td>{usd(month.balances[i])}</td><td>{projection.accountPayoffMonths[i]??'Not projected'}</td></tr>)}</tbody></table></div>
    </> : <p className="empty-note">No affordable monthly schedule was produced, or all starting balances are zero. Review the outcome and warnings.</p>}
  </div>;
}
