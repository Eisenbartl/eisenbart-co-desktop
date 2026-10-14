import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import {usd} from './BudgetEditor';
import type {Client,ClientSavingsPlan,SavingsForecast,SavingsGoal,SavingsPlanData,SavingsSummary} from './types';

const cash=(value:string|null)=>value===null?'Unknown':usd(value);
const forecastText=(forecast:SavingsForecast)=>({funded:'Already funded from entered reserves',projected:'Projected with entered contributions',noContribution:'No positive contribution',unknown:'Missing inputs — not projected',horizonReached:'Not reached within 600 months'}[forecast.status]);
function emptyPlan():SavingsPlanData {const now=new Date();return{startMonth:`${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}`,targetMonths:3,liquidSavings:null,restrictedSavings:null,obligationsReserved:null,emergencyContribution:null,alternativeContributions:[],goals:[],prioritiesNote:''};}
function message(error:unknown){return error instanceof Error?error.message:String(error);}
export default function SavingsEditor({client,onDirty,onBusy,otherSaving}:{client:Client;onDirty:(value:boolean)=>void;onBusy:(value:boolean)=>void;otherSaving:boolean}) {
  const [saved,setSaved]=useState<ClientSavingsPlan|null>(null);
  const [data,setData]=useState<SavingsPlanData>(emptyPlan);
  const [baseline,setBaseline]=useState<SavingsPlanData>(data);
  const [loading,setLoading]=useState(true);
  const [saving,setSaving]=useState(false);
  const [refreshing,setRefreshing]=useState(false);
  const [loadError,setLoadError]=useState('');
  const [error,setError]=useState('');
  const [notice,setNotice]=useState('');
  const [attempt,setAttempt]=useState(0);
  const mounted=useRef(true);
  const dirty=saved!==null && JSON.stringify(data)!==JSON.stringify(baseline);
  useEffect(()=>{onDirty(dirty);},[dirty,onDirty]);
  useEffect(()=>{onBusy(saving||refreshing);},[saving,refreshing,onBusy]);
  useEffect(()=>{
    mounted.current=true;let current=true;
    setSaved(null);setLoading(true);setLoadError('');setError('');setNotice('');
    api.getClientSavingsPlan(client.id).then(result=>{
      if(!current)return;
      if(result.clientId!==client.id){setLoadError('The savings plan did not match this client.');return;}
      const next=result.data??emptyPlan();setSaved(result);setData(next);setBaseline(next);
    }).catch(caught=>{if(current)setLoadError(message(caught));}).finally(()=>{if(current)setLoading(false);});
    return()=>{current=false;mounted.current=false;};
  },[client.id,attempt]);
  function edit(patch:Partial<SavingsPlanData>){setData(previous=>({...previous,...patch}));setNotice('');setError('');}
  function goal(index:number,patch:Partial<SavingsGoal>){edit({goals:data.goals.map((g,i)=>i===index?{...g,...patch}:g)});}
  async function save(event:FormEvent){
    event.preventDefault();if(!saved||saving||otherSaving)return;
    setError('');setNotice('');
    if(!Number.isInteger(data.targetMonths)||data.targetMonths<1||data.targetMonths>60){setError('Choose 1–60 whole months for emergency coverage.');return;}
    setSaving(true);onBusy(true);
    try{
      const result=await api.saveClientSavingsPlan(client.id,{expectedRevision:saved.revision,data});
      if(!mounted.current)return;
      if(result.clientId!==client.id||!result.data){setError('The saved savings plan did not match this client. Reopen the record.');return;}
      setSaved(result);setData(result.data);setBaseline(result.data);setNotice('Savings plan saved and calculated.');
    }catch(caught){if(mounted.current)setError(message(caught));}
    finally{if(mounted.current){setSaving(false);onBusy(false);}}
  }
  async function refreshReferences(){
    if(!saved||saving||refreshing||otherSaving)return;
    setRefreshing(true);onBusy(true);setError('');
    try{
      const result=await api.getClientSavingsPlan(client.id);
      if(!mounted.current)return;
      if(result.clientId!==client.id||result.revision!==saved.revision){setError('The savings plan changed elsewhere. Reopen this client before saving.');return;}
      setSaved(result); // Keep the draft and baseline; refreshing never adopts another plan's inputs.
    }catch(caught){if(mounted.current)setError(message(caught));}
    finally{if(mounted.current){setRefreshing(false);onBusy(false);}}
  }
  return <section className="panel budget-panel savings-panel" aria-labelledby="savings-title">
    <div className="panel-heading"><div><p className="eyebrow">RESERVES & GOALS</p><h2 id="savings-title">Savings planning</h2></div><span className="count-badge">USD</span></div>
    <p className="muted small">Plan emergency coverage and sinking funds for {client.firstName} {client.lastName}. Savings transfers are allocations, not additional household spending.</p>
    {loading?<p role="status">Loading savings plan…</p>:loadError?<><div className="error-message" role="alert">{loadError}</div><button type="button" className="button secondary" onClick={()=>setAttempt(a=>a+1)}>Retry loading savings plan</button></>:<>
      <form onSubmit={save} noValidate><fieldset disabled={saving||refreshing||otherSaving}>
        <div className="form-grid"><label htmlFor="savings-start-month">First contribution month<input id="savings-start-month" value={data.startMonth} maxLength={7} placeholder="YYYY-MM" onChange={e=>edit({startMonth:e.target.value})}/></label>
        <label htmlFor="savings-target-months">Emergency target (months)<input id="savings-target-months" type="number" min={1} max={60} step={1} value={Number.isNaN(data.targetMonths)?'':data.targetMonths} onChange={e=>edit({targetMonths:e.target.valueAsNumber})}/></label></div>
        <div className="budget-add" aria-label="Emergency target presets">{[1,3,6].map(months=><button type="button" className="button secondary" key={months} onClick={()=>edit({targetMonths:months})}>{months} {months===1?'month':'months'}</button>)}</div>
        <h3>Current savings buckets</h3>
        <p className="field-help">Enter total accessible savings before earmarks. Bill reserves and each sinking fund's reserved amount are parts of that same pool. They are subtracted to find emergency money. Restricted savings are separate and excluded. Use USD without commas or dollar signs; blank means unknown, 0 means a known zero.</p>
        <div className="form-grid">{([
          ['liquidSavings','Total accessible savings before earmarks (USD)'],['restrictedSavings','Restricted savings (USD)'],['obligationsReserved','Reserved for upcoming bills (USD)'],['emergencyContribution','Monthly emergency contribution (USD)'],
        ] as const).map(([key,label])=><label key={key} htmlFor={`savings-${key}`}>{label}<input id={`savings-${key}`} inputMode="decimal" value={data[key]??''} maxLength={16} placeholder="Unknown" onChange={e=>edit({[key]:e.target.value===''?null:e.target.value})}/></label>)}</div>
        {saved && <p className="debt-budget-reference">References last checked: budget revision {saved.context.budgetRevision}, debt plan revision {saved.context.debtRevision}. Necessary monthly expenses: <strong>{cash(saved.context.necessaryExpenses)}</strong>. Saving rechecks the latest plans; it does not change their amounts.</p>}
        <button type="button" className="button secondary" disabled={refreshing} onClick={refreshReferences}>{refreshing?'Refreshing references…':'Refresh budget and debt references'}</button>
        <h3 className="savings-section-title">Sinking funds</h3>
        {!data.goals.length && <p className="empty-note">No sinking funds yet. Add a goal for insurance, repairs, medical costs, or other planned needs.</p>}
        {data.goals.map((g,index)=><div className="budget-line" key={index}>
          <div className="budget-line-heading"><span>Sinking fund {index+1}</span><button type="button" className="text-button" aria-label={`Remove sinking fund ${index+1}`} onClick={()=>edit({goals:data.goals.filter((_,i)=>i!==index)})}>Remove</button></div>
          <div className="form-grid"><label htmlFor={`savings-goal-label-${index}`}>Goal name<input id={`savings-goal-label-${index}`} value={g.label} maxLength={120} onChange={e=>goal(index,{label:e.target.value})}/></label>
          <label htmlFor={`savings-goal-date-${index}`}>Target date (YYYY-MM-DD)<input id={`savings-goal-date-${index}`} value={g.targetDate??''} maxLength={10} placeholder="Unknown" onChange={e=>goal(index,{targetDate:e.target.value===''?null:e.target.value})}/></label>
          {([['targetAmount','Goal target (USD)'],['reservedAmount','Already reserved in accessible savings (USD)'],['monthlyContribution','Monthly goal contribution (USD)']] as const).map(([key,label])=><label key={key} htmlFor={`savings-goal-${key}-${index}`}>{label}<input id={`savings-goal-${key}-${index}`} inputMode="decimal" value={g[key]??''} maxLength={16} placeholder="Unknown" onChange={e=>goal(index,{[key]:e.target.value===''?null:e.target.value})}/></label>)}</div>
        </div>)}
        <div className="budget-add"><button type="button" className="button secondary" disabled={data.goals.length>=50} onClick={()=>edit({goals:[...data.goals,{label:'',targetAmount:null,targetDate:null,reservedAmount:null,monthlyContribution:null}]})}>Add sinking fund</button></div>
        <details className="debt-events"><summary>Compare emergency contributions</summary><p className="field-help">Compare up to five alternative monthly contributions with the same reserves, goals, and debt commitments. These estimates do not replace your selected plan.</p>
          {data.alternativeContributions.map((amount,index)=><div className="savings-alternative" key={index}><label htmlFor={`savings-alternative-${index}`}>Alternative monthly contribution (USD)<input id={`savings-alternative-${index}`} inputMode="decimal" value={amount} maxLength={16} onChange={e=>edit({alternativeContributions:data.alternativeContributions.map((v,i)=>i===index?e.target.value:v)})}/></label><button type="button" className="text-button" aria-label={`Remove alternative ${index+1}`} onClick={()=>edit({alternativeContributions:data.alternativeContributions.filter((_,i)=>i!==index)})}>Remove</button></div>)}
          <button type="button" className="button secondary" disabled={data.alternativeContributions.length>=5} onClick={()=>edit({alternativeContributions:[...data.alternativeContributions,'']})}>Add alternative contribution</button>
        </details>
        <label htmlFor="savings-priorities">Funding priorities and notes<textarea id="savings-priorities" value={data.prioritiesNote} rows={3} maxLength={1000} onChange={e=>edit({prioritiesNote:e.target.value})}/></label>
        <p className="field-help">Set the contributions and priorities with your client. Reserve debt commitments first; the review uses the higher of the saved budget debt allocation and the debt plan's payment, including future changes and extras. Reconcile annual bills already included in budget expenses so the same need is not counted twice.</p>
        {error && <div className="error-message" role="alert">{error}</div>}
        <div className="form-actions"><span className="save-status" role="status">{notice||(dirty?'Unsaved savings plan changes':saved?.revision?'Saved savings plan':'No saved savings plan yet')}</span><button className="button primary" type="submit">{saving?'Calculating savings…':'Save savings plan and calculate'}</button></div>
      </fieldset></form>
      {dirty?<p className="budget-pending" role="status">Save the savings plan to calculate your changes.</p>:saved?.summary&&<SavingsResults key={`${saved.clientId}-${saved.revision}`} plan={saved} summary={saved.summary}/>}
    </>}
  </section>;
}

