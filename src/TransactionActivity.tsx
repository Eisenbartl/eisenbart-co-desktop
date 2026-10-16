import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import type {ActivityEvent,ActivityPage,ActivityQuery,ActivitySource,ReviewStatus} from './types';
const empty=():ActivityQuery=>({text:'',from:null,through:null,sourceKind:null,sourceId:null,accountId:null,status:null,offset:0,snapshot:null});
const unknown=(s:string|null|undefined)=>s||'Unknown';
export default function TransactionActivity({clientId}:{clientId:string}) {
 const [draft,setDraft]=useState(empty),[page,setPage]=useState<ActivityPage|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const [choices,setChoices]=useState<{clientId:string;sources:ActivitySource[]}|null>(null);
 const generation=useRef(0);
 async function load(input:ActivityQuery) {
  const ticket=++generation.current;setBusy(true);setError('');setPage(null);
  try {const result=await api.queryTransactionActivity(clientId,input);if(ticket!==generation.current)return;if(result.clientId!==clientId)throw new Error('Activity response did not match the selected client.');setPage(result);setChoices({clientId,sources:result.sources});}
  catch(e){if(ticket===generation.current)setError(String(e));}
  finally{if(ticket===generation.current)setBusy(false);}
 }
 useEffect(()=>{setDraft(empty());setPage(null);setChoices(null);setError('');void load(empty());return()=>{generation.current++;};},[clientId]);
 const sources=choices?.clientId===clientId?choices.sources:[];
 function search(input:ActivityQuery){setDraft({...input,offset:0,snapshot:null});void load({...input,offset:0,snapshot:null});}
 function follow(sourceId:string,text:string){search({...empty(),sourceId,text});}
 return <section className="panel transaction-activity" aria-label="Transaction activity history">
  <h2>Transaction activity history</h2>
  <p>Imported bank and card rows and every recorded review decision, newest activity first. This read-only history does not change approvals or financial figures.</p>
  <p className="field-help">Dates filter the transaction date recorded at each event. Status filters the decision at that time. Unknown dates are excluded when date filters are used. Search covers descriptions, merchants, categories, types, reasons, filenames, account names and record/decision IDs, including original descriptions.</p>
  <form onSubmit={e=>{e.preventDefault();search(draft);}}><fieldset disabled={busy}>
   <div className="form-grid">
    <label htmlFor="activity-text">Search activity<input id="activity-text" maxLength={200} value={draft.text} onChange={e=>setDraft(d=>({...d,text:e.target.value}))}/></label>
    <label htmlFor="activity-kind">Activity source type<select id="activity-kind" value={draft.sourceKind??''} onChange={e=>setDraft(d=>({...d,sourceKind:(e.target.value||null) as ActivityQuery['sourceKind'],sourceId:null}))}><option value="">Bank and card</option><option value="bank">Bank</option><option value="card">Card</option></select></label>
    <label htmlFor="activity-source">Activity source document<select id="activity-source" value={draft.sourceId??''} onChange={e=>setDraft(d=>({...d,sourceId:e.target.value||null}))}><option value="">All source documents</option>{sources.filter(s=>!draft.sourceKind||s.kind===draft.sourceKind).map(s=><option key={s.id} value={s.id}>{s.fileName} · {s.currentAccount} · {s.kind}</option>)}</select></label>
    <label htmlFor="activity-account">Activity account<select id="activity-account" value={draft.accountId??''} onChange={e=>setDraft(d=>({...d,accountId:e.target.value||null}))}><option value="">All accounts</option>{sources.filter((s,i,a)=>s.accountId&&a.findIndex(v=>v.accountId===s.accountId)===i).map(s=><option key={s.accountId} value={s.accountId!}>{s.currentAccount}{s.archived?' (archived)':''}</option>)}</select></label>
    <label htmlFor="activity-status">Historical review status<select id="activity-status" value={draft.status??''} onChange={e=>setDraft(d=>({...d,status:(e.target.value||null) as ReviewStatus|null}))}><option value="">All review states</option>{['Pending','Needs Clarification','Approved','Rejected'].map(s=><option key={s}>{s}</option>)}</select></label>
    <label htmlFor="activity-from">Transaction date from<input id="activity-from" placeholder="YYYY-MM-DD" maxLength={10} value={draft.from??''} onChange={e=>setDraft(d=>({...d,from:e.target.value||null}))}/></label>
    <label htmlFor="activity-through">Transaction date through<input id="activity-through" placeholder="YYYY-MM-DD" maxLength={10} value={draft.through??''} onChange={e=>setDraft(d=>({...d,through:e.target.value||null}))}/></label>
   </div><div className="budget-add"><button className="button primary" type="submit">Search activity</button><button className="button secondary" type="button" onClick={()=>search(draft)}>Refresh activity</button><button className="button secondary" type="button" onClick={()=>search(empty())}>Clear activity filters</button></div>
  </fieldset></form>
  {busy&&<p role="status">Loading transaction activity…</p>}{error&&<p role="alert">{error}</p>}
  {page?.clientId===clientId&&<>
   <p className="activity-results" role="status">{page.total===0?'No activity matches these filters.':`Showing ${page.query.offset+1}–${Math.min(page.query.offset+page.pageSize,page.total)} of ${page.total} events.`}</p>
   <p className="field-help">Applied search: {page.query.text||'all text'} · {page.query.sourceKind??'bank and card'} · {page.query.status??'all review states'} · {page.query.from??'any start date'} through {page.query.through??'any end date'}. Filters apply when you search. A source change requires a fresh search before paging.</p>
   <div className="budget-add"><button type="button" className="button secondary" disabled={busy||page.query.offset===0} onClick={()=>void load({...page.query,offset:Math.max(0,page.query.offset-page.pageSize)})}>Previous activity page</button><button type="button" className="button secondary" disabled={busy||page.query.offset+page.pageSize>=page.total} onClick={()=>void load({...page.query,offset:page.query.offset+page.pageSize})}>Next activity page</button></div>
   <ol className="activity-events" start={page.query.offset+1}>{page.events.map(e=><li key={e.id}><ActivityEvidence event={e} source={page.sources.find(s=>s.id===e.sourceId)!} onFollow={follow}/></li>)}</ol>
   {!page.sources.length&&<p>No imported bank or card sources are recorded for this client. Import and review a statement in Documents to begin.</p>}
  </>}
 </section>;
}
function ActivityEvidence({event:e,source:s,onFollow}:{event:ActivityEvent;source:ActivitySource;onFollow:(source:string,text:string)=>void}) {
 return <details className="activity-event" data-event-id={e.id}><summary>{e.fields.date??'Unknown date'} · {e.fields.description||'No description'} · {e.status} · {e.decisionId?'Review decision':'Source import'} · {s.kind} · USD {unknown(e.fields.amount)}</summary>
  <p>{e.occurredAt} · {e.counselorName??'Source import'} · {e.fromStatus?`${e.fromStatus} → ${e.status}`:e.status} · {e.latest?'Latest recorded state':'Historical event'} · Current row status: {e.currentStatus}</p>
  <p>{e.reason}</p><p>Type {e.fields.kind} · Merchant {unknown(e.fields.merchant)} · Category {unknown(e.fields.category)}</p>
  <p className="field-help">Bank amounts describe cash movement. Positive card amounts increase liability; negative card amounts reduce liability. Events are historical evidence, not amounts to add together.</p>
  <dl className="activity-locators"><dt>Source document</dt><dd>{s.fileName} · {s.id}</dd><dt>SHA-256</dt><dd>{s.sha256}</dd><dt>Account</dt><dd>{s.currentAccount}{s.archived?' (archived)':''} · {s.accountId??'Unassigned card identity'} · Original source name: {s.originalAccount}</dd><dt>Statement period</dt><dd>{unknown(s.periodStart)} through {unknown(s.periodEnd)}</dd><dt>Original file</dt><dd>{s.retained?'Retained in encrypted storage':'Original bytes not retained; captured extraction evidence remains'}</dd><dt>Record locator</dt><dd>{e.recordId} · Record {e.raw.recordNumber} · Line {e.raw.line}{e.raw.page?` · Page ${e.raw.page}`:''} · {e.raw.extractionMethod}</dd><dt>Decision ID</dt><dd>{e.decisionId??'Source import; no review decision'}</dd></dl>
  <h4>Original reported values</h4><p>Date {unknown(e.raw.reportedDate)} · Amount {unknown(e.raw.reportedAmount)} · Debit {unknown(e.raw.reportedDebit)} · Credit {unknown(e.raw.reportedCredit)}</p><p>{e.raw.reportedDescription}</p><p>Merchant {unknown(e.raw.reportedMerchant)} · Category {unknown(e.raw.reportedCategory)} · Type {unknown(e.raw.reportedKind)} · Reference {unknown(e.raw.reportedReference)} · Balance {unknown(e.raw.reportedBalance)}</p>{e.raw.reportedLine&&<pre>{e.raw.reportedLine}</pre>}
  {e.raw.issues.length>0&&<ul>{e.raw.issues.map((v,i)=><li key={i}>{v}</li>)}</ul>}
  <button className="button secondary" type="button" onClick={()=>onFollow(s.id,e.recordId)}>Show all events for this record</button>
  {!!e.links.length&&<><h4>Current relationship context</h4><p className="field-help">These links describe the latest stored relationships, not relationships captured at this historical event. An obsolete repayment binding must be reviewed again in Financial Review → Bank & card activity.</p><ul>{e.links.map((l,i)=><li key={i}>{l.kind} · {l.current?'Current approval bindings':'Obsolete approval bindings'} · Source {l.sourceId} · Record {l.recordId} · Decision {l.decisionId??'Unknown'}<div className="budget-add"><button className="button secondary" type="button" onClick={()=>onFollow(l.sourceId,l.recordId)}>Open counterpart history</button>{l.decisionId&&<button className="button secondary" type="button" onClick={()=>onFollow(l.sourceId,l.decisionId!)}>Open linked decision</button>}</div></li>)}</ul></>}
 </details>;
}
