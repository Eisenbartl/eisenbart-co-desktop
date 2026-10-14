import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import {FinancialAssessmentView} from './FinancialDashboard';
import {usd} from './BudgetEditor';
import type {Client,ClientImports,AssessmentDirectory,SavedAssessment} from './types';
const blank=()=>({fromMonth:'',throughMonth:'',title:'',findings:'',recommendations:'',followUp:'',acknowledge:false});
const date=(v:string)=>new Intl.DateTimeFormat('en-US',{dateStyle:'medium',timeStyle:'short',timeZone:'America/Chicago'}).format(new Date(v));
const message=(e:unknown)=>e instanceof Error?e.message:String(e);
export default function AssessmentRecords({client,source,disabled,onDirty,onBusy}:{client:Client;source:ClientImports;disabled:boolean;onDirty:(v:boolean)=>void;onBusy:(v:boolean)=>void}) {
 const [open,setOpen]=useState(false),[draft,setDraft]=useState(blank),[context,setContext]=useState({revision:source.revision,profile:client.updatedAt});
 const [directory,setDirectory]=useState<AssessmentDirectory|null>(null),[selected,setSelected]=useState<SavedAssessment|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState(''),[attempt,setAttempt]=useState(0),[page,setPage]=useState(0),[exportPath,setExportPath]=useState('');
 const generation=useRef(0),selectedId=useRef<string|null>(null);
 useEffect(()=>{setExportPath('');setDraft(blank());setContext({revision:source.revision,profile:client.updatedAt});setDirectory(null);setSelected(null);selectedId.current=null;setPage(0);setNotice('');},[client.id]);
 const dirty=JSON.stringify(draft)!==JSON.stringify(blank());
 const changed=context.revision!==source.revision||context.profile!==client.updatedAt;
 useEffect(()=>{if(!dirty)setContext({revision:source.revision,profile:client.updatedAt});},[dirty,source.revision,client.updatedAt]);
 useEffect(()=>{onDirty(dirty);return()=>onDirty(false);},[dirty,onDirty]);useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 useEffect(()=>{
  let active=true;const current=++generation.current;if(!open)return()=>{generation.current++;};
  setBusy(true);setError('');
  api.listAssessmentRecords(client.id).then(async value=>{
   if(!active||current!==generation.current)return;if(value.clientId!==client.id)throw new Error('The assessment directory did not match this client.');setDirectory(value);
   if(selectedId.current){const id=selectedId.current;const r=await api.getAssessmentRecord(client.id,id);if(!active||current!==generation.current)return;if(r.clientId!==client.id||r.payload.client.id!==client.id||r.payload.id!==id)throw new Error('The saved assessment did not match this client.');setSelected(r);}
  }).catch(e=>{if(active&&current===generation.current){setError(message(e));setDirectory(null);setSelected(null);}}).finally(()=>{if(active&&current===generation.current)setBusy(false);});
  return()=>{active=false;generation.current++;};
 },[client.id,client.updatedAt,source.revision,open,attempt]);
 async function select(id:string){
  if(busy||disabled)return;selectedId.current=id;setExportPath('');setNotice('');setSelected(null);setPage(0);setError('');if(!id)return;
  const current=generation.current;setBusy(true);
  try{const r=await api.getAssessmentRecord(client.id,id);if(current!==generation.current)return;if(r.clientId!==client.id||r.payload.client.id!==client.id||r.payload.id!==id)throw new Error('The saved assessment did not match this client.');setSelected(r);}catch(e){if(current===generation.current)setError(message(e));}finally{if(current===generation.current)setBusy(false);}
 }
 async function save(){
  if(disabled||busy||changed)return;const current=generation.current;setBusy(true);setError('');setNotice('');
  try{const r=await api.saveAssessmentRecord(client.id,{...draft,expectedRevision:context.revision,expectedProfileUpdatedAt:context.profile});if(current!==generation.current)return;if(r.clientId!==client.id||r.payload.client.id!==client.id)throw new Error('The saved assessment did not match this client.');selectedId.current=r.payload.id;setSelected(r);setPage(0);setDraft(blank());setContext({revision:source.revision,profile:client.updatedAt});setNotice('Assessment record saved with its supporting evidence.');setAttempt(v=>v+1);}catch(e){if(current===generation.current)setError(message(e));}finally{if(current===generation.current)setBusy(false);}
 }
 async function exportPdf(choose:boolean){
  if(!selected||busy||disabled||dirty)return;
  const current=generation.current,id=selected.payload.id;setBusy(true);setError('');setNotice('');
  try{
   if(choose){const path=await api.chooseAssessmentPdf(client.id,id);if(current===generation.current&&path!==null)setExportPath(path);}
   else{const result=await api.exportAssessmentPdf(client.id,id,exportPath);if(current!==generation.current)return;if(result.recordId!==id)throw new Error('The exported assessment did not match the selected record.');setNotice(`PDF saved locally (${result.pages} pages): ${result.path}`);}
  }catch(e){if(current===generation.current)setError(message(e));}finally{if(current===generation.current)setBusy(false);}
 }
 const p=selected?.payload;
 const needsReview=Boolean(selected?.needsReview||(p&&(p.assessment.revision!==source.revision||p.client.updatedAt!==client.updatedAt)));
 return <details className="saved-assessments" open={open}><summary onClick={e=>{e.preventDefault();if(!busy)setOpen(v=>!v);}}>Saved counselor assessments{dirty?' — unsaved draft':''}</summary>
  <p className="field-help">Save an internal assessment record with its exact approved figures and evidence. Client identity and goals use the saved profile. Planning scenarios, unsaved edits and final client action plans are not included in this checkpoint. Saved records can be exported as printable internal PDFs. Missing information remains disclosed.</p>
  {open&&<>
   {busy&&<p role="status">Working on saved assessments…</p>}{error&&<div className="alert error" role="alert">{error}<button className="text-button" disabled={busy||disabled} onClick={()=>setAttempt(v=>v+1)}>Retry loading assessments</button></div>}{notice&&<p role="status" className="success-message">{notice}</p>}
   {changed&&<p className="alert warning">Client or source data changed after this draft began. Review your notes, then update its source context before saving.<button className="text-button" disabled={busy||disabled} onClick={()=>{setContext({revision:source.revision,profile:client.updatedAt});setDraft(d=>({...d,acknowledge:false}));}}>Use current source context</button></p>}
   <form noValidate onSubmit={e=>{e.preventDefault();void save();}}><fieldset disabled={busy||disabled}>
    <div className="form-grid"><label htmlFor="record-from">Recorded assessment from month<input id="record-from" placeholder="YYYY-MM" value={draft.fromMonth} onChange={e=>setDraft(d=>({...d,fromMonth:e.target.value}))}/></label><label htmlFor="record-through">Recorded assessment through month<input id="record-through" placeholder="YYYY-MM" value={draft.throughMonth} onChange={e=>setDraft(d=>({...d,throughMonth:e.target.value}))}/></label></div>
    <label htmlFor="record-title">Assessment record title<input id="record-title" maxLength={120} value={draft.title} onChange={e=>setDraft(d=>({...d,title:e.target.value}))}/></label>
    {([['findings','Counselor findings'],['recommendations','Counselor recommendations'],['followUp','Follow-up arrangements']] as const).map(([key,label])=><label key={key} htmlFor={`record-${key}`}>{label}<textarea id={`record-${key}`} rows={3} maxLength={4000} value={draft[key]} onChange={e=>setDraft(d=>({...d,[key]:e.target.value}))}/></label>)}
    <label className="check-label"><input id="record-ack" type="checkbox" checked={draft.acknowledge} onChange={e=>setDraft(d=>({...d,acknowledge:e.target.checked}))}/>I reviewed this client, period, approved source decisions, estimates, exclusions and missing information. This is an incomplete internal assessment, not a final client plan.</label>
    <div className="budget-add"><button className="button primary" type="submit" disabled={changed}>Save assessment record</button><button className="button secondary" type="button" onClick={()=>{setDraft(blank());setContext({revision:source.revision,profile:client.updatedAt});}}>Discard assessment draft</button></div>
   </fieldset></form>
   {directory&&<label htmlFor="saved-assessment">Open saved assessment<select id="saved-assessment" disabled={busy||disabled} value={selectedId.current??''} onChange={e=>void select(e.target.value)}><option value="">Choose a saved record ({directory.records.length})</option>{directory.records.map(r=><option value={r.id} key={r.id}>{r.title} — {r.periodStart} to {r.periodEnd}{r.needsReview?' — Needs review':''}</option>)}</select></label>}
   {p&&selected&&<article className="saved-assessment-view" aria-label="Saved counselor assessment">
    <p className="eyebrow">IMMUTABLE INTERNAL ASSESSMENT · INCOMPLETE INFORMATION</p><h3>{p.title}</h3><p className="saved-assessment-identity">{p.workspace.organizationName} · {p.client.firstName} {p.client.lastName} · Counselor {p.workspace.counselorName}</p>
    <p className="field-help">Recorded {date(p.createdAt)} (America/Chicago) · Calculation {p.calculator} · App {p.appVersion}</p>
    <p className="saved-assessment-status">{needsReview?'Needs review — source or client context changed':'Recorded context matches the current source and client profile'}. The saved figures remain historical and are never overwritten.</p>{selected.reviewReasons.map(r=><p className="alert warning" key={r}>{r}</p>)}
    <p className="field-help">Household: {p.client.householdSize}. Recorded goals: {p.client.goals||'None recorded'}. No financial scenario is selected or included. Recommendations and follow-up are counselor-authored notes, not approved transaction amounts.</p>
    <fieldset className="assessment-export" disabled={busy||disabled||dirty}>
     <p className="field-help">Export this saved record as a printable internal PDF. It contains readable client information outside workspace encryption. Choose a private location and a new filename. Missing information and historical review status are included.</p>
     <label htmlFor="assessment-pdf-path">PDF destination<input id="assessment-pdf-path" value={exportPath} placeholder="Full local path ending in .pdf" onChange={e=>setExportPath(e.target.value)}/></label>
     <div className="budget-add"><button className="button secondary" type="button" onClick={()=>void exportPdf(true)}>Choose PDF destination</button><button className="button primary" type="button" disabled={!exportPath.trim()} onClick={()=>void exportPdf(false)}>Export saved assessment PDF</button></div>
     {dirty&&<p className="field-help">Save or discard your assessment draft before exporting a saved record.</p>}
    </fieldset>
    <FinancialAssessmentView value={p.assessment}/>
    {([['findings','Counselor findings'],['recommendations','Counselor recommendations'],['followUp','Follow-up arrangements']] as const).map(([key,label])=><div key={key}><h4>{label}</h4><p className="assessment-note">{p[key]||'None recorded.'}</p></div>)}
    <details><summary>Captured sources ({p.basis.documents.length})</summary>{p.basis.documents.map(d=><div className="budget-line" key={d.id}><strong>{d.fileName} — {p.basis.accounts.find(a=>a.id===d.accountId)?.label}</strong><p className="field-help">Source ID {d.id} · SHA-256 {d.sha256} · entered period {d.input.periodStart??'Unknown'} to {d.input.periodEnd??'Unknown'}. Original file {d.sourceRetained?'retained in encrypted workspace':'not retained'}. Raw mapped evidence remains captured.</p><p className="field-help">Whole-source difference: {d.sourceDifference===null?'Unknown':usd(d.sourceDifference)}. Approved closing difference: {d.approvedDifference===null?'Unknown':usd(d.approvedDifference)}.</p>{d.warnings.map((w,i)=><p className="field-help" key={i}>{w}</p>)}{d.balanceReconciliation&&<p className="field-help">Running-balance source differences: {d.balanceReconciliation.sourceDifferences}. Approved differences: {d.balanceReconciliation.approvedDifferences}. These checks do not verify assets.</p>}</div>)}</details>
    <details><summary>Captured transaction evidence ({p.basis.records.length})</summary><p className="field-help">All captured imported dates are retained for provenance; amounts in the assessment use only Approved rows within its period. Pending, clarification, rejected and out-of-period rows do not enter those amounts.</p><div className="budget-table-wrap"><table aria-label="Captured assessment transactions"><thead><tr><th>Date / transaction</th><th>Status / signed amount</th></tr></thead><tbody>{p.basis.records.slice(page*50,(page+1)*50).map(r=><tr key={r.id}><td>{r.fields.date??'Unknown'} · {r.fields.description}<br/>{r.fields.category}<details><summary>Original source and decision</summary><p>{r.raw.reportedDate} · {r.raw.reportedDescription} · reported amount {r.raw.reportedAmount??'Unknown'} · {r.raw.page?`page ${r.raw.page}`:`record ${r.raw.recordNumber}`} line {r.raw.line}</p><p className="field-help">Record {r.id} · Source {r.documentId} · Latest captured decision {p.basis.history.slice().reverse().find(h=>h.recordId===r.id)?.id??'No decision'}</p></details></td><td>{r.status}<br/>{r.fields.amount===null?'Unknown':usd(r.fields.amount)}</td></tr>)}</tbody></table></div><button className="text-button" disabled={page===0} onClick={()=>setPage(v=>v-1)}>Previous evidence page</button><button className="text-button" disabled={(page+1)*50>=p.basis.records.length} onClick={()=>setPage(v=>v+1)}>Next evidence page</button></details>
    <details><summary>Captured balance evidence and integrity</summary><p className="field-help">Record ID {p.id} · SHA-256 {selected.digest}. This checksum checks stored bytes; it is not a digital signature or professional certification.</p>{p.basis.financialFacts?.data?.balances.map(b=><p key={b.label} className="field-help">{b.label} · {b.kind} · {b.amount===null?'Unknown':usd(b.amount)} · {b.evidence} · source {b.documentId??'Other reviewed evidence'}</p>)}</details>
   </article>}
  </>}
 </details>;
}