function SavingsResults({plan,summary}:{plan:ClientSavingsPlan;summary:SavingsSummary}) {
  const [month,setMonth]=useState(0);
  const row=summary.schedule[month];
  return <div className="savings-results">
    <h3>Emergency savings summary</h3>
    <dl className="budget-totals">{([
      ['Available emergency savings',cash(summary.accessibleEmergency)],['Reserved for bills',cash(summary.obligationsReserved)],['Reserved in sinking funds',cash(summary.sinkingFundsReserved)],['Restricted savings — excluded',cash(summary.restrictedSavings)],
      ['Current emergency coverage',summary.coverageMonths===null?'Not calculated':`${summary.coverageMonths} months`],['Emergency target',cash(summary.emergencyTarget)],['Remaining to target',cash(summary.emergency.remaining)],
      ['Entered monthly savings contributions',cash(summary.requestedMonthlySavings)],['Emergency forecast',forecastText(summary.emergency)],['Estimated emergency completion',summary.emergency.completionDate??'Not projected'],
    ]).map(([label,value])=><div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl>
    <div className="budget-warnings"><h4>Assumptions and allocation review</h4><ul>{summary.warnings.map(w=><li key={w}>{w}</li>)}</ul></div>
    {summary.goals.length>0&&<div className="debt-table-scroll"><table aria-label="Sinking fund forecasts"><thead><tr><th scope="col">Goal</th><th scope="col">Remaining</th><th scope="col">Suggested monthly contribution</th><th scope="col">Estimated completion</th><th scope="col">Deadline review</th></tr></thead><tbody>{summary.goals.map((g,i)=><tr key={i}><th scope="row">{plan.data!.goals[i].label}</th><td>{cash(g.forecast.remaining)}</td><td>{cash(g.suggestedContribution)}</td><td>{g.forecast.completionDate??forecastText(g.forecast)}</td><td>{g.onTrack===null?'Not verified':g.onTrack?'Funded / on track':'Funding shortfall'}{g.warnings.map(w=><p className="field-help" key={w}>{w}</p>)}</td></tr>)}</tbody></table></div>}
    {summary.alternatives.length>0&&<div className="debt-table-scroll"><table aria-label="Emergency contribution scenarios"><thead><tr><th scope="col">Monthly contribution</th><th scope="col">Outcome</th><th scope="col">Estimated completion</th><th scope="col">Allocation check</th></tr></thead><tbody>{summary.alternatives.map((s,i)=><tr key={i}><th scope="row">{cash(s.forecast.contribution)}</th><td>{forecastText(s.forecast)}</td><td>{s.forecast.completionDate??'Not projected'}</td><td className={s.allocationWarning?'budget-deficit':undefined}>{s.allocationWarning===null?'Unavailable':s.allocationWarning?'Exceeds entered funds':'Within entered funds'}</td></tr>)}</tbody></table></div>}
    <p className="field-help">Contributions arrive at month end, stop at each goal, and cap the last transfer at the remaining amount. A deadline before month end cannot use that month's contribution. Suggested amounts round upward to cents. Emergency coverage rounds down to two decimal places. Review estimates and cash availability before acting.</p>
    {summary.schedule.length>0&&<div className="savings-schedule"><h3>Monthly allocation check</h3><label htmlFor="savings-schedule-month">Allocation month<select id="savings-schedule-month" value={month} onChange={e=>setMonth(Number(e.target.value))}>{summary.schedule.map((m,i)=><option key={m.month} value={i}>{m.month}</option>)}</select></label>
      {row&&<><dl className="budget-totals">{([['Emergency transfer',cash(row.emergencyContribution)],['Total savings transfers',cash(row.totalSavings)],['Debt payment reserved',cash(row.debtReserved)],['Unallocated / deficit',cash(row.unallocated)]]).map(([label,value])=><div key={label}><dt>{label}</dt><dd className={value.startsWith('-')?'budget-deficit':''}>{value}</dd></div>)}</dl>
      <div className="debt-table-scroll"><table aria-label="Monthly sinking fund contributions"><thead><tr><th scope="col">Goal</th><th scope="col">Contribution</th></tr></thead><tbody>{row.goalContributions.map((amount,i)=><tr key={i}><th scope="row">{plan.data!.goals[i].label}</th><td>{usd(amount)}</td></tr>)}</tbody></table></div></>}
    </div>}
  </div>;
}
