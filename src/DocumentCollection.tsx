import {useEffect,useRef,useState,type FormEvent} from 'react';
import {api} from './api';
import type {DocumentCollection as Collection,DocumentRequest} from './types';
const kinds=[['statement','Bank / savings statement'],['creditCard','Credit-card statement'],['loan','Loan statement'],['paystub','Pay stub'],['tax','Tax document'],['other','Other document']] as const;
const blank=():DocumentRequest=>({id:'',title:'',kind:'statement',status:'missing',accountId:null,sourceDocumentId:null,periodStart:null,periodEnd:null,dueDate:null,receivedDate:null,notes:'',archived:false});
const message=(e:unknown)=>e instanceof Error?e.message:String(e);
const when=(s:string)=>new Intl.DateTimeFormat('en-US',{dateStyle:'medium',timeStyle:'short',timeZone:'America/Chicago'}).format(new Date(s));
export default function DocumentCollection({clientId,onDirty,onBusy}:{clientId:string;onDirty:(v:boolean)=>void;onBusy:(v:boolean)=>void}) {
 const [saved,setSaved]=useState<Collection|null>(null),[items,setItems]=useState<DocumentRequest[]>([]),[selected,setSelected]=useState<number|null>(null);
 const [revision,setRevision]=useState<number|null>(null),[attempt,setAttempt]=useState(0),[loading,setLoading]=useState(true),[saving,setSaving]=useState(false),[error,setError]=useState(''),[notice,setNotice]=useState('');
 const [filter,setFilter]=useState('active');const generation=useRef(0);
 const historical=revision!==null;
 const dirty=Boolean(saved&&!historical&&JSON.stringify(items)!==JSON.stringify(saved.items));const busy=loading||saving;
 useEffect(()=>{onDirty(dirty);return()=>onDirty(false);},[dirty,onDirty]);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false);},[busy,onBusy]);
 useEffect(()=>{setRevision(null);setFilter('active');},[clientId]);
 useEffect(()=>{
  const g=++generation.current;setLoading(true);setSaved(null);setItems([]);setSelected(null);setSaving(false);setError('');setNotice('');
  api.getDocumentCollection(clientId,revision).then(r=>{if(g!==generation.current)return;if(r.clientId!==clientId||r.revision!==(revision??r.latestRevision))throw new Error('The checklist response did not match this client or version.');setSaved(r);setItems(structuredClone(r.items));}).catch(e=>{if(g===generation.current)setError(message(e));}).finally(()=>{if(g===generation.current)setLoading(false);});
  return()=>{generation.current++;};
 },[clientId,revision,attempt]);
 function edit<K extends keyof DocumentRequest>(key:K,value:DocumentRequest[K]){if(selected===null)return;setItems(a=>a.map((r,i)=>i===selected?{...r,[key]:value,...(key==='status'&&value==='missing'?{receivedDate:null,sourceDocumentId:null}:{})}:r));setNotice('');}
 async function save(e:FormEvent){
  e.preventDefault();if(!saved||!dirty||busy||historical)return;const g=generation.current;setSaving(true);setError('');setNotice('');
  try{const r=await api.saveDocumentCollection(clientId,{expectedRevision:saved.revision,items});if(g!==generation.current)return;if(r.clientId!==clientId||r.revision!==saved.revision+1||r.latestRevision!==r.revision)throw new Error('The saved checklist did not match this client or expected version.');setSaved(r);setItems(structuredClone(r.items));setSelected(null);setNotice('Document checklist saved locally.');}
  catch(e){if(g===generation.current)setError(message(e));}finally{if(g===generation.current)setSaving(false);}
 }
 const current=selected===null?null:items[selected];
 const visible=items.map((item,index)=>({item,index})).filter(({item})=>filter==='all'||(filter==='archived'?item.archived:!item.archived&&(filter==='active'||item.status===filter)));
 return <section className="panel document-collection" aria-labelledby="collection-title">
  <p className="eyebrow">DOCUMENT COLLECTION</p><h2 id="collection-title">Collection checklist</h2>
  <p className="muted small">Track information requested from this client. Marking a document received records receipt only; its financial contents still need separate import, review and approval.</p>
  {loading&&<p role="status">Loading document checklist…</p>}{error&&<p role="alert" className="alert error">{error}</p>}{notice&&<p role="status" className="success-message">{notice}</p>}
  {!loading&&!saved&&<button className="button secondary" onClick={()=>setAttempt(v=>v+1)}>Retry checklist loading</button>}
  {saved&&<>
   <p className="collection-counts">Saved version {saved.revision} · {saved.missing} missing · {saved.received} received · {saved.archived} archived{saved.updatedAt?` · ${when(saved.updatedAt)} (America/Chicago)`:''}</p>
   <div className="form-grid collection-controls"><label htmlFor="collection-filter">Show requests<select id="collection-filter" value={filter} onChange={e=>setFilter(e.target.value)}><option value="active">Active requests</option><option value="missing">Missing</option><option value="received">Received</option><option value="archived">Archived</option><option value="all">All requests</option></select></label><label htmlFor="collection-history">Checklist version<select id="collection-history" disabled={busy||dirty} value={revision??''} onChange={e=>setRevision(e.target.value?Number(e.target.value):null)}><option value="">Current checklist</option>{saved.history.slice().reverse().map(v=><option key={v.revision} value={v.revision}>Version {v.revision} · {when(v.updatedAt)} (America/Chicago)</option>)}</select></label></div>
   {historical&&<p className="alert warning">Historical checklist version {saved.revision} of {saved.latestRevision}. This view is read-only. Choose Current checklist to make changes.</p>}
   {visible.length?<div className="table-scroll"><table className="collection-table"><caption className="sr-only">Document collection requests</caption><thead><tr><th>Request</th><th>Status</th><th>Due / received</th><th>Open</th></tr></thead><tbody>{visible.map(({item,index})=><tr key={item.id||`draft-${index}`}><td><strong>{item.title||'New unsaved request'}</strong><span>{kinds.find(([key])=>key===item.kind)?.[1]}{item.accountId?` · ${saved.accounts.find(a=>a.id===item.accountId)?.label??'Unavailable account'}`:''}</span></td><td>{item.archived?'Archived':item.status==='received'?'Received':'Missing'}</td><td>{item.status==='received'?`Received ${item.receivedDate??'Date needed'}`:item.dueDate??'No due date'}</td><td><button type="button" className="text-button" disabled={busy} aria-label={`Open request ${item.title||index+1}`} onClick={()=>setSelected(index)}>{historical?'View':'Edit'}</button></td></tr>)}</tbody></table></div>:<p className="empty-note">{items.length?'No requests match this view.':'No document requests yet. Add a request to begin.'}</p>}
   <form onSubmit={e=>void save(e)} noValidate><fieldset disabled={busy||historical}>
    <button type="button" className="button secondary" disabled={items.length>=100} onClick={()=>{setSelected(items.length);setItems(a=>[...a,blank()]);setNotice('');}}>Add document request</button>
    {current&&<div className="collection-editor"><h3>{historical?'Recorded request':current.id?'Edit document request':'New document request'}</h3><div className="form-grid">
     <label htmlFor="collection-request-title">Request title<input id="collection-request-title" value={current.title} maxLength={200} onChange={e=>edit('title',e.target.value)}/></label>
     <label htmlFor="collection-kind">Document type<select id="collection-kind" value={current.kind} onChange={e=>edit('kind',e.target.value as DocumentRequest['kind'])}>{kinds.map(([v,l])=><option key={v} value={v}>{l}</option>)}</select></label>
     <label htmlFor="collection-status">Collection status<select id="collection-status" value={current.status} onChange={e=>edit('status',e.target.value as DocumentRequest['status'])}><option value="missing">Missing / requested</option><option value="received">Received (not financially approved)</option></select></label>
     <label htmlFor="collection-account">Statement account (optional)<select id="collection-account" value={current.accountId??''} onChange={e=>edit('accountId',e.target.value||null)}><option value="">No account selected</option>{saved.accounts.map(a=><option key={a.id} value={a.id}>{a.label}</option>)}</select></label>
     {(['periodStart','periodEnd','dueDate','receivedDate'] as const).map((key,i)=><label key={key} htmlFor={`collection-${key}`}>{['Requested period starts (optional)','Requested period ends (optional)','Due date (optional)','Received date (required when received)'][i]}<input type="date" id={`collection-${key}`} value={current[key]??''} disabled={key==='receivedDate'&&current.status!=='received'} onChange={e=>edit(key,e.target.value||null)}/></label>)}
     <label htmlFor="collection-source">Linked imported source (optional)<select id="collection-source" value={current.sourceDocumentId??''} disabled={current.status!=='received'} onChange={e=>edit('sourceDocumentId',e.target.value||null)}><option value="">No source linked / received outside the app</option>{saved.sources.map(s=><option key={s.id} value={s.id}>{s.fileName} — {saved.accounts.find(a=>a.id===s.accountId)?.label??'Account unavailable'}</option>)}</select></label>
    </div><label htmlFor="collection-notes">Request / receipt notes<textarea id="collection-notes" rows={3} maxLength={2000} value={current.notes} onChange={e=>edit('notes',e.target.value)}/></label><label className="check-label"><input id="collection-archived" type="checkbox" checked={current.archived} onChange={e=>edit('archived',e.target.checked)}/>Archive this request (keep its history)</label>{!current.id&&<button type="button" className="text-button" onClick={()=>{setItems(a=>a.filter((_,i)=>i!==selected));setSelected(null);}}>Remove unsaved request</button>}</div>}
    {!historical&&<div className="form-actions"><span className="save-status">{dirty?'Unsaved checklist changes':'Saved checklist'}</span><button className="button primary" type="submit" disabled={!dirty}>Save document checklist</button></div>}
   </fieldset></form>
   <button className="text-button" type="button" disabled={busy} onClick={()=>{setRevision(null);setAttempt(v=>v+1);}}>{dirty?'Discard changes and reload checklist':'Refresh current checklist'}</button>
   <p className="field-help">Blank dates and accounts remain unknown. Received documents can be linked to an existing imported source; other document formats can be tracked as received without importing them. Saved requests remain in version history. Use Archive instead of deleting saved records.</p>
  </>}
 </section>;
}
