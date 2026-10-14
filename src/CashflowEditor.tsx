import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import {usd} from './BudgetEditor';
import type {Client,ClientCashflowPlan,CashflowPlanData,CashflowSchedule,CashflowKind,CashflowFrequency,CashflowSummary} from './types';
const cash=(value:string|null)=>value===null?'Unknown':usd(value);
const kinds: [CashflowKind,string][]=[['paycheck','Paycheck'],['bill','Bill'],['loan','Loan payment'],['savings','Savings outside modeled accounts'],['otherInflow','Other deposit'],['otherOutflow','Other payment'],['transfer','Transfer between modeled accounts']];
const frequencies: [CashflowFrequency,string][]=[['once','One time / irregular'],['weekly','Weekly'],['biweekly','Every two weeks'],['semimonthly','Twice each month'],['monthly','Monthly']];
function emptyPlan():CashflowPlanData {const now=new Date();const today=`${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`;return{startDate:today,endDate:today,accounts:[],schedules:[],notes:''};}
function message(error:unknown){return error instanceof Error?error.message:String(error);}
export default function CashflowEditor({client,onDirty,onBusy,otherSaving}:{client:Client;onDirty:(value:boolean)=>void;onBusy:(value:boolean)=>void;otherSaving:boolean}) {
  const [saved,setSaved]=useState<ClientCashflowPlan|null>(null);
  const [data,setData]=useState<CashflowPlanData>(emptyPlan);
  const [baseline,setBaseline]=useState<CashflowPlanData>(data);
  const [result,setResult]=useState<ClientCashflowPlan|null>(null);
  const [resultInputs,setResultInputs]=useState('');
  const [loading,setLoading]=useState(true),[busy,setBusy]=useState(false);
  const [loadError,setLoadError]=useState(''),[error,setError]=useState(''),[notice,setNotice]=useState('');
  const [attempt,setAttempt]=useState(0);
  const generation=useRef(0);
  const dirty=saved!==null && JSON.stringify(data)!==JSON.stringify(baseline);
  useEffect(()=>{onDirty(dirty);},[dirty,onDirty]);
  useEffect(()=>{onBusy(busy);},[busy,onBusy]);
  useEffect(()=>{
    const current=++generation.current;setSaved(null);setResult(null);setLoading(true);setBusy(false);setLoadError('');setError('');setNotice('');
    api.getClientCashflowPlan(client.id).then(value=>{
      if(current!==generation.current)return;
      if(value.clientId!==client.id){setLoadError('The cash-flow plan did not match this client.');return;}
      const next=value.data??emptyPlan();setSaved(value);setData(next);setBaseline(next);setResult(value);setResultInputs(JSON.stringify(next));
    }).catch(e=>{if(current===generation.current)setLoadError(message(e));}).finally(()=>{if(current===generation.current)setLoading(false);});
    return()=>{generation.current++;};
  },[client.id,attempt]);
  function edit(patch:Partial<CashflowPlanData>){setData(old=>({...old,...patch}));setNotice('');setError('');}
  function activity(index:number,patch:Partial<CashflowSchedule>){edit({schedules:data.schedules.map((s,i)=>i===index?{...s,...patch}:s)});}
  async function calculate(save:boolean){
    if(!saved||busy||otherSaving)return;
    const current=generation.current;const snapshot=JSON.stringify(data);setBusy(true);onBusy(true);setError('');setNotice('');
    try{
      const input={expectedRevision:saved.revision,data};
      const value=save?await api.saveClientCashflowPlan(client.id,input):await api.previewClientCashflowPlan(client.id,input);
      if(current!==generation.current)return;
      if(value.clientId!==client.id||!value.data||!value.summary||(!save && value.revision!==saved.revision)) {setError('The cash-flow response did not match this plan. Reopen the client.');return;}
      setResult(value);
      if(save){setSaved(value);setData(value.data);setBaseline(value.data);setResultInputs(JSON.stringify(value.data));setNotice('Cash-flow plan saved and calculated.');}
      else {setResultInputs(snapshot);setNotice('Preview calculated from this draft. Changes have not been saved.');}
    }catch(e){if(current===generation.current)setError(message(e));}
    finally{if(current===generation.current){setBusy(false);onBusy(false);}}
  }
  function submit(event:FormEvent){event.preventDefault();void calculate(true);}
  return <section className="panel cashflow-panel" aria-labelledby="cashflow-title">
    <div className="panel-heading"><div><p className="eyebrow">PAYDAYS & PAYMENT DATES</p><h2 id="cashflow-title">Cash-flow calendar</h2></div><span className="count-badge">USD</span></div>
    <p className="muted small">Forecast balances for {client.firstName} {client.lastName} using entered dates. A positive monthly budget can still have days with insufficient cash.</p>
    {loading?<p role="status">Loading cash-flow plan…</p>:loadError?<><div className="error-message" role="alert">{loadError}</div><button className="button secondary" type="button" onClick={()=>setAttempt(a=>a+1)}>Retry loading cash-flow plan</button></>:<>
      <form onSubmit={submit} noValidate><fieldset disabled={busy||otherSaving}>
        <div className="form-grid"><label htmlFor="cashflow-start">Period starts<input id="cashflow-start" maxLength={10} placeholder="YYYY-MM-DD" value={data.startDate} onChange={e=>edit({startDate:e.target.value})}/></label><label htmlFor="cashflow-end">Period ends<input id="cashflow-end" maxLength={10} placeholder="YYYY-MM-DD" value={data.endDate} onChange={e=>edit({endDate:e.target.value})}/></label></div>
        <h3>Accounts and opening balances</h3>
        <p className="field-help">Balances are before the first day's activity. Enter USD without commas or dollar signs; negative opening balances are allowed. Blank means unknown, 0 means a known zero. Include savings as a separate account to model internal transfers.</p>
        {data.accounts.map((a,index)=><div className="budget-line" key={a.id}><div className="budget-line-heading"><span>Account {index+1}</span><button type="button" className="text-button" aria-label={`Remove cash-flow account ${index+1}`} disabled={data.schedules.some(s=>s.accountId===a.id||s.destinationId===a.id)} onClick={()=>edit({accounts:data.accounts.filter((_,i)=>i!==index)})}>Remove</button></div><div className="form-grid">
          <label htmlFor={`cashflow-account-label-${index}`}>Account name<input id={`cashflow-account-label-${index}`} value={a.label} maxLength={120} onChange={e=>edit({accounts:data.accounts.map((v,i)=>i===index?{...v,label:e.target.value}:v)})}/></label>
          <label htmlFor={`cashflow-account-balance-${index}`}>Opening balance (USD)<input id={`cashflow-account-balance-${index}`} inputMode="decimal" placeholder="Unknown" value={a.openingBalance??''} onChange={e=>edit({accounts:data.accounts.map((v,i)=>i===index?{...v,openingBalance:e.target.value===''?null:e.target.value}:v)})}/></label>
        </div></div>)}
        <button type="button" className="button secondary" disabled={data.accounts.length>=20} onClick={()=>edit({accounts:[...data.accounts,{id:crypto.randomUUID(),label:'',openingBalance:null}]})}>Add cash-flow account</button>
        <h3 className="savings-section-title">Dated activity</h3>
        <p className="field-help">Enter actual expected pay and payment dates. Use one-time entries for irregular pay. Monthly activity repeats the first date's day, clamped to month end. Twice-monthly activity uses that day and a second day; coinciding month-end dates produce one event. No holiday adjustments are applied. Confirmed means the schedule is known, not bank verified.</p>
        {data.schedules.map((s,index)=><div className="budget-line" key={index}><div className="budget-line-heading"><span>Activity {index+1}</span><button type="button" className="text-button" aria-label={`Remove cash-flow activity ${index+1}`} onClick={()=>edit({schedules:data.schedules.filter((_,i)=>i!==index)})}>Remove</button></div><div className="form-grid">
          <label htmlFor={`cashflow-label-${index}`}>Activity name<input id={`cashflow-label-${index}`} value={s.label} maxLength={120} onChange={e=>activity(index,{label:e.target.value})}/></label>
          <label htmlFor={`cashflow-kind-${index}`}>Activity type<select id={`cashflow-kind-${index}`} value={s.kind} onChange={e=>activity(index,{kind:e.target.value as CashflowKind,destinationId:null})}>{kinds.map(([v,l])=><option value={v} key={v}>{l}</option>)}</select></label>
          <label htmlFor={`cashflow-account-${index}`}>Source account<select id={`cashflow-account-${index}`} value={s.accountId} onChange={e=>activity(index,{accountId:e.target.value})}><option value="">Choose an account</option>{data.accounts.map(a=><option key={a.id} value={a.id}>{a.label||'Unnamed account'}</option>)}</select></label>
          {s.kind==='transfer' && <label htmlFor={`cashflow-destination-${index}`}>Destination account<select id={`cashflow-destination-${index}`} value={s.destinationId??''} onChange={e=>activity(index,{destinationId:e.target.value||null})}><option value="">Choose an account</option>{data.accounts.filter(a=>a.id!==s.accountId).map(a=><option key={a.id} value={a.id}>{a.label||'Unnamed account'}</option>)}</select></label>}
          <label htmlFor={`cashflow-amount-${index}`}>Activity amount (USD)<input id={`cashflow-amount-${index}`} inputMode="decimal" placeholder="Unknown" value={s.amount??''} onChange={e=>activity(index,{amount:e.target.value===''?null:e.target.value})}/></label>
          <label htmlFor={`cashflow-date-${index}`}>First activity date<input id={`cashflow-date-${index}`} maxLength={10} placeholder="YYYY-MM-DD" value={s.firstDate} onChange={e=>activity(index,{firstDate:e.target.value})}/></label>
          <label htmlFor={`cashflow-frequency-${index}`}>Repeat<select id={`cashflow-frequency-${index}`} value={s.frequency} onChange={e=>activity(index,{frequency:e.target.value as CashflowFrequency,secondDay:null})}>{frequencies.map(([v,l])=><option value={v} key={v}>{l}</option>)}</select></label>
          {s.frequency==='semimonthly' && <label htmlFor={`cashflow-second-${index}`}>Second day of month<input id={`cashflow-second-${index}`} type="number" min={1} max={31} value={s.secondDay??''} onChange={e=>activity(index,{secondDay:Number.isNaN(e.target.valueAsNumber)?null:e.target.valueAsNumber})}/></label>}
          <label htmlFor={`cashflow-until-${index}`}>Last activity date (optional)<input id={`cashflow-until-${index}`} maxLength={10} placeholder="Through selected period" value={s.untilDate??''} onChange={e=>activity(index,{untilDate:e.target.value||null})}/></label>
          <label htmlFor={`cashflow-certainty-${index}`}>Schedule certainty<select id={`cashflow-certainty-${index}`} value={s.certainty} onChange={e=>activity(index,{certainty:e.target.value as 'confirmed'|'estimated'})}><option value="estimated">Estimated</option><option value="confirmed">Confirmed schedule</option></select></label>
        </div></div>)}
        <button className="button secondary" type="button" disabled={!data.accounts.length||data.schedules.length>=100} onClick={()=>edit({schedules:[...data.schedules,{label:'',accountId:data.accounts[0].id,destinationId:null,kind:'paycheck',amount:null,firstDate:'',frequency:'once',secondDay:null,untilDate:null,certainty:'estimated'}]})}>Add dated activity</button>
        <label className="savings-section-title" htmlFor="cashflow-notes">Cash-flow notes<textarea id="cashflow-notes" value={data.notes} maxLength={1000} rows={3} onChange={e=>edit({notes:e.target.value})}/></label>
        {error && <div className="error-message" role="alert">{error}</div>}{notice && <p role="status" className="success-message">{notice}</p>}
        <div className="budget-add"><button className="button secondary" type="button" disabled={!data.accounts.length} onClick={()=>void calculate(false)}>Preview cash flow / refresh references</button><button className="button primary" disabled={!data.accounts.length} type="submit">{busy?'Calculating cash flow…':'Save cash-flow plan and calculate'}</button></div>
      </fieldset></form>
      {!saved?.data && !result?.summary && <p className="empty-note">No saved cash-flow plan yet</p>}
      {result?.summary && result.data && resultInputs===JSON.stringify(data) ? <CashflowResults key={resultInputs} data={result.data} summary={result.summary} context={result.context} preview={dirty}/> : <p className="field-help">Preview or save to calculate this draft and refresh plan references.</p>}
    </>}
  </section>;
}
function CashflowResults({data,summary,context,preview}:{data:CashflowPlanData;summary:CashflowSummary;context:ClientCashflowPlan['context'];preview:boolean}) {
  const months=[...new Set(summary.days.map(d=>d.date.slice(0,7)))];
  const [month,setMonth]=useState(months[0]??'');const [view,setView]=useState<'calendar'|'ledger'>('calendar');const [selected,setSelected]=useState(summary.days[0]?.date??'');
  const days=summary.days.filter(d=>d.date.startsWith(month));
  const chosen=summary.days.find(d=>d.date===selected);
  const name=(id:string)=>data.accounts.find(a=>a.id===id)?.label??'Unknown account';
  const offset=days.length?new Date(`${days[0].date}T12:00:00Z`).getUTCDay():0;
  return <div className="cashflow-results">
    <h3>{preview?'Draft cash-flow preview':'Saved cash-flow forecast'}</h3>
    <p className="field-help">References: budget revision {context.budgetRevision}, debt revision {context.debtRevision}, savings revision {context.savingsRevision}. Monthly budget surplus before allocations: {cash(context.monthlySurplus)}. Budget debt / savings allocations: {cash(context.budgetDebt)} / {cash(context.budgetSavings)}. Debt base payment: {cash(context.debtMonthlyPayment)}; requested monthly savings: {cash(context.savingsMonthlyContribution)}. These are reference amounts only; enter each dated obligation once. Future changes and capped contributions must be reviewed in their source plans.</p>
    <ul className="budget-warnings">{summary.warnings.map((w,i)=><li key={i}>{w}</li>)}</ul>
    <div className="budget-totals"><div><span>Combined closing balance</span><strong>{cash(summary.combinedClosing)}</strong></div>{summary.closingBalances.map(b=><div key={b.accountId}><span>{name(b.accountId)} closing balance</span><strong>{cash(b.amount)}</strong></div>)}</div>
    <div className="cashflow-controls"><label htmlFor="cashflow-month">Forecast month<select id="cashflow-month" value={month} onChange={e=>{setMonth(e.target.value);setSelected(summary.days.find(d=>d.date.startsWith(e.target.value))?.date??'');}}>{months.map(m=><option key={m}>{m}</option>)}</select></label><button type="button" className="button secondary" aria-pressed={view==='calendar'} onClick={()=>setView('calendar')}>Calendar view</button><button type="button" className="button secondary" aria-pressed={view==='ledger'} onClick={()=>setView('ledger')}>Chronological ledger</button></div>
    {view==='calendar'?<>
      <div className="cashflow-calendar" aria-label="Projected cash-flow calendar">{['Sun','Mon','Tue','Wed','Thu','Fri','Sat'].map(d=><div className="calendar-weekday" key={d}>{d}</div>)}{Array.from({length:offset},(_,i)=><div key={`blank-${i}`}/>)}
      {days.map(d=><button className={`cashflow-day${d.warnings.length?' cashflow-day-warning':''}`} type="button" key={d.date} aria-label={`Review ${d.date}`} aria-pressed={d.date===selected} onClick={()=>setSelected(d.date)}><strong>{d.date.slice(8)}</strong><span>Combined {cash(d.combinedBalance)}</span>{d.entryIndices.map(i=><span key={i}>{summary.ledger[i].label}: {cash(summary.ledger[i].amount)} ({summary.ledger[i].certainty})</span>)}{d.warnings.length>0 && <span className="cashflow-warning-label">Review funds / payments</span>}</button>)}
      </div>
      {chosen && <div className="cashflow-day-details"><h4>Account review: {chosen.date}</h4><ul>{chosen.balances.map(b=><li key={b.accountId}>{name(b.accountId)}: {cash(b.amount)}</li>)}</ul><ul className="budget-warnings">{chosen.warnings.map((w,i)=><li key={i}>{w}</li>)}{chosen.entryIndices.flatMap(i=>summary.ledger[i].warnings.map((w,j)=><li key={`${i}-${j}`}>{summary.ledger[i].label}: {w}</li>))}</ul></div>}
    </>:<div className="budget-table-wrap"><table className="cashflow-ledger" aria-label="Cash-flow chronological ledger"><thead><tr><th>Date / activity</th><th>Type / certainty</th><th>Amount</th><th>Source after activity</th><th>Destination after transfer</th><th>Combined after activity</th><th>Funding review</th></tr></thead><tbody>{summary.ledger.filter(e=>e.date.startsWith(month)).map((e,i)=><tr key={i}><th>{e.date}<br/>{e.label}</th><td>{kinds.find(([v])=>v===e.kind)?.[1]}<br/>{e.certainty}</td><td>{cash(e.amount)}</td><td>{name(e.accountId)}<br/>{cash(e.sourceBalance)}</td><td>{e.destinationId?<>{name(e.destinationId)}<br/>{cash(e.destinationBalance)}</>:'—'}</td><td>{cash(e.combinedBalance)}</td><td>{e.warnings.join(' ')||'No warning from entered activity'}</td></tr>)}</tbody></table></div>}
  </div>;
}
