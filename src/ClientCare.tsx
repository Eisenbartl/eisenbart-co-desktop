import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import type {ClientCareData,ClientCareRecord,ClientFollowUp,ClientIntake} from './types';

const employment = [['','Not specified'],['employed','Employed'],['selfEmployed','Self-employed'],['unemployed','Unemployed'],['retired','Retired'],['student','Student'],['other','Other']];
const message=(e:unknown)=>e instanceof Error?e.message:String(e);

export default function ClientCare({clientId,onDirty,onBusy,otherSaving}:{clientId:string;onDirty:(v:boolean)=>void;onBusy:(v:boolean)=>void;otherSaving:boolean}) {
  const [open,setOpen]=useState(false),[saved,setSaved]=useState<ClientCareRecord|null>(null),[draft,setDraft]=useState<ClientCareData|null>(null);
  const [loading,setLoading]=useState(false),[saving,setSaving]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState(''),[attempt,setAttempt]=useState(0);
  const generation=useRef(0);
  const dirty=Boolean(saved&&draft&&JSON.stringify(saved.data)!==JSON.stringify(draft));
  const busy=loading||saving;
  useEffect(()=>{onDirty(dirty);return()=>onDirty(false);},[dirty,onDirty]);
  useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
  useEffect(()=>{
    const g=++generation.current;
    setSaved(null);setDraft(null);setError('');setNotice('');setSaving(false);setLoading(false);
    if(!open)return()=>{generation.current++;};
    setLoading(true);
    api.getClientCare(clientId).then(r=>{
      if(g!==generation.current)return;
      if(r.clientId!==clientId)throw new Error('The intake record did not match this client.');
      setSaved(r);setDraft(structuredClone(r.data));
    }).catch(e=>{if(g===generation.current)setError(message(e));}).finally(()=>{if(g===generation.current)setLoading(false);});
    return()=>{generation.current++;};
  },[clientId,open,attempt]);
  function intake<K extends keyof ClientIntake>(key:K,value:ClientIntake[K]) {setDraft(d=>d?{...d,intake:{...d.intake,[key]:value}}:d);setNotice('');}
  function follow<K extends keyof ClientFollowUp>(index:number,key:K,value:ClientFollowUp[K]) {
    setDraft(d=>d?{...d,followUps:d.followUps.map((f,i)=>i===index?{...f,[key]:value,...(key==='kind'&&value==='task'?{time:''}:{})}:f)}:d);setNotice('');
  }
  async function save() {
    if(!saved||!draft||busy||otherSaving)return;
    const g=generation.current;setSaving(true);setError('');setNotice('');
    try {
      const r=await api.saveClientCare(clientId,{expectedRevision:saved.revision,data:draft});
      if(g!==generation.current)return;
      if(r.clientId!==clientId)throw new Error('The saved intake record did not match this client.');
      setSaved(r);setDraft(structuredClone(r.data));setNotice('Intake and follow-ups saved locally.');
    }catch(e){if(g===generation.current)setError(message(e));}
    finally{if(g===generation.current)setSaving(false);}
  }
  return <details className="panel client-care" open={open} onToggle={e=>{
    const next=e.currentTarget.open;
    if(!next&&(dirty||busy)){e.currentTarget.open=true;return;}
    setOpen(next);
  }}>
    <summary><strong>Intake and follow-ups</strong><span className="muted small">Optional household and employment details · appointments and tasks</span></summary>
    {open&&<div className="client-care-body">
      <p className="field-help">Keep sensitive details optional. Enter financial amounts in the financial models. Intake notes do not supply verified income or appear in report PDFs.</p>
      {loading&&<p role="status">Loading intake and follow-ups…</p>}
      {error&&<p className="alert error" role="alert">{error}</p>}
      {!saved&&!loading&&<button type="button" className="button secondary" disabled={otherSaving} onClick={()=>setAttempt(v=>v+1)}>Retry intake loading</button>}
      {saved&&draft&&<>
        <p className="care-reminders" role="status">Saved reminders · {saved.overdue} overdue · {saved.dueToday} due today · reviewed {saved.reviewDate} (America/Chicago)</p>
        <p className="field-help">Reminders appear in this client record. No background or Windows notifications are sent. Dates and appointment times use America/Chicago. Refresh to update the review date.</p>
        <fieldset disabled={busy||otherSaving}>
          <h3>Optional intake details</h3>
          <div className="form-grid">
            <label htmlFor="intake-address">Mailing address<textarea id="intake-address" rows={3} maxLength={1000} value={draft.intake.address} onChange={e=>intake('address',e.target.value)}/></label>
            <label htmlFor="intake-household">Household composition<textarea id="intake-household" rows={3} maxLength={2000} value={draft.intake.household} onChange={e=>intake('household',e.target.value)} placeholder="Relationships, dependents and relevant household circumstances"/></label>
            <label htmlFor="intake-employment">Employment status<select id="intake-employment" value={draft.intake.employment} onChange={e=>intake('employment',e.target.value)}>{employment.map(([v,l])=><option key={v} value={v}>{l}</option>)}</select></label>
            <label htmlFor="intake-employer">Employer<input id="intake-employer" maxLength={200} value={draft.intake.employer} onChange={e=>intake('employer',e.target.value)}/></label>
            <label htmlFor="intake-occupation">Occupation<input id="intake-occupation" maxLength={200} value={draft.intake.occupation} onChange={e=>intake('occupation',e.target.value)}/></label>
          </div>
          <label htmlFor="intake-income-notes">Employment and income details<textarea id="intake-income-notes" rows={3} maxLength={4000} value={draft.intake.incomeNotes} onChange={e=>intake('incomeNotes',e.target.value)} placeholder="Income sources, stability and information still needed; record amounts in the appropriate financial models"/></label>
          <h3>Appointments and follow-up tasks</h3>
          {!draft.followUps.length&&<p className="muted">No appointments or follow-up tasks recorded.</p>}
          {draft.followUps.map((f,i)=><div className="care-follow-up" key={f.id||`new-${i}`}>
            <h4>Follow-up {i+1}{f.id?' · Saved':''}</h4>
            <div className="form-grid">
              <label htmlFor={`follow-kind-${i}`}>Kind<select id={`follow-kind-${i}`} value={f.kind} onChange={e=>follow(i,'kind',e.target.value as ClientFollowUp['kind'])}><option value="task">Task</option><option value="appointment">Appointment</option></select></label>
              <label htmlFor={`follow-title-${i}`}>Title<input id={`follow-title-${i}`} maxLength={200} value={f.title} onChange={e=>follow(i,'title',e.target.value)} required/></label>
              <label htmlFor={`follow-date-${i}`}>Due date<input id={`follow-date-${i}`} type="date" min="1900-01-01" max="2100-12-31" value={f.dueDate} onChange={e=>follow(i,'dueDate',e.target.value)} required/></label>
              {f.kind==='appointment'&&<label htmlFor={`follow-time-${i}`}>Appointment time (optional)<input id={`follow-time-${i}`} type="time" value={f.time} onChange={e=>follow(i,'time',e.target.value)}/></label>}
              <label htmlFor={`follow-status-${i}`}>Status<select id={`follow-status-${i}`} value={f.status} onChange={e=>follow(i,'status',e.target.value as ClientFollowUp['status'])}><option value="open">Open</option><option value="completed">Completed</option><option value="cancelled">Cancelled</option></select></label>
            </div>
            <label htmlFor={`follow-notes-${i}`}>Follow-up notes / outcome<textarea id={`follow-notes-${i}`} rows={2} maxLength={2000} value={f.notes} onChange={e=>follow(i,'notes',e.target.value)}/></label>
            {!f.id&&<button type="button" className="button secondary" onClick={()=>setDraft(d=>d?{...d,followUps:d.followUps.filter((_,j)=>i!==j)}:d)}>Remove unsaved follow-up {i+1}</button>}
          </div>)}
          <div className="care-actions">
            <button type="button" className="button secondary" disabled={draft.followUps.length>=100} onClick={()=>{setNotice('');setDraft(d=>d?{...d,followUps:[...d.followUps,{id:'',kind:'task',title:'',dueDate:'',time:'',status:'open',notes:''}]}:d);}}>Add follow-up</button>
            <button type="button" className="button primary" disabled={!dirty} onClick={()=>void save()}>Save intake and follow-ups</button>
            <button type="button" className="button secondary" onClick={()=>setAttempt(v=>v+1)}>{dirty?'Discard changes and reload intake':'Refresh intake and reminders'}</button>
          </div>
          <p className="field-help">Saved follow-ups stay in history; mark unwanted items cancelled. Up to 100 items and 500 saved versions per client are supported.</p>
        </fieldset>
        {dirty&&<p role="status">Unsaved intake or follow-up changes.</p>}
        {notice&&<p className="success-message" role="status">{notice}</p>}
      </>}
    </div>}
  </details>;
}
